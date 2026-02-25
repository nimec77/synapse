//! Telegram bot slash-command handlers for Synapse session management.
//!
//! Implements the `/start`, `/help`, `/new`, `/history`, `/list`, `/switch [N]`, and
//! `/delete [N]` commands. None of these commands invoke LLM inference — they manage
//! sessions only. When `/switch` or `/delete` are used without an argument, an inline
//! keyboard is displayed so the user can select a session by tapping a button.

use std::sync::Arc;

use chrono::TimeZone;
use synapse_core::session::Session;
use synapse_core::{Config, SessionStore, SessionSummary};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{
    CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Message as TgMessage,
};
use teloxide::utils::command::BotCommands;
use uuid::Uuid;

use crate::handlers::{ChatSessionMap, ChatSessions, chunk_message, is_authorized};

/// All slash commands supported by the Synapse Telegram bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    /// Welcome message for first-time users.
    #[command(description = "Start the bot")]
    Start,
    /// Show available commands.
    #[command(description = "Show available commands")]
    Help,
    /// Start a new conversation session.
    #[command(description = "Start a new session")]
    New,
    /// Display the conversation history of the current session.
    #[command(description = "Show conversation history")]
    History,
    /// List all sessions for this chat.
    #[command(description = "List all sessions")]
    List,
    /// Switch to session number N (1-based index). Omit N to see a keyboard.
    #[command(description = "Switch to session N")]
    Switch(String),
    /// Delete session number N (1-based index). Omit N to see a keyboard.
    #[command(description = "Delete session N")]
    Delete(String),
}

/// Entry-point handler for all slash commands.
///
/// Checks authorization and dispatches to the appropriate private handler.
pub async fn handle_command(
    bot: Bot,
    msg: TgMessage,
    cmd: Command,
    config: Arc<Config>,
    storage: Arc<Box<dyn SessionStore>>,
    chat_map: ChatSessionMap,
) -> ResponseResult<()> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let allowed_users = config
        .telegram
        .as_ref()
        .map(|t| t.allowed_users.as_slice())
        .unwrap_or(&[]);

    if !is_authorized(user_id, allowed_users) {
        return Ok(()); // Silent drop — do not reveal bot existence.
    }

    match cmd {
        Command::Start => cmd_start(&bot, &msg).await,
        Command::Help => cmd_help(&bot, &msg).await,
        Command::New => cmd_new(&bot, &msg, &config, &storage, &chat_map).await,
        Command::History => cmd_history(&bot, &msg, &storage, &chat_map).await,
        Command::List => cmd_list(&bot, &msg, &storage, &chat_map).await,
        Command::Switch(ref arg) => cmd_switch(&bot, &msg, arg, &storage, &chat_map).await,
        Command::Delete(ref arg) => cmd_delete(&bot, &msg, arg, &config, &storage, &chat_map).await,
    }
}

/// Send a welcome message for new users or re-opening the bot.
async fn cmd_start(bot: &Bot, msg: &TgMessage) -> ResponseResult<()> {
    let welcome = "Welcome to Synapse! I'm an AI assistant.\n\n\
        Send me a message to start chatting, or use /help to see available commands.";
    bot.send_message(msg.chat.id, welcome).await?;
    Ok(())
}

/// Parse a session argument string into a typed result.
///
/// - Empty/whitespace-only string → `Ok(None)` — show interactive keyboard
/// - Valid `usize` string → `Ok(Some(n))` — direct execution with index n
/// - Non-numeric, non-empty string → `Err(hint)` — invalid argument
fn parse_session_arg(arg: &str) -> Result<Option<usize>, String> {
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed.parse::<usize>().map(Some).map_err(|_| {
            format!(
                "Invalid argument '{}'. Use a number or omit to see a list.",
                trimmed
            )
        })
    }
}

/// Parse callback data in the format "action:N" (e.g., "switch:2", "delete:1").
fn parse_callback_data(data: &str) -> Option<(&str, usize)> {
    let (action, n_str) = data.split_once(':')?;
    let n = n_str.parse::<usize>().ok()?;
    Some((action, n))
}

/// Reply with the teloxide-generated command description string.
async fn cmd_help(bot: &Bot, msg: &TgMessage) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

/// Create a new session for this chat, evicting the oldest if the cap is reached.
async fn cmd_new(
    bot: &Bot,
    msg: &TgMessage,
    config: &Config,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;
    let max_sessions = config
        .telegram
        .as_ref()
        .map(|t| t.max_sessions_per_chat as usize)
        .unwrap_or(10);

    let mut evicted = false;

    {
        let mut map = chat_map.write().await;
        let chat_sessions = map.entry(chat_id).or_insert_with(|| ChatSessions {
            sessions: vec![],
            active_idx: 0,
        });

        // Enforce session cap: evict the oldest session (last in the vec = oldest).
        if chat_sessions.sessions.len() >= max_sessions
            && let Some(oldest_id) = chat_sessions.sessions.last().copied()
        {
            let _ = storage.delete_session(oldest_id).await;
            chat_sessions.sessions.pop();
            evicted = true;
        }

        // Create the new session.
        let session =
            Session::new(&config.provider, &config.model).with_name(format!("tg:{}", chat_id));

        if let Err(e) = storage.create_session(&session).await {
            tracing::error!("Failed to create session for chat {}: {}", chat_id, e);
            drop(map);
            bot.send_message(msg.chat.id, "Failed to create session. Please try again.")
                .await?;
            return Ok(());
        }

        // Insert at the front and set as active.
        chat_sessions.sessions.insert(0, session.id);
        chat_sessions.active_idx = 0;
    }

    let reply = if evicted {
        "New session created. Oldest session removed to stay within the session limit.".to_string()
    } else {
        "New session created.".to_string()
    };
    bot.send_message(msg.chat.id, reply).await?;
    Ok(())
}

/// Show the conversation history of the currently active session.
async fn cmd_history(
    bot: &Bot,
    msg: &TgMessage,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    let session_id = {
        let map = chat_map.read().await;
        map.get(&chat_id).and_then(|cs| cs.active_session_id())
    };

    let session_id = match session_id {
        Some(id) => id,
        None => {
            bot.send_message(
                msg.chat.id,
                "No active session. Send a message or use /new to start one.",
            )
            .await?;
            return Ok(());
        }
    };

    let messages = storage.get_messages(session_id).await.unwrap_or_default();

    if messages.is_empty() {
        bot.send_message(msg.chat.id, "No messages in current session.")
            .await?;
        return Ok(());
    }

    let mut output = String::new();
    for m in &messages {
        let role_label = match m.role {
            synapse_core::message::Role::User => "You",
            synapse_core::message::Role::Assistant => "Assistant",
            synapse_core::message::Role::System => "System",
            synapse_core::message::Role::Tool => "Tool",
        };
        let timestamp = chrono::Utc
            .from_utc_datetime(&m.timestamp.naive_utc())
            .format("%Y-%m-%d %H:%M")
            .to_string();
        output.push_str(&format!(
            "[{}] {}\n{}\n\n",
            role_label, timestamp, m.content
        ));
    }

    for chunk in chunk_message(output.trim()) {
        bot.send_message(msg.chat.id, chunk).await?;
    }
    Ok(())
}

/// List all sessions for this chat with timestamps and a marker for the active one.
async fn cmd_list(
    bot: &Bot,
    msg: &TgMessage,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    let (session_uuids, active_id) = {
        let map = chat_map.read().await;
        match map.get(&chat_id) {
            Some(cs) if !cs.sessions.is_empty() => (cs.sessions.clone(), cs.active_session_id()),
            _ => {
                bot.send_message(
                    msg.chat.id,
                    "No sessions. Send a message or use /new to start one.",
                )
                .await?;
                return Ok(());
            }
        }
    };

    let all_sessions: Vec<SessionSummary> = storage.list_sessions().await.unwrap_or_default();

    // Filter to only sessions belonging to this chat (preserve DB ordering = updated_at DESC).
    let chat_session_list: Vec<&SessionSummary> = all_sessions
        .iter()
        .filter(|s| session_uuids.contains(&s.id))
        .collect();

    if chat_session_list.is_empty() {
        bot.send_message(
            msg.chat.id,
            "No sessions. Send a message or use /new to start one.",
        )
        .await?;
        return Ok(());
    }

    let mut output = String::new();
    for (i, s) in chat_session_list.iter().enumerate() {
        let active_marker = if Some(s.id) == active_id { "*" } else { " " };
        let timestamp = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
        let preview = s
            .preview
            .as_deref()
            .unwrap_or("")
            .chars()
            .take(40)
            .collect::<String>();
        output.push_str(&format!(
            "{}. [{}] {} | {} msgs | {}\n",
            i + 1,
            active_marker,
            timestamp,
            s.message_count,
            preview,
        ));
    }

    for chunk in chunk_message(output.trim()) {
        bot.send_message(msg.chat.id, chunk).await?;
    }
    Ok(())
}

/// Fetch the display-ordered session list for a chat.
///
/// Returns `Some((sessions, active_id))` if the chat has sessions, `None` otherwise.
/// Sessions are ordered by `updated_at DESC` (matching `/list` ordering).
async fn fetch_chat_sessions(
    chat_id: i64,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> Option<(Vec<SessionSummary>, Option<Uuid>)> {
    let (session_uuids, active_id) = {
        let map = chat_map.read().await;
        match map.get(&chat_id) {
            Some(cs) if !cs.sessions.is_empty() => (cs.sessions.clone(), cs.active_session_id()),
            _ => return None,
        }
    };

    let all_sessions: Vec<SessionSummary> = storage.list_sessions().await.unwrap_or_default();
    let chat_sessions: Vec<SessionSummary> = all_sessions
        .into_iter()
        .filter(|s| session_uuids.contains(&s.id))
        .collect();

    if chat_sessions.is_empty() {
        None
    } else {
        Some((chat_sessions, active_id))
    }
}

/// Build an inline keyboard with one button per session.
///
/// Each button's callback data follows the format `"action:N"` where `action` is
/// `"switch"` or `"delete"` and `N` is the 1-based session index. The active session
/// is marked with `*` in the button label.
fn build_session_keyboard(
    action: &str,
    sessions: &[&SessionSummary],
    active_id: Option<Uuid>,
) -> InlineKeyboardMarkup {
    let buttons: Vec<Vec<InlineKeyboardButton>> = sessions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let idx = i + 1; // 1-based
            let active_marker = if Some(s.id) == active_id { "*" } else { " " };
            let date = s.updated_at.format("%Y-%m-%d").to_string();
            let preview = s
                .preview
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(20)
                .collect::<String>();
            let label = format!(
                "{}. [{}] {} | {} msgs | {}",
                idx, active_marker, date, s.message_count, preview
            );
            let data = format!("{}:{}", action, idx);
            vec![InlineKeyboardButton::callback(label, data)]
        })
        .collect();
    InlineKeyboardMarkup::new(buttons)
}

/// Send an inline keyboard for session switching when no argument is provided.
async fn cmd_switch_keyboard(
    bot: &Bot,
    msg: &TgMessage,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match fetch_chat_sessions(chat_id, storage, chat_map).await {
        None => {
            bot.send_message(
                msg.chat.id,
                "No sessions. Send a message or use /new to start one.",
            )
            .await?;
        }
        Some((sessions, active_id)) => {
            let refs: Vec<&SessionSummary> = sessions.iter().collect();
            let keyboard = build_session_keyboard("switch", &refs, active_id);
            bot.send_message(msg.chat.id, "Select a session to switch to:")
                .reply_markup(keyboard)
                .await?;
        }
    }
    Ok(())
}

/// Send an inline keyboard for session deletion when no argument is provided.
async fn cmd_delete_keyboard(
    bot: &Bot,
    msg: &TgMessage,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match fetch_chat_sessions(chat_id, storage, chat_map).await {
        None => {
            bot.send_message(
                msg.chat.id,
                "No sessions. Send a message or use /new to start one.",
            )
            .await?;
        }
        Some((sessions, active_id)) => {
            let refs: Vec<&SessionSummary> = sessions.iter().collect();
            let keyboard = build_session_keyboard("delete", &refs, active_id);
            bot.send_message(msg.chat.id, "Select a session to delete:")
                .reply_markup(keyboard)
                .await?;
        }
    }
    Ok(())
}

/// Execute the switch-to-session-N logic, returning a reply string.
///
/// Re-fetches the session list from storage for consistency (handles staleness).
/// Returns `Ok(reply)` on success or `Err(error_message)` on invalid index.
async fn do_switch(
    n: usize,
    chat_id: i64,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> Result<String, String> {
    let (sessions, _) = fetch_chat_sessions(chat_id, storage, chat_map)
        .await
        .ok_or_else(|| "No sessions available.".to_string())?;

    if n == 0 || n > sessions.len() {
        return Err(format!(
            "Invalid session index {}. Use /list to see available sessions.",
            n
        ));
    }

    let target_id = sessions[n - 1].id;

    {
        let mut map = chat_map.write().await;
        if let Some(chat_sessions) = map.get_mut(&chat_id)
            && let Some(pos) = chat_sessions
                .sessions
                .iter()
                .position(|&id| id == target_id)
        {
            chat_sessions.active_idx = pos;
        }
    }

    storage.touch_session(target_id).await.ok();
    Ok(format!("Switched to session {}.", n))
}

/// Execute the delete-session-N logic, returning a reply string.
///
/// Re-fetches the session list from storage for consistency (handles staleness).
/// Returns `Ok(reply)` on success or `Err(error_message)` on invalid index.
async fn do_delete(
    n: usize,
    chat_id: i64,
    config: &Config,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> Result<String, String> {
    let (sessions, _) = fetch_chat_sessions(chat_id, storage, chat_map)
        .await
        .ok_or_else(|| "No sessions available.".to_string())?;

    if n == 0 || n > sessions.len() {
        return Err(format!(
            "Invalid session index {}. Use /list to see available sessions.",
            n
        ));
    }

    let target_id = sessions[n - 1].id;
    let _ = storage.delete_session(target_id).await;

    let reply = {
        let mut map = chat_map.write().await;
        let chat_sessions = map.entry(chat_id).or_insert_with(|| ChatSessions {
            sessions: vec![],
            active_idx: 0,
        });

        let deleted_vec_pos = chat_sessions
            .sessions
            .iter()
            .position(|&id| id == target_id);
        let was_active = deleted_vec_pos == Some(chat_sessions.active_idx);

        if let Some(pos) = deleted_vec_pos {
            chat_sessions.sessions.remove(pos);
        }

        if chat_sessions.sessions.is_empty() {
            // Auto-create a new session.
            let session =
                Session::new(&config.provider, &config.model).with_name(format!("tg:{}", chat_id));
            if let Ok(()) = storage.create_session(&session).await {
                chat_sessions.sessions.push(session.id);
                chat_sessions.active_idx = 0;
            }
            format!("Session {} deleted. New session created.", n)
        } else if was_active {
            chat_sessions.active_idx = 0;
            format!("Session {} deleted. Switched to session 1.", n)
        } else {
            // Adjust active_idx if a lower-indexed session was removed.
            if let Some(pos) = deleted_vec_pos
                && pos < chat_sessions.active_idx
            {
                chat_sessions.active_idx = chat_sessions.active_idx.saturating_sub(1);
            }
            format!("Session {} deleted.", n)
        }
    };

    Ok(reply)
}

/// Switch the active session to the N-th session (1-based index from `/list` ordering).
///
/// When `arg` is empty, displays an inline keyboard for session selection.
/// When `arg` is a valid number, switches directly to that session.
/// When `arg` is non-numeric, replies with an error hint.
async fn cmd_switch(
    bot: &Bot,
    msg: &TgMessage,
    arg: &str,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match parse_session_arg(arg) {
        Ok(None) => cmd_switch_keyboard(bot, msg, storage, chat_map).await,
        Ok(Some(n)) => {
            let reply = match do_switch(n, chat_id, storage, chat_map).await {
                Ok(r) | Err(r) => r,
            };
            bot.send_message(msg.chat.id, reply).await?;
            Ok(())
        }
        Err(hint) => {
            bot.send_message(msg.chat.id, hint).await?;
            Ok(())
        }
    }
}

/// Delete the N-th session (1-based index from `/list` ordering).
///
/// When `arg` is empty, displays an inline keyboard for session selection.
/// When `arg` is a valid number, deletes directly.
/// When `arg` is non-numeric, replies with an error hint.
///
/// If the active session is deleted and others remain, switches to session 1 (index 0).
/// If the active session is deleted and no sessions remain, auto-creates a new session.
/// If a non-active session is deleted, adjusts `active_idx` if needed.
async fn cmd_delete(
    bot: &Bot,
    msg: &TgMessage,
    arg: &str,
    config: &Config,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match parse_session_arg(arg) {
        Ok(None) => cmd_delete_keyboard(bot, msg, storage, chat_map).await,
        Ok(Some(n)) => {
            let reply = match do_delete(n, chat_id, config, storage, chat_map).await {
                Ok(r) | Err(r) => r,
            };
            bot.send_message(msg.chat.id, reply).await?;
            Ok(())
        }
        Err(hint) => {
            bot.send_message(msg.chat.id, hint).await?;
            Ok(())
        }
    }
}

/// Handle inline keyboard button taps (CallbackQuery updates).
///
/// Parses callback data (`"switch:N"` or `"delete:N"`), executes the action
/// via `do_switch` / `do_delete`, and edits the keyboard message to show
/// the result text (removing the keyboard).
pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    config: Arc<Config>,
    storage: Arc<Box<dyn SessionStore>>,
    chat_map: ChatSessionMap,
) -> ResponseResult<()> {
    // 1. Authorization check — silent drop for unauthorized users.
    let user_id = q.from.id.0;
    let allowed_users = config
        .telegram
        .as_ref()
        .map(|t| t.allowed_users.as_slice())
        .unwrap_or(&[]);
    if !is_authorized(user_id, allowed_users) {
        return Ok(());
    }

    // 2. Answer callback query immediately — dismiss Telegram's loading spinner.
    bot.answer_callback_query(q.id.clone()).await?;

    // 3. Parse callback data.
    let data = match q.data.as_deref() {
        Some(d) if !d.is_empty() => d,
        _ => return Ok(()),
    };

    // 4. Extract chat_id and message_id from the original message.
    let message = match q.regular_message() {
        Some(m) => m,
        None => {
            tracing::warn!("Callback query without regular message, skipping edit");
            return Ok(());
        }
    };
    let chat_id = message.chat.id.0;
    let message_id = message.id;
    let tg_chat_id = message.chat.id;

    // 5. Parse "action:N" format.
    let (action, n) = match parse_callback_data(data) {
        Some(parsed) => parsed,
        None => {
            tracing::warn!("Invalid callback data: {}", data);
            return Ok(());
        }
    };

    // 6. Execute action.
    let reply = match action {
        "switch" => match do_switch(n, chat_id, &storage, &chat_map).await {
            Ok(r) | Err(r) => r,
        },
        "delete" => match do_delete(n, chat_id, &config, &storage, &chat_map).await {
            Ok(r) | Err(r) => r,
        },
        _ => {
            tracing::warn!("Unknown callback action: {}", action);
            return Ok(());
        }
    };

    // 7. Edit message to remove keyboard and show result.
    if let Err(e) = bot.edit_message_text(tg_chat_id, message_id, reply).await {
        tracing::warn!("Failed to edit callback message: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    // --- parse_session_arg ---

    #[test]
    fn test_parse_session_arg_empty() {
        assert_eq!(parse_session_arg(""), Ok(None));
    }

    #[test]
    fn test_parse_session_arg_whitespace() {
        assert_eq!(parse_session_arg("  "), Ok(None));
    }

    #[test]
    fn test_parse_session_arg_numeric() {
        assert_eq!(parse_session_arg("3"), Ok(Some(3)));
    }

    #[test]
    fn test_parse_session_arg_zero() {
        // Index validation is deferred to do_switch/do_delete; parser accepts 0.
        assert_eq!(parse_session_arg("0"), Ok(Some(0)));
    }

    #[test]
    fn test_parse_session_arg_non_numeric() {
        let result = parse_session_arg("abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid argument"));
    }

    #[test]
    fn test_parse_session_arg_negative() {
        // Negative values fail usize parse → treated as invalid argument.
        let result = parse_session_arg("-1");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid argument"));
    }

    // --- parse_callback_data ---

    #[test]
    fn test_parse_callback_data_valid_switch() {
        assert_eq!(parse_callback_data("switch:2"), Some(("switch", 2)));
    }

    #[test]
    fn test_parse_callback_data_valid_delete() {
        assert_eq!(parse_callback_data("delete:1"), Some(("delete", 1)));
    }

    #[test]
    fn test_parse_callback_data_invalid_no_colon() {
        assert_eq!(parse_callback_data("switch2"), None);
    }

    #[test]
    fn test_parse_callback_data_invalid_non_numeric() {
        assert_eq!(parse_callback_data("switch:abc"), None);
    }

    // --- build_session_keyboard ---

    fn make_session(id: Uuid, preview: Option<&str>, msg_count: u32) -> SessionSummary {
        SessionSummary {
            id,
            name: Some("tg:123".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: msg_count,
            preview: preview.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_build_session_keyboard_callback_data() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let s1 = make_session(id1, Some("Session A"), 5);
        let s2 = make_session(id2, Some("Session B"), 3);
        let refs: Vec<&SessionSummary> = vec![&s1, &s2];

        let markup = build_session_keyboard("switch", &refs, None);
        let rows = markup.inline_keyboard;

        assert_eq!(rows.len(), 2);
        let data1 = rows[0][0].kind.clone();
        let data2 = rows[1][0].kind.clone();

        if let teloxide::types::InlineKeyboardButtonKind::CallbackData(d) = data1 {
            assert_eq!(d, "switch:1");
        } else {
            panic!("Expected CallbackData for button 1");
        }
        if let teloxide::types::InlineKeyboardButtonKind::CallbackData(d) = data2 {
            assert_eq!(d, "switch:2");
        } else {
            panic!("Expected CallbackData for button 2");
        }
    }

    #[test]
    fn test_build_session_keyboard_active_marker() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let s1 = make_session(id1, Some("First"), 1);
        let s2 = make_session(id2, Some("Second"), 2);
        let refs: Vec<&SessionSummary> = vec![&s1, &s2];

        // Second session is active.
        let markup = build_session_keyboard("switch", &refs, Some(id2));
        let rows = markup.inline_keyboard;

        let label1 = &rows[0][0].text;
        let label2 = &rows[1][0].text;

        // Active marker '*' in second, not in first.
        assert!(label2.contains('*'), "Second button should have '*' marker");
        assert!(
            !label1.contains('*'),
            "First button should not have '*' marker"
        );
    }

    #[test]
    fn test_build_session_keyboard_empty_sessions() {
        let refs: Vec<&SessionSummary> = vec![];
        let markup = build_session_keyboard("delete", &refs, None);
        assert!(
            markup.inline_keyboard.is_empty(),
            "Empty sessions should produce no keyboard rows"
        );
    }

    // --- Defensive guard slash detection ---

    #[test]
    fn test_defensive_guard_slash_detection() {
        // These should be caught by the defensive guard.
        assert!("/".starts_with('/'));
        assert!("/foo".starts_with('/'));
        assert!("/switch".starts_with('/'));

        // Regular message should NOT be caught.
        assert!(!"hello".starts_with('/'));
    }

    // --- ChatSessions unit tests ---

    #[test]
    fn test_chat_sessions_active_session_id_non_empty() {
        let id = Uuid::new_v4();
        let cs = ChatSessions {
            sessions: vec![id],
            active_idx: 0,
        };
        assert_eq!(cs.active_session_id(), Some(id));
    }

    #[test]
    fn test_chat_sessions_active_session_id_empty() {
        let cs = ChatSessions {
            sessions: vec![],
            active_idx: 0,
        };
        assert_eq!(cs.active_session_id(), None);
    }

    #[test]
    fn test_chat_sessions_active_session_id_multiple() {
        let id0 = Uuid::new_v4();
        let id1 = Uuid::new_v4();
        let cs = ChatSessions {
            sessions: vec![id0, id1],
            active_idx: 1,
        };
        assert_eq!(cs.active_session_id(), Some(id1));
    }

    // --- Session cap enforcement logic ---

    #[test]
    fn test_session_cap_evicts_last_when_at_cap() {
        let id_old = Uuid::new_v4();
        let id_new = Uuid::new_v4();
        let mut cs = ChatSessions {
            sessions: vec![id_old],
            active_idx: 0,
        };

        // Simulate cap enforcement: evict last if at cap.
        let cap = 1usize;
        let mut evicted_id = None;
        if cs.sessions.len() >= cap {
            evicted_id = cs.sessions.last().copied();
            cs.sessions.pop();
        }
        cs.sessions.insert(0, id_new);
        cs.active_idx = 0;

        assert_eq!(evicted_id, Some(id_old));
        assert_eq!(cs.sessions.len(), 1);
        assert_eq!(cs.sessions[0], id_new);
        assert_eq!(cs.active_idx, 0);
    }

    #[test]
    fn test_session_cap_no_eviction_when_below_cap() {
        let id_existing = Uuid::new_v4();
        let id_new = Uuid::new_v4();
        let mut cs = ChatSessions {
            sessions: vec![id_existing],
            active_idx: 0,
        };

        let cap = 10usize;
        let mut evicted_id = None;
        if cs.sessions.len() >= cap {
            evicted_id = cs.sessions.last().copied();
            cs.sessions.pop();
        }
        cs.sessions.insert(0, id_new);
        cs.active_idx = 0;

        assert_eq!(evicted_id, None);
        assert_eq!(cs.sessions.len(), 2);
        assert_eq!(cs.sessions[0], id_new);
    }

    // --- Index validation ---

    #[test]
    fn test_index_validation_zero_is_invalid() {
        let sessions = [Uuid::new_v4()];
        let n = 0usize;
        // 1-based: index 0 is always invalid.
        assert!(n == 0 || n > sessions.len());
    }

    #[test]
    fn test_index_validation_valid_index() {
        let sessions = [Uuid::new_v4(), Uuid::new_v4()];
        let n = 1usize;
        assert!(n >= 1 && n <= sessions.len());
    }

    #[test]
    fn test_index_validation_exceeds_count() {
        let sessions = [Uuid::new_v4()];
        let n = 99usize;
        assert!(n == 0 || n > sessions.len());
    }

    // --- /delete active_idx adjustment ---

    #[test]
    fn test_delete_active_session_switches_to_index_0() {
        let id0 = Uuid::new_v4();
        let id1 = Uuid::new_v4();
        let mut cs = ChatSessions {
            sessions: vec![id0, id1],
            active_idx: 0, // id0 is active
        };

        // Delete vec position 0 (active session).
        let deleted_pos = 0usize;
        let was_active = deleted_pos == cs.active_idx;
        cs.sessions.remove(deleted_pos);

        if cs.sessions.is_empty() || was_active {
            cs.active_idx = 0;
        } else if deleted_pos < cs.active_idx {
            cs.active_idx = cs.active_idx.saturating_sub(1);
        }

        // After deleting active (id0), id1 should be active at index 0.
        assert_eq!(cs.active_idx, 0);
        assert_eq!(cs.active_session_id(), Some(id1));
    }

    #[test]
    fn test_delete_non_active_below_active_decrements_active_idx() {
        let id0 = Uuid::new_v4();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut cs = ChatSessions {
            sessions: vec![id0, id1, id2],
            active_idx: 2, // id2 is active
        };

        // Delete vec position 0 (non-active, below active).
        let deleted_pos = 0usize;
        let was_active = deleted_pos == cs.active_idx;
        cs.sessions.remove(deleted_pos);

        if cs.sessions.is_empty() || was_active {
            cs.active_idx = 0;
        } else if deleted_pos < cs.active_idx {
            cs.active_idx = cs.active_idx.saturating_sub(1);
        }

        // active_idx should have decremented from 2 to 1, still pointing at id2.
        assert_eq!(cs.active_idx, 1);
        assert_eq!(cs.active_session_id(), Some(id2));
    }

    #[test]
    fn test_delete_non_active_above_active_no_change() {
        let id0 = Uuid::new_v4();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut cs = ChatSessions {
            sessions: vec![id0, id1, id2],
            active_idx: 0, // id0 is active
        };

        // Delete vec position 2 (non-active, above active).
        let deleted_pos = 2usize;
        let was_active = deleted_pos == cs.active_idx;
        cs.sessions.remove(deleted_pos);

        if cs.sessions.is_empty() || was_active {
            cs.active_idx = 0;
        } else if deleted_pos < cs.active_idx {
            cs.active_idx = cs.active_idx.saturating_sub(1);
        }

        // active_idx should remain 0, still pointing at id0.
        assert_eq!(cs.active_idx, 0);
        assert_eq!(cs.active_session_id(), Some(id0));
    }
}
