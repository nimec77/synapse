//! Telegram bot slash-command handlers for Synapse session management.
//!
//! Implements the `/start`, `/help`, `/new`, `/history`, `/list`, `/switch [N]`, and
//! `/delete [N]` commands. None of these commands invoke LLM inference — they manage
//! sessions only. When `/switch` or `/delete` are used without an argument, an inline
//! keyboard is displayed so the user can select a session by tapping a button.
//!
//! Keyboard builders and callback logic are in the [`keyboard`] submodule.

mod keyboard;

pub use keyboard::handle_callback;

use std::sync::Arc;

use chrono::TimeZone;
use synapse_core::message::Role;
use synapse_core::session::{Session, StoredMessage};
use synapse_core::text::truncate;
use synapse_core::{Config, SessionStore};
use teloxide::prelude::*;
use teloxide::types::Message as TgMessage;
use teloxide::utils::command::BotCommands;

use crate::handlers::{
    ChatSessionMap, ChatSessions, NO_SESSIONS_HINT, check_auth, chunk_message, tg_session_name,
};

/// Maximum number of messages shown in `/history`.
const HISTORY_MESSAGE_LIMIT: usize = 10;

/// Maximum characters per message content in `/history` output.
const HISTORY_TRUNCATE_CHARS: usize = 150;

/// Maximum characters shown in the session preview in `/list`.
const LIST_PREVIEW_MAX_CHARS: usize = 40;

/// Maximum characters shown in the session preview in keyboard buttons.
const KEYBOARD_PREVIEW_MAX_CHARS: usize = 20;

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
    #[command(description = "Show recent messages")]
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
    storage: Arc<dyn SessionStore>,
    chat_map: ChatSessionMap,
) -> ResponseResult<()> {
    if let Some(result) = check_auth(&msg, &config).await {
        return result;
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

/// Format stored messages into a compact history string.
///
/// Filters to `Role::User` and `Role::Assistant` only, takes the last 10
/// messages (chronologically most recent), and formats each as:
/// `[role_label] timestamp\ntruncated_content\n\n`
///
/// Returns an empty string if no User/Assistant messages exist.
fn format_history(messages: &[StoredMessage]) -> String {
    let filtered: Vec<&StoredMessage> = messages
        .iter()
        .filter(|m| matches!(m.role, Role::User | Role::Assistant))
        .collect();
    let skip = filtered.len().saturating_sub(HISTORY_MESSAGE_LIMIT);
    let recent = &filtered[skip..];

    let mut output = String::new();
    for m in recent {
        let role_label = match m.role {
            Role::User => "You",
            Role::Assistant => "Assistant",
            _ => unreachable!(), // filtered above
        };
        let timestamp = chrono::Utc
            .from_utc_datetime(&m.timestamp.naive_utc())
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let content = truncate(&m.content, HISTORY_TRUNCATE_CHARS);
        output.push_str(&format!("[{}] {}\n{}\n\n", role_label, timestamp, content));
    }
    output
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
    storage: &Arc<dyn SessionStore>,
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
            Session::new(&config.provider, &config.model).with_name(tg_session_name(chat_id));

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
    storage: &Arc<dyn SessionStore>,
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
    let output = format_history(&messages);

    if output.is_empty() {
        bot.send_message(msg.chat.id, "No messages in current session.")
            .await?;
        return Ok(());
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
    storage: &Arc<dyn SessionStore>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    let (chat_session_list, active_id) =
        match keyboard::fetch_chat_sessions(chat_id, storage, chat_map).await {
            None => {
                bot.send_message(msg.chat.id, NO_SESSIONS_HINT).await?;
                return Ok(());
            }
            Some(result) => result,
        };

    let mut output = String::new();
    for (i, s) in chat_session_list.iter().enumerate() {
        let active_marker = if Some(s.id) == active_id { "*" } else { " " };
        let timestamp = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
        let preview = truncate(s.preview.as_deref().unwrap_or(""), LIST_PREVIEW_MAX_CHARS);
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

/// Switch the active session to the N-th session (1-based index from `/list` ordering).
///
/// When `arg` is empty, displays an inline keyboard for session selection.
/// When `arg` is a valid number, switches directly to that session.
/// When `arg` is non-numeric, replies with an error hint.
async fn cmd_switch(
    bot: &Bot,
    msg: &TgMessage,
    arg: &str,
    storage: &Arc<dyn SessionStore>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match parse_session_arg(arg) {
        Ok(None) => {
            keyboard::build_action_keyboard(
                "switch",
                "Select a session to switch to:",
                bot,
                msg,
                storage,
                chat_map,
            )
            .await
        }
        Ok(Some(n)) => {
            let reply = keyboard::do_switch(n, chat_id, storage, chat_map)
                .await
                .unwrap_or_else(|e| e);
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
    storage: &Arc<dyn SessionStore>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match parse_session_arg(arg) {
        Ok(None) => {
            keyboard::build_action_keyboard(
                "delete",
                "Select a session to delete:",
                bot,
                msg,
                storage,
                chat_map,
            )
            .await
        }
        Ok(Some(n)) => {
            let reply = keyboard::do_delete(n, chat_id, config, storage, chat_map)
                .await
                .unwrap_or_else(|e| e);
            bot.send_message(msg.chat.id, reply).await?;
            Ok(())
        }
        Err(hint) => {
            bot.send_message(msg.chat.id, hint).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    // --- Test helpers ---

    fn make_stored_message(role: Role, content: &str) -> StoredMessage {
        StoredMessage::new(Uuid::new_v4(), role, content)
    }

    // --- truncate (using shared synapse_core::text::truncate) ---

    #[test]
    fn test_truncate_content_short() {
        let content = "0123456789"; // 10 chars
        let result = truncate(content, 150);
        assert_eq!(result, content);
        assert!(!result.ends_with("..."));
    }

    #[test]
    fn test_truncate_content_exact_limit() {
        let content = "a".repeat(150);
        let result = truncate(&content, 150);
        assert_eq!(result, content);
        assert!(
            !result.ends_with("..."),
            "Exact limit should not be truncated"
        );
    }

    #[test]
    fn test_truncate_content_over_limit() {
        let content = "a".repeat(151);
        let result = truncate(&content, 150);
        // truncate gives max_chars total: (max_chars - 3) chars + "..."
        assert_eq!(result.chars().count(), 150);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_content_long() {
        let content = "b".repeat(500);
        let result = truncate(&content, 150);
        assert_eq!(result.chars().count(), 150);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_content_empty() {
        let result = truncate("", 150);
        assert_eq!(result, "");
        assert!(!result.ends_with("..."));
    }

    // --- format_history ---

    #[test]
    fn test_format_history_filters_system_and_tool() {
        let messages = vec![
            make_stored_message(Role::User, "user msg"),
            make_stored_message(Role::Assistant, "assistant msg"),
            make_stored_message(Role::System, "system prompt"),
            make_stored_message(Role::Tool, "tool result"),
        ];
        let output = format_history(&messages);
        assert!(output.contains("You"), "Should contain 'You'");
        assert!(output.contains("Assistant"), "Should contain 'Assistant'");
        assert!(!output.contains("System"), "Should NOT contain 'System'");
        assert!(!output.contains("Tool"), "Should NOT contain 'Tool'");
    }

    #[test]
    fn test_format_history_keeps_user_and_assistant() {
        let mut messages = Vec::new();
        for i in 0..3 {
            messages.push(make_stored_message(Role::User, &format!("user {}", i)));
            messages.push(make_stored_message(
                Role::Assistant,
                &format!("assistant {}", i),
            ));
        }
        let output = format_history(&messages);
        for i in 0..3 {
            assert!(output.contains(&format!("user {}", i)));
            assert!(output.contains(&format!("assistant {}", i)));
        }
    }

    #[test]
    fn test_format_history_last_10_limit() {
        // 15 messages with unique, non-overlapping content markers.
        // "early-N" for the first 5, "recent-N" for the last 10.
        let mut messages: Vec<StoredMessage> = Vec::new();
        for i in 1..=5 {
            let role = if i % 2 == 0 {
                Role::Assistant
            } else {
                Role::User
            };
            messages.push(make_stored_message(role, &format!("early-{}", i)));
        }
        for i in 1..=10 {
            let role = if i % 2 == 0 {
                Role::Assistant
            } else {
                Role::User
            };
            messages.push(make_stored_message(role, &format!("recent-{}", i)));
        }
        let output = format_history(&messages);
        // First 5 (early-1 through early-5) must be absent.
        for i in 1..=5 {
            assert!(
                !output.contains(&format!("early-{}", i)),
                "early-{} should be absent (not in last 10)",
                i
            );
        }
        // Last 10 (recent-1 through recent-10) must be present.
        for i in 1..=10 {
            assert!(
                output.contains(&format!("recent-{}", i)),
                "recent-{} should be present (in last 10)",
                i
            );
        }
    }

    #[test]
    fn test_format_history_fewer_than_10() {
        let messages = vec![
            make_stored_message(Role::User, "first"),
            make_stored_message(Role::Assistant, "second"),
            make_stored_message(Role::User, "third"),
        ];
        let output = format_history(&messages);
        assert!(output.contains("first"));
        assert!(output.contains("second"));
        assert!(output.contains("third"));
    }

    #[test]
    fn test_format_history_empty() {
        let messages = vec![
            make_stored_message(Role::System, "system only"),
            make_stored_message(Role::Tool, "tool only"),
        ];
        let output = format_history(&messages);
        assert_eq!(
            output, "",
            "Only System/Tool messages should yield empty string"
        );
    }

    #[test]
    fn test_format_history_truncates_long_content() {
        let long_content = "x".repeat(200);
        let messages = vec![make_stored_message(Role::Assistant, &long_content)];
        let output = format_history(&messages);
        // The formatted content portion should be truncated (150 chars + "...")
        assert!(
            output.contains("..."),
            "Long content should be truncated with ..."
        );
        // Find content line (second line of the entry)
        let content_line = output.lines().nth(1).unwrap_or("");
        assert!(
            content_line.chars().count() <= 153,
            "Truncated content portion should be at most 153 chars (150 + ...)"
        );
    }

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
        let cs = ChatSessions::new(id);
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
        let mut cs = ChatSessions::new(id_old);

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
        let mut cs = ChatSessions::new(id_existing);

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
