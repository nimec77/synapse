//! Inline keyboard builders and callback logic for session selection.
//!
//! Handles the interactive keyboard UX for `/switch [N]` and `/delete [N]`:
//! - `build_session_keyboard` — constructs the `InlineKeyboardMarkup`
//! - `build_action_keyboard` — sends keyboard or hint if no sessions exist
//! - `do_switch` / `do_delete` — executes the action given a 1-based index
//! - `handle_callback` — processes `CallbackQuery` updates from button taps
//! - `fetch_chat_sessions` — shared session-list fetcher
//! - `parse_callback_data` — parses `"action:N"` callback data strings

use std::sync::Arc;

use synapse_core::session::Session;
use synapse_core::text::truncate;
use synapse_core::{Config, SessionStore, SessionSummary};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup};
use uuid::Uuid;

use crate::handlers::{
    ChatSessionMap, ChatSessions, NO_SESSIONS_HINT, is_authorized, tg_session_name,
};

use super::KEYBOARD_PREVIEW_MAX_CHARS;

/// Fetch the display-ordered session list for a chat.
///
/// Returns `Some((sessions, active_id))` if the chat has sessions, `None` otherwise.
/// Sessions are ordered by `updated_at DESC` (matching `/list` ordering).
pub(super) async fn fetch_chat_sessions(
    chat_id: i64,
    storage: &Arc<dyn SessionStore>,
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
pub(super) fn build_session_keyboard(
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
            let preview = truncate(
                s.preview.as_deref().unwrap_or(""),
                KEYBOARD_PREVIEW_MAX_CHARS,
            );
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

/// Send an inline keyboard for session selection.
///
/// `action` is `"switch"` or `"delete"`. `prompt` is the message shown above the keyboard.
/// If no sessions exist, sends `NO_SESSIONS_HINT` instead.
pub(super) async fn build_action_keyboard(
    action: &str,
    prompt: &str,
    bot: &Bot,
    msg: &teloxide::types::Message,
    storage: &Arc<dyn SessionStore>,
    chat_map: &ChatSessionMap,
) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    match fetch_chat_sessions(chat_id, storage, chat_map).await {
        None => {
            bot.send_message(msg.chat.id, NO_SESSIONS_HINT).await?;
        }
        Some((sessions, active_id)) => {
            let refs: Vec<&SessionSummary> = sessions.iter().collect();
            let keyboard = build_session_keyboard(action, &refs, active_id);
            bot.send_message(msg.chat.id, prompt)
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
pub(super) async fn do_switch(
    n: usize,
    chat_id: i64,
    storage: &Arc<dyn SessionStore>,
    chat_map: &ChatSessionMap,
) -> Result<String, String> {
    let (sessions, _) = fetch_chat_sessions(chat_id, storage, chat_map)
        .await
        .ok_or_else(|| NO_SESSIONS_HINT.to_string())?;

    if n == 0 || n > sessions.len() {
        return Err(format!(
            "Invalid session index {}. Use /list to see available sessions.",
            n
        ));
    }

    let target_id = sessions[n - 1].id;

    {
        let mut map = chat_map.write().await;
        if let Some(cs) = map.get_mut(&chat_id)
            && let Some(pos) = cs.sessions.iter().position(|&id| id == target_id)
        {
            cs.active_idx = pos;
        }
    }

    storage.touch_session(target_id).await.ok();
    Ok(format!("Switched to session {}.", n))
}

/// Execute the delete-session-N logic, returning a reply string.
///
/// Re-fetches the session list from storage for consistency (handles staleness).
/// Returns `Ok(reply)` on success or `Err(error_message)` on invalid index.
pub(super) async fn do_delete(
    n: usize,
    chat_id: i64,
    config: &Config,
    storage: &Arc<dyn SessionStore>,
    chat_map: &ChatSessionMap,
) -> Result<String, String> {
    let (sessions, _) = fetch_chat_sessions(chat_id, storage, chat_map)
        .await
        .ok_or_else(|| NO_SESSIONS_HINT.to_string())?;

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
                Session::new(&config.provider, &config.model).with_name(tg_session_name(chat_id));
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

/// Parse callback data in the format "action:N" (e.g., "switch:2", "delete:1").
pub(crate) fn parse_callback_data(data: &str) -> Option<(&str, usize)> {
    let (action, n_str) = data.split_once(':')?;
    let n = n_str.parse::<usize>().ok()?;
    Some((action, n))
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
    storage: Arc<dyn SessionStore>,
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
        return Ok(()); // Silent drop.
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
        "switch" => do_switch(n, chat_id, &storage, &chat_map)
            .await
            .unwrap_or_else(|e| e),
        "delete" => do_delete(n, chat_id, &config, &storage, &chat_map)
            .await
            .unwrap_or_else(|e| e),
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
    use synapse_core::SessionSummary;
    use uuid::Uuid;

    use super::*;

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
}
