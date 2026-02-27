//! Startup helpers: bot token resolution and chat-map reconstruction.

use std::collections::HashMap;

use synapse_core::{Config, SessionStore, SessionSummary};

use crate::handlers::ChatSessions;

#[cfg(test)]
mod tests;

/// Resolve the bot token with the following priority:
///
/// 1. `TELEGRAM_BOT_TOKEN` environment variable (if set and non-empty).
/// 2. `telegram.token` in `config.toml`.
///
/// The token is **never** passed to any tracing macro.
///
/// # Errors
///
/// Returns an error if neither source provides a token.
pub fn resolve_bot_token(config: &Config) -> anyhow::Result<String> {
    if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN")
        && !token.is_empty()
    {
        return Ok(token);
    }
    config
        .telegram
        .as_ref()
        .and_then(|t| t.token.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Bot token required: set TELEGRAM_BOT_TOKEN env var or telegram.token in config"
            )
        })
}

/// Rebuild the in-memory chat-ID-to-session map from persisted sessions.
///
/// Sessions created by this bot follow the naming convention `"tg:<chat_id>"`.
/// Any session without this prefix is ignored. `list_sessions()` returns sessions
/// ordered by `updated_at DESC`, so the first UUID encountered per chat is the
/// most recently updated and becomes the active session (index 0).
pub async fn rebuild_chat_map(storage: &dyn SessionStore) -> HashMap<i64, ChatSessions> {
    let sessions: Vec<SessionSummary> = storage.list_sessions().await.unwrap_or_default();
    let mut map: HashMap<i64, Vec<uuid::Uuid>> = HashMap::new();

    for s in &sessions {
        if let Some(chat_id) = s
            .name
            .as_deref()
            .and_then(|n| n.strip_prefix("tg:"))
            .and_then(|id_str| id_str.parse::<i64>().ok())
        {
            map.entry(chat_id).or_default().push(s.id);
        }
    }

    map.into_iter()
        .map(|(chat_id, session_ids)| {
            (
                chat_id,
                ChatSessions {
                    sessions: session_ids,
                    active_idx: 0,
                },
            )
        })
        .collect()
}
