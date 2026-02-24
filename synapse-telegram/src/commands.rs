//! Telegram bot slash-command handlers for Synapse session management.
//!
//! Implements the `/help`, `/new`, `/history`, `/list`, `/switch N`, and `/delete N`
//! commands. None of these commands invoke LLM inference — they manage sessions only.

use std::sync::Arc;

use chrono::TimeZone;
use synapse_core::session::Session;
use synapse_core::{Config, SessionStore, SessionSummary};
use teloxide::prelude::*;
use teloxide::types::Message as TgMessage;
use teloxide::utils::command::BotCommands;

use crate::handlers::{ChatSessionMap, ChatSessions, chunk_message, is_authorized};

/// All slash commands supported by the Synapse Telegram bot.
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
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
    /// Switch to session number N (1-based index).
    #[command(description = "Switch to session N")]
    Switch(usize),
    /// Delete session number N (1-based index).
    #[command(description = "Delete session N")]
    Delete(usize),
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
        Command::Help => cmd_help(&bot, &msg).await,
        Command::New => cmd_new(&bot, &msg, &config, &storage, &chat_map).await,
        Command::History => cmd_history(&bot, &msg, &storage, &chat_map).await,
        Command::List => cmd_list(&bot, &msg, &storage, &chat_map).await,
        Command::Switch(n) => cmd_switch(&bot, &msg, n, &storage, &chat_map).await,
        Command::Delete(n) => cmd_delete(&bot, &msg, n, &config, &storage, &chat_map).await,
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

/// Switch the active session to the N-th session (1-based index from `/list` ordering).
async fn cmd_switch(
    bot: &Bot,
    msg: &TgMessage,
    n: usize,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    // Reconstruct the display-order list (same ordering as /list).
    let session_uuids = {
        let map = chat_map.read().await;
        map.get(&chat_id)
            .map(|cs| cs.sessions.clone())
            .unwrap_or_default()
    };

    let all_sessions: Vec<SessionSummary> = storage.list_sessions().await.unwrap_or_default();
    let display_list: Vec<&SessionSummary> = all_sessions
        .iter()
        .filter(|s| session_uuids.contains(&s.id))
        .collect();

    if n == 0 || n > display_list.len() {
        bot.send_message(
            msg.chat.id,
            format!(
                "Invalid session index {}. Use /list to see available sessions.",
                n
            ),
        )
        .await?;
        return Ok(());
    }

    let target_id = display_list[n - 1].id;

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
    bot.send_message(msg.chat.id, format!("Switched to session {}.", n))
        .await?;
    Ok(())
}

/// Delete the N-th session (1-based index from `/list` ordering).
///
/// If the active session is deleted and others remain, switches to session 1 (index 0).
/// If the active session is deleted and no sessions remain, auto-creates a new session.
/// If a non-active session is deleted, adjusts `active_idx` if needed.
async fn cmd_delete(
    bot: &Bot,
    msg: &TgMessage,
    n: usize,
    config: &Config,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    // Reconstruct the display-order list (same ordering as /list).
    let session_uuids = {
        let map = chat_map.read().await;
        map.get(&chat_id)
            .map(|cs| cs.sessions.clone())
            .unwrap_or_default()
    };

    let all_sessions: Vec<SessionSummary> = storage.list_sessions().await.unwrap_or_default();
    let display_list: Vec<&SessionSummary> = all_sessions
        .iter()
        .filter(|s| session_uuids.contains(&s.id))
        .collect();

    if n == 0 || n > display_list.len() {
        bot.send_message(
            msg.chat.id,
            format!(
                "Invalid session index {}. Use /list to see available sessions.",
                n
            ),
        )
        .await?;
        return Ok(());
    }

    let target_id = display_list[n - 1].id;

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

    bot.send_message(msg.chat.id, reply).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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
