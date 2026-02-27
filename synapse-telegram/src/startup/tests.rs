use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use synapse_core::config::SessionConfig;
use synapse_core::session::{Session, StoredMessage};
use synapse_core::storage::{CleanupResult, SessionStore, StorageError};
use synapse_core::{Config, SessionSummary, TelegramConfig};
use uuid::Uuid;

use super::*;

/// Guards tests that mutate environment variables to prevent race conditions.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Minimal in-memory SessionStore mock for chat map tests.
struct MockSessionStore {
    sessions: Vec<SessionSummary>,
}

impl MockSessionStore {
    fn with_sessions(sessions: Vec<SessionSummary>) -> Self {
        Self { sessions }
    }
}

#[async_trait]
impl SessionStore for MockSessionStore {
    async fn create_session(&self, _session: &Session) -> Result<(), StorageError> {
        Ok(())
    }

    async fn get_session(&self, _id: Uuid) -> Result<Option<Session>, StorageError> {
        Ok(None)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError> {
        Ok(self.sessions.clone())
    }

    async fn touch_session(&self, _id: Uuid) -> Result<(), StorageError> {
        Ok(())
    }

    async fn delete_session(&self, _id: Uuid) -> Result<bool, StorageError> {
        Ok(false)
    }

    async fn add_message(&self, _message: &StoredMessage) -> Result<(), StorageError> {
        Ok(())
    }

    async fn get_messages(&self, _session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError> {
        Ok(vec![])
    }

    async fn cleanup(&self, _config: &SessionConfig) -> Result<CleanupResult, StorageError> {
        Ok(CleanupResult::default())
    }
}

// Token resolution tests

#[test]
fn test_resolve_token_env_var() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: guarded by mutex; single-threaded section.
    unsafe { std::env::set_var("TELEGRAM_BOT_TOKEN", "env-token-value") };

    let config = Config {
        telegram: Some(TelegramConfig {
            token: Some("config-token".to_string()),
            ..TelegramConfig::default()
        }),
        ..Config::default()
    };

    let result = resolve_bot_token(&config);
    assert_eq!(result.unwrap(), "env-token-value");

    // SAFETY: guarded by mutex.
    unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN") };
}

#[test]
fn test_resolve_token_config() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: guarded by mutex.
    unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN") };

    let config = Config {
        telegram: Some(TelegramConfig {
            token: Some("config-token".to_string()),
            ..TelegramConfig::default()
        }),
        ..Config::default()
    };

    let result = resolve_bot_token(&config);
    assert_eq!(result.unwrap(), "config-token");
}

#[test]
fn test_resolve_token_none() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: guarded by mutex.
    unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN") };

    let config = Config::default(); // No telegram config.
    let result = resolve_bot_token(&config);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("TELEGRAM_BOT_TOKEN"));
}

#[test]
fn test_resolve_token_empty_env_var() {
    let _guard = ENV_MUTEX.lock().unwrap();
    // SAFETY: guarded by mutex.
    unsafe { std::env::set_var("TELEGRAM_BOT_TOKEN", "") };

    let config = Config {
        telegram: Some(TelegramConfig {
            token: Some("fallback-config-token".to_string()),
            ..TelegramConfig::default()
        }),
        ..Config::default()
    };

    let result = resolve_bot_token(&config);
    // Empty env var should fall through to config.
    assert_eq!(result.unwrap(), "fallback-config-token");

    // SAFETY: guarded by mutex.
    unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN") };
}

// Chat map reconstruction tests

#[tokio::test]
async fn test_rebuild_chat_map_empty() {
    let store = MockSessionStore::with_sessions(vec![]);
    let map = rebuild_chat_map(&store).await;
    assert!(map.is_empty());
}

#[tokio::test]
async fn test_rebuild_chat_map_with_telegram_sessions() {
    let now = Utc::now();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let sessions = vec![
        SessionSummary {
            id: id1,
            name: Some("tg:111222333".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            preview: None,
        },
        SessionSummary {
            id: id2,
            name: Some("tg:444555666".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            preview: None,
        },
    ];

    let store = MockSessionStore::with_sessions(sessions);
    let map = rebuild_chat_map(&store).await;

    assert_eq!(map.len(), 2);
    let cs1 = map.get(&111222333i64).unwrap();
    assert_eq!(cs1.sessions, vec![id1]);
    assert_eq!(cs1.active_idx, 0);
    let cs2 = map.get(&444555666i64).unwrap();
    assert_eq!(cs2.sessions, vec![id2]);
    assert_eq!(cs2.active_idx, 0);
}

#[tokio::test]
async fn test_rebuild_chat_map_ignores_non_telegram() {
    let now = Utc::now();
    let tg_id = Uuid::new_v4();

    let sessions = vec![
        SessionSummary {
            id: tg_id,
            name: Some("tg:123456789".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: now,
            updated_at: now,
            message_count: 5,
            preview: None,
        },
        SessionSummary {
            id: Uuid::new_v4(),
            name: Some("My CLI session".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: now,
            updated_at: now,
            message_count: 10,
            preview: None,
        },
        SessionSummary {
            id: Uuid::new_v4(),
            name: None, // Unnamed session.
            provider: "anthropic".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            preview: None,
        },
    ];

    let store = MockSessionStore::with_sessions(sessions);
    let map = rebuild_chat_map(&store).await;

    // Only the tg: session should be in the map.
    assert_eq!(map.len(), 1);
    let cs = map.get(&123456789i64).unwrap();
    assert_eq!(cs.sessions, vec![tg_id]);
    assert_eq!(cs.active_idx, 0);
}

#[tokio::test]
async fn test_rebuild_chat_map_multi_session_per_chat() {
    let now = Utc::now();
    let older_time = now - chrono::Duration::hours(1);
    let newest_id = Uuid::new_v4();
    let older_id = Uuid::new_v4();

    // DB returns ORDER BY updated_at DESC — most recent first.
    let sessions = vec![
        SessionSummary {
            id: newest_id,
            name: Some("tg:999888777".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: older_time,
            updated_at: now,
            message_count: 3,
            preview: None,
        },
        SessionSummary {
            id: older_id,
            name: Some("tg:999888777".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: older_time,
            updated_at: older_time,
            message_count: 5,
            preview: None,
        },
    ];

    let store = MockSessionStore::with_sessions(sessions);
    let map = rebuild_chat_map(&store).await;

    // Both sessions should be grouped under the same chat_id.
    assert_eq!(map.len(), 1);
    let cs = map.get(&999888777i64).unwrap();
    assert_eq!(cs.sessions.len(), 2);
    // Most recently updated session should be first (index 0 = active).
    assert_eq!(cs.sessions[0], newest_id);
    assert_eq!(cs.sessions[1], older_id);
    assert_eq!(cs.active_idx, 0);
}
