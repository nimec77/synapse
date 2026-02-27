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
mod tests;
