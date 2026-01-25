//! SQLite storage implementation.
//!
//! Provides [`SqliteStore`] as the default storage backend for session persistence.

use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

use crate::config::SessionConfig;
use crate::message::Role;
use crate::session::{Session, SessionSummary, StoredMessage};
use crate::storage::{CleanupResult, SessionStore, StorageError};

/// SQLite-based session storage.
///
/// Uses connection pooling and WAL mode for performance.
/// Runs migrations automatically on startup.
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    /// Create a new SqliteStore from a database URL.
    ///
    /// The URL should be in the format `sqlite:path/to/database.db`.
    /// Runs migrations automatically and enables WAL mode.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if connection fails.
    /// Returns [`StorageError::Migration`] if migrations fail.
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        // Parse URL and configure connection options
        let url = database_url.strip_prefix("sqlite:").unwrap_or(database_url);

        // Ensure parent directory exists
        let path = PathBuf::from(url);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::Database(format!("failed to create database directory: {}", e))
            })?;
        }

        let options = SqliteConnectOptions::new()
            .filename(url)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let store = Self { pool };

        // Run migrations
        store.run_migrations().await?;

        Ok(store)
    }

    /// Run database migrations.
    async fn run_migrations(&self) -> Result<(), StorageError> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))
    }

    /// Parse a Role from a string.
    fn parse_role(s: &str) -> Result<Role, StorageError> {
        match s {
            "system" => Ok(Role::System),
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            _ => Err(StorageError::InvalidData(format!("unknown role: {}", s))),
        }
    }

    /// Convert Role to string for storage.
    fn role_to_string(role: Role) -> &'static str {
        match role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

#[async_trait]
impl SessionStore for SqliteStore {
    async fn create_session(&self, session: &Session) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, name, provider, model, system_prompt, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session.id.to_string())
        .bind(&session.name)
        .bind(&session.provider)
        .bind(&session.model)
        .bind(&session.system_prompt)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, provider, model, system_prompt, created_at, updated_at
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        match row {
            Some(row) => {
                let id_str: String = row.get("id");
                let id = Uuid::parse_str(&id_str)
                    .map_err(|e| StorageError::InvalidData(format!("invalid UUID: {}", e)))?;

                let created_at_str: String = row.get("created_at");
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map_err(|e| StorageError::InvalidData(format!("invalid datetime: {}", e)))?
                    .with_timezone(&Utc);

                let updated_at_str: String = row.get("updated_at");
                let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| StorageError::InvalidData(format!("invalid datetime: {}", e)))?
                    .with_timezone(&Utc);

                Ok(Some(Session {
                    id,
                    name: row.get("name"),
                    provider: row.get("provider"),
                    model: row.get("model"),
                    system_prompt: row.get("system_prompt"),
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT
                s.id, s.name, s.provider, s.model, s.created_at, s.updated_at,
                (SELECT COUNT(*) FROM messages WHERE session_id = s.id) as message_count,
                (SELECT content FROM messages WHERE session_id = s.id AND role = 'user' ORDER BY timestamp ASC LIMIT 1) as preview
            FROM sessions s
            ORDER BY s.updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut summaries = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| StorageError::InvalidData(format!("invalid UUID: {}", e)))?;

            let created_at_str: String = row.get("created_at");
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| StorageError::InvalidData(format!("invalid datetime: {}", e)))?
                .with_timezone(&Utc);

            let updated_at_str: String = row.get("updated_at");
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| StorageError::InvalidData(format!("invalid datetime: {}", e)))?
                .with_timezone(&Utc);

            let message_count: i32 = row.get("message_count");
            let preview: Option<String> = row.get("preview");

            // Truncate preview to 50 characters
            let preview = preview.map(|p| {
                if p.len() > 50 {
                    format!("{}...", &p[..47])
                } else {
                    p
                }
            });

            summaries.push(SessionSummary {
                id,
                name: row.get("name"),
                provider: row.get("provider"),
                model: row.get("model"),
                created_at,
                updated_at,
                message_count: message_count as u32,
                preview,
            });
        }

        Ok(summaries)
    }

    async fn touch_session(&self, id: Uuid) -> Result<(), StorageError> {
        let result = sqlx::query(
            r#"
            UPDATE sessions SET updated_at = ? WHERE id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(id));
        }

        Ok(())
    }

    async fn delete_session(&self, id: Uuid) -> Result<bool, StorageError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sessions WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError> {
        // Insert message
        sqlx::query(
            r#"
            INSERT INTO messages (id, session_id, role, content, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.session_id.to_string())
        .bind(Self::role_to_string(message.role))
        .bind(&message.content)
        .bind(message.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        // Update session's updated_at
        sqlx::query(
            r#"
            UPDATE sessions SET updated_at = ? WHERE id = ?
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(message.session_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, role, content, timestamp
            FROM messages
            WHERE session_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut messages = Vec::new();
        for row in rows {
            let id_str: String = row.get("id");
            let id = Uuid::parse_str(&id_str)
                .map_err(|e| StorageError::InvalidData(format!("invalid UUID: {}", e)))?;

            let session_id_str: String = row.get("session_id");
            let session_id = Uuid::parse_str(&session_id_str)
                .map_err(|e| StorageError::InvalidData(format!("invalid UUID: {}", e)))?;

            let role_str: String = row.get("role");
            let role = Self::parse_role(&role_str)?;

            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map_err(|e| StorageError::InvalidData(format!("invalid datetime: {}", e)))?
                .with_timezone(&Utc);

            messages.push(StoredMessage {
                id,
                session_id,
                role,
                content: row.get("content"),
                timestamp,
            });
        }

        Ok(messages)
    }

    async fn cleanup(&self, config: &SessionConfig) -> Result<CleanupResult, StorageError> {
        let mut result = CleanupResult::default();

        // Delete sessions older than retention_days
        let cutoff = Utc::now() - chrono::Duration::days(config.retention_days as i64);
        let retention_result = sqlx::query(
            r#"
            DELETE FROM sessions WHERE updated_at < ?
            "#,
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        result.by_retention = retention_result.rows_affected() as u32;

        // Count current sessions
        let count_row = sqlx::query(
            r#"
            SELECT COUNT(*) as count FROM sessions
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        let session_count: i32 = count_row.get("count");

        // Delete oldest sessions if over limit
        if session_count > config.max_sessions as i32 {
            let excess = session_count - config.max_sessions as i32;
            let limit_result = sqlx::query(
                r#"
                DELETE FROM sessions WHERE id IN (
                    SELECT id FROM sessions ORDER BY updated_at ASC LIMIT ?
                )
                "#,
            )
            .bind(excess)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

            result.by_max_limit = limit_result.rows_affected() as u32;
        }

        result.sessions_deleted = result.by_retention + result.by_max_limit;

        Ok(result)
    }
}

/// Create a storage backend from database URL.
///
/// Defaults to `sqlite:~/.config/synapse/sessions.db` if no URL provided.
///
/// # Errors
///
/// Returns [`StorageError`] if storage creation fails.
pub async fn create_storage(
    database_url: Option<&str>,
) -> Result<Box<dyn SessionStore>, StorageError> {
    let url = match database_url {
        Some(url) => url.to_string(),
        None => {
            let config_dir = dirs::home_dir()
                .ok_or_else(|| {
                    StorageError::Database("could not determine home directory".to_string())
                })?
                .join(".config/synapse");

            format!("sqlite:{}", config_dir.join("sessions.db").display())
        }
    };

    let store = SqliteStore::new(&url).await?;
    Ok(Box::new(store))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_role_to_string() {
        assert_eq!(SqliteStore::role_to_string(Role::System), "system");
        assert_eq!(SqliteStore::role_to_string(Role::User), "user");
        assert_eq!(SqliteStore::role_to_string(Role::Assistant), "assistant");
    }

    #[test]
    fn test_parse_role() {
        assert!(matches!(
            SqliteStore::parse_role("system"),
            Ok(Role::System)
        ));
        assert!(matches!(SqliteStore::parse_role("user"), Ok(Role::User)));
        assert!(matches!(
            SqliteStore::parse_role("assistant"),
            Ok(Role::Assistant)
        ));
        assert!(matches!(
            SqliteStore::parse_role("invalid"),
            Err(StorageError::InvalidData(_))
        ));
    }

    /// Create a temporary database for testing.
    async fn create_test_store() -> SqliteStore {
        let db_path = temp_dir().join(format!("synapse_test_{}.db", Uuid::new_v4()));
        let url = format!("sqlite:{}", db_path.display());
        SqliteStore::new(&url)
            .await
            .expect("failed to create test store")
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let store = create_test_store().await;
        let session = Session::new("deepseek", "deepseek-chat");
        let session_id = session.id;

        // Create session
        store.create_session(&session).await.expect("create failed");

        // Get session
        let retrieved = store.get_session(session_id).await.expect("get failed");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, session_id);
        assert_eq!(retrieved.provider, "deepseek");
        assert_eq!(retrieved.model, "deepseek-chat");
    }

    #[tokio::test]
    async fn test_get_session_not_found() {
        let store = create_test_store().await;
        let result = store
            .get_session(Uuid::new_v4())
            .await
            .expect("query failed");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = create_test_store().await;

        // Create two sessions
        let session1 = Session::new("provider1", "model1");
        let session2 = Session::new("provider2", "model2");

        store
            .create_session(&session1)
            .await
            .expect("create failed");
        store
            .create_session(&session2)
            .await
            .expect("create failed");

        // List sessions
        let summaries = store.list_sessions().await.expect("list failed");
        assert_eq!(summaries.len(), 2);
    }

    #[tokio::test]
    async fn test_touch_session() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;
        let original_updated = session.updated_at;

        store.create_session(&session).await.expect("create failed");

        // Wait a bit to ensure timestamp changes
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        store.touch_session(session_id).await.expect("touch failed");

        let updated = store
            .get_session(session_id)
            .await
            .expect("get failed")
            .unwrap();
        assert!(updated.updated_at > original_updated);
    }

    #[tokio::test]
    async fn test_touch_session_not_found() {
        let store = create_test_store().await;
        let result = store.touch_session(Uuid::new_v4()).await;
        assert!(matches!(result, Err(StorageError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;

        store.create_session(&session).await.expect("create failed");

        // Delete session
        let deleted = store
            .delete_session(session_id)
            .await
            .expect("delete failed");
        assert!(deleted);

        // Verify it's gone
        let retrieved = store.get_session(session_id).await.expect("get failed");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_session_not_found() {
        let store = create_test_store().await;
        let deleted = store
            .delete_session(Uuid::new_v4())
            .await
            .expect("delete failed");
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_add_and_get_messages() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;

        store.create_session(&session).await.expect("create failed");

        // Add messages
        let msg1 = StoredMessage::new(session_id, Role::User, "Hello!");
        let msg2 = StoredMessage::new(session_id, Role::Assistant, "Hi there!");

        store.add_message(&msg1).await.expect("add msg1 failed");
        store.add_message(&msg2).await.expect("add msg2 failed");

        // Get messages
        let messages = store.get_messages(session_id).await.expect("get failed");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[0].content, "Hello!");
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_messages_ordered_by_timestamp() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;

        store.create_session(&session).await.expect("create failed");

        // Add messages in order
        for i in 0..5 {
            let msg = StoredMessage::new(session_id, Role::User, format!("Message {}", i));
            store.add_message(&msg).await.expect("add failed");
        }

        // Verify order
        let messages = store.get_messages(session_id).await.expect("get failed");
        for (i, msg) in messages.iter().enumerate() {
            assert_eq!(msg.content, format!("Message {}", i));
        }
    }

    #[tokio::test]
    async fn test_list_sessions_with_message_count_and_preview() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;

        store.create_session(&session).await.expect("create failed");

        // Add a user message (this will be the preview)
        let msg = StoredMessage::new(session_id, Role::User, "What is the weather today?");
        store.add_message(&msg).await.expect("add failed");

        // List sessions
        let summaries = store.list_sessions().await.expect("list failed");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].message_count, 1);
        assert_eq!(
            summaries[0].preview,
            Some("What is the weather today?".to_string())
        );
    }

    #[tokio::test]
    async fn test_preview_truncated_to_50_chars() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;

        store.create_session(&session).await.expect("create failed");

        // Add a long message
        let long_content =
            "This is a very long message that exceeds fifty characters and should be truncated";
        let msg = StoredMessage::new(session_id, Role::User, long_content);
        store.add_message(&msg).await.expect("add failed");

        // List sessions
        let summaries = store.list_sessions().await.expect("list failed");
        let preview = summaries[0].preview.as_ref().unwrap();
        assert_eq!(preview.len(), 50); // 47 chars + "..."
        assert!(preview.ends_with("..."));
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let store = create_test_store().await;
        let session = Session::new("test", "model");
        let session_id = session.id;

        store.create_session(&session).await.expect("create failed");

        // Add messages
        let msg = StoredMessage::new(session_id, Role::User, "Test");
        store.add_message(&msg).await.expect("add failed");

        // Delete session (should cascade to messages)
        store
            .delete_session(session_id)
            .await
            .expect("delete failed");

        // Messages should be gone too (this would fail without CASCADE)
        let messages = store.get_messages(session_id).await.expect("get failed");
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_by_max_sessions() {
        let store = create_test_store().await;

        // Create 5 sessions
        for i in 0..5 {
            let session = Session::new("test", format!("model-{}", i));
            store.create_session(&session).await.expect("create failed");
            // Small delay to ensure different timestamps
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        // Verify 5 sessions exist
        let before = store.list_sessions().await.expect("list failed");
        assert_eq!(before.len(), 5);

        // Run cleanup with max_sessions = 3
        let config = SessionConfig {
            max_sessions: 3,
            retention_days: 365, // Don't delete by retention
            auto_cleanup: true,
        };

        let result = store.cleanup(&config).await.expect("cleanup failed");
        assert_eq!(result.sessions_deleted, 2);
        assert_eq!(result.by_max_limit, 2);
        assert_eq!(result.by_retention, 0);

        // Verify only 3 sessions remain
        let after = store.list_sessions().await.expect("list failed");
        assert_eq!(after.len(), 3);
    }

    #[tokio::test]
    async fn test_cleanup_no_action_when_under_limit() {
        let store = create_test_store().await;

        // Create 2 sessions
        let session1 = Session::new("test", "model1");
        let session2 = Session::new("test", "model2");
        store
            .create_session(&session1)
            .await
            .expect("create failed");
        store
            .create_session(&session2)
            .await
            .expect("create failed");

        // Run cleanup with max_sessions = 10 (under limit)
        let config = SessionConfig {
            max_sessions: 10,
            retention_days: 365,
            auto_cleanup: true,
        };

        let result = store.cleanup(&config).await.expect("cleanup failed");
        assert_eq!(result.sessions_deleted, 0);
        assert_eq!(result.by_max_limit, 0);
        assert_eq!(result.by_retention, 0);

        // All sessions still exist
        let after = store.list_sessions().await.expect("list failed");
        assert_eq!(after.len(), 2);
    }

    #[tokio::test]
    async fn test_cleanup_result_counts() {
        let store = create_test_store().await;

        // Create 3 sessions
        for i in 0..3 {
            let session = Session::new("test", format!("model-{}", i));
            store.create_session(&session).await.expect("create failed");
        }

        // Cleanup with max_sessions = 1
        let config = SessionConfig {
            max_sessions: 1,
            retention_days: 365,
            auto_cleanup: true,
        };

        let result = store.cleanup(&config).await.expect("cleanup failed");

        // Should delete 2 sessions (3 - 1 = 2)
        assert_eq!(result.by_max_limit, 2);
        assert_eq!(
            result.sessions_deleted,
            result.by_max_limit + result.by_retention
        );
    }
}
