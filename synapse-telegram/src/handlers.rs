//! Telegram message handler for the Synapse bot.
//!
//! Handles incoming messages: authorization, session management, agent invocation,
//! and response delivery.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use synapse_core::message::{Message as CoreMessage, Role};
use synapse_core::session::Session;
use synapse_core::{Agent, Config, SessionStore, StoredMessage};
use teloxide::prelude::*;
use teloxide::types::{ChatAction, Message as TgMessage, ParseMode};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Telegram's maximum message length in characters.
const TELEGRAM_MSG_LIMIT: usize = 4096;

/// Error message sent to the user when agent or session operations fail.
const ERROR_REPLY: &str = "Sorry, I encountered an error. Please try again.";

/// Per-chat session state: ordered list of session UUIDs and the active session index.
///
/// Sessions are ordered by `updated_at DESC` (most recent first), matching the
/// 1-based index used in `/list`, `/switch`, and `/delete` commands.
#[derive(Debug, Clone)]
pub struct ChatSessions {
    /// Session UUIDs belonging to this chat. Insertion order reflects DB ordering
    /// (most recent first) at bot startup; new sessions are prepended on `/new`.
    pub sessions: Vec<Uuid>,
    /// Index into `sessions` indicating the currently active session.
    pub active_idx: usize,
}

impl ChatSessions {
    /// Return the currently active session UUID, or `None` if `sessions` is empty.
    pub fn active_session_id(&self) -> Option<Uuid> {
        self.sessions.get(self.active_idx).copied()
    }
}

/// In-memory map from Telegram chat IDs to multi-session state.
pub type ChatSessionMap = Arc<RwLock<HashMap<i64, ChatSessions>>>;

/// Handle an incoming Telegram message.
///
/// Steps:
/// 1. Check user authorization (silent drop if not in `allowed_users`).
/// 2. Look up or create a session for this chat.
/// 3. Load conversation history, append the new user message.
/// 4. Store the user message in the database.
/// 5. Send a typing indicator.
/// 6. Call the agent for a response.
/// 7. Store and send the response (chunked if > 4096 chars).
pub async fn handle_message(
    bot: Bot,
    msg: TgMessage,
    config: Arc<Config>,
    agent: Arc<Agent>,
    storage: Arc<Box<dyn SessionStore>>,
    chat_map: ChatSessionMap,
) -> ResponseResult<()> {
    // Step 1: User authorization.
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let allowed_users = config
        .telegram
        .as_ref()
        .map(|t| t.allowed_users.as_slice())
        .unwrap_or(&[]);

    if !is_authorized(user_id, allowed_users) {
        return Ok(()); // Silent drop — do not reveal bot existence.
    }

    // Step 2: Extract text content. Non-text updates are ignored.
    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => return Ok(()),
    };

    let chat_id = msg.chat.id.0;

    // Step 3: Resolve or create session for this chat.
    let session_id = match resolve_session(chat_id, &config, &storage, &chat_map).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to resolve session for chat {}: {}", chat_id, e);
            bot.send_message(msg.chat.id, ERROR_REPLY).await?;
            return Ok(());
        }
    };

    // Step 4: Load conversation history and append user message.
    let stored_messages = storage.get_messages(session_id).await.unwrap_or_default();

    let mut messages: Vec<CoreMessage> = stored_messages
        .into_iter()
        .map(|m| CoreMessage::new(m.role, m.content))
        .collect();

    let user_message = CoreMessage::new(Role::User, &text);
    messages.push(user_message);

    // Step 5: Store the user message before calling the agent.
    let stored_user_msg = StoredMessage::new(session_id, Role::User, &text);
    if let Err(e) = storage.add_message(&stored_user_msg).await {
        tracing::warn!("Failed to store user message for chat {}: {}", chat_id, e);
    }

    // Step 6: Send typing indicator.
    bot.send_chat_action(msg.chat.id, ChatAction::Typing)
        .await
        .ok(); // Non-critical — ignore failure.

    // Step 7: Call agent for a response.
    match agent.complete(&mut messages).await {
        Ok(response) => {
            // Store the assistant response.
            let stored_response =
                StoredMessage::new(session_id, Role::Assistant, &response.content);
            if let Err(e) = storage.add_message(&stored_response).await {
                tracing::warn!(
                    "Failed to store assistant message for chat {}: {}",
                    chat_id,
                    e
                );
            }

            // Convert Markdown to Telegram HTML, chunk, and send with fallback.
            let html = crate::format::md_to_telegram_html(&response.content);
            let chunks = crate::format::chunk_html(&html);
            let mut html_failed = false;
            for chunk in &chunks {
                match bot
                    .send_message(msg.chat.id, chunk)
                    .parse_mode(ParseMode::Html)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(
                            "HTML send failed for chat {}, falling back to plain text: {}",
                            chat_id,
                            e
                        );
                        html_failed = true;
                        break;
                    }
                }
            }
            if html_failed {
                let plain_chunks = chunk_message(&response.content);
                for plain_chunk in plain_chunks {
                    bot.send_message(msg.chat.id, plain_chunk).await?;
                }
            }
        }
        Err(e) => {
            tracing::error!("Agent error for chat {}: {}", chat_id, e);
            bot.send_message(msg.chat.id, ERROR_REPLY).await?;
        }
    }

    Ok(())
}

/// Resolve the session ID for a chat, creating a new session if needed.
///
/// Uses a read-lock first for the common case (session already exists),
/// then a write-lock with double-check to prevent race conditions on creation.
async fn resolve_session(
    chat_id: i64,
    config: &Config,
    storage: &Arc<Box<dyn SessionStore>>,
    chat_map: &ChatSessionMap,
) -> anyhow::Result<Uuid> {
    // Fast path: check with a read lock.
    {
        let map = chat_map.read().await;
        if let Some(chat_sessions) = map.get(&chat_id)
            && let Some(id) = chat_sessions.active_session_id()
        {
            storage.touch_session(id).await.ok();
            return Ok(id);
        }
    }

    // Slow path: create a new session, with write-lock double-check.
    let session =
        Session::new(&config.provider, &config.model).with_name(format!("tg:{}", chat_id));

    storage
        .create_session(&session)
        .await
        .map_err(|e| anyhow!("failed to create session for chat {}: {}", chat_id, e))?;

    let mut map = chat_map.write().await;
    // Double-check: another task might have inserted while we awaited the write lock.
    if let Some(existing) = map.get(&chat_id)
        && let Some(existing_id) = existing.active_session_id()
    {
        return Ok(existing_id);
    }
    map.insert(
        chat_id,
        ChatSessions {
            sessions: vec![session.id],
            active_idx: 0,
        },
    );
    Ok(session.id)
}

/// Split a message into chunks that fit within Telegram's 4096-character limit.
///
/// Splitting priority:
/// 1. Paragraph boundaries (`\n\n`)
/// 2. Newline boundaries (`\n`)
/// 3. Space boundaries
/// 4. Hard split at the limit (last resort)
pub fn chunk_message(text: &str) -> Vec<&str> {
    if text.len() <= TELEGRAM_MSG_LIMIT {
        return vec![text];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while remaining.len() > TELEGRAM_MSG_LIMIT {
        let limit = crate::format::floor_char_boundary(remaining, TELEGRAM_MSG_LIMIT);
        let slice = &remaining[..limit];

        // Try paragraph boundary.
        let split_at = slice
            .rfind("\n\n")
            .or_else(|| slice.rfind('\n'))
            .or_else(|| slice.rfind(' '))
            .map(|pos| pos + 1) // Include the delimiter in the first chunk.
            .unwrap_or(TELEGRAM_MSG_LIMIT); // Hard split as last resort.

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk);
        remaining = rest.trim_start_matches('\n');
    }

    if !remaining.is_empty() {
        chunks.push(remaining);
    }

    chunks
}

/// Check whether a Telegram user ID is in the allowed users list.
///
/// Returns `false` for an empty list (secure by default).
pub fn is_authorized(user_id: u64, allowed_users: &[u64]) -> bool {
    allowed_users.contains(&user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Authorization tests

    #[test]
    fn test_is_authorized_user_in_list() {
        let allowed = vec![123456789u64, 987654321u64];
        assert!(is_authorized(123456789, &allowed));
    }

    #[test]
    fn test_is_authorized_user_not_in_list() {
        let allowed = vec![123456789u64];
        assert!(!is_authorized(999999999, &allowed));
    }

    #[test]
    fn test_is_authorized_empty_list() {
        let allowed: Vec<u64> = vec![];
        assert!(!is_authorized(123456789, &allowed));
    }

    // Message chunking tests

    #[test]
    fn test_chunk_short_message() {
        let text = "Hello, world!";
        let chunks = chunk_message(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_chunk_long_message() {
        // Create a message longer than 4096 chars.
        let text = "a".repeat(5000);
        let chunks = chunk_message(&text);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
        }
        // All content should be preserved (joined).
        let joined: String = chunks.join("");
        assert_eq!(joined, text);
    }

    #[test]
    fn test_chunk_at_boundary() {
        // Create two paragraphs that together exceed the limit.
        let part1 = "a".repeat(3000);
        let part2 = "b".repeat(3000);
        let text = format!("{}\n\n{}", part1, part2);
        let chunks = chunk_message(&text);
        // Should split at the paragraph boundary.
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
        }
    }

    #[test]
    fn test_chunk_message_multibyte() {
        // Cyrillic: 2 bytes per char → 4096 byte limit lands mid-char without the fix.
        let text = "Привет мир ".repeat(400); // ~4400 chars, ~8800 bytes
        let chunks = chunk_message(&text);
        assert!(chunks.len() >= 2);
        for chunk in chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
            // Panics on invalid UTF-8 boundary — this is the regression guard.
            let _ = chunk.chars().count();
        }
    }
}
