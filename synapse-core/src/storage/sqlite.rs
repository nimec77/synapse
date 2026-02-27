//! SQLite storage implementation.
//!
//! Provides [`SqliteStore`] as the default storage backend for session persistence.
//!
//! ## Database Initialization
//!
//! When [`create_storage`] is called, the following happens automatically:
//!
//! 1. **URL Resolution**: Database URL is determined by priority:
//!    - `DATABASE_URL` environment variable
//!    - `session.database_url` from config.toml
//!    - Default: `sqlite:~/.config/synapse/sessions.db`
//!
//! 2. **Directory Creation**: Parent directory is created if missing
//!
//! 3. **Database Creation**: SQLite file is created if missing (via `create_if_missing(true)`)
//!
//! 4. **WAL Mode**: Write-Ahead Logging is enabled for better performance
//!
//! 5. **Migrations**: Schema migrations run automatically using sqlx's embedded migrations
//!
//! No manual setup is required - the database is ready on first use.

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

/// Maximum number of characters for session preview text in `list_sessions`.
const SESSION_PREVIEW_MAX_CHARS: usize = 50;

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
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
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
        s.parse::<Role>().map_err(StorageError::InvalidData)
    }

    /// Convert Role to string for storage.
    fn role_to_string(role: Role) -> &'static str {
        role.as_str()
    }
}

#[async_trait]
impl SessionStore for SqliteStore {
    async fn create_session(&self, session: &Session) -> Result<(), StorageError> {
        tracing::debug!(session_id = %session.id, "sqlite: creating session");
        sqlx::query(
            r#"
            INSERT INTO sessions (id, name, provider, model, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session.id.to_string())
        .bind(&session.name)
        .bind(&session.provider)
        .bind(&session.model)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        tracing::debug!(session_id = %id, "sqlite: retrieving session");
        let row = sqlx::query(
            r#"
            SELECT id, name, provider, model, created_at, updated_at
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

            // Truncate preview to the configured character limit (char-safe).
            let preview = preview.map(|p| crate::text::truncate(&p, SESSION_PREVIEW_MAX_CHARS));

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
        tracing::debug!(session_id = %id, "sqlite: deleting session");
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
        tracing::debug!(session_id = %message.session_id, role = %message.role.as_str(), "sqlite: adding message");
        // Insert message
        sqlx::query(
            r#"
            INSERT INTO messages (id, session_id, role, content, tool_calls, tool_results, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.session_id.to_string())
        .bind(Self::role_to_string(message.role))
        .bind(&message.content)
        .bind(&message.tool_calls)
        .bind(&message.tool_results)
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
            SELECT id, session_id, role, content, tool_calls, tool_results, timestamp
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
                tool_calls: row.get("tool_calls"),
                tool_results: row.get("tool_results"),
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
/// URL resolution priority:
/// 1. `DATABASE_URL` environment variable (highest priority)
/// 2. `config_database_url` parameter (from config.toml `session.database_url`)
/// 3. Default: `sqlite:~/.config/synapse/sessions.db`
///
/// # Errors
///
/// Returns [`StorageError`] if storage creation fails.
pub async fn create_storage(
    config_database_url: Option<&str>,
) -> Result<Box<dyn SessionStore>, StorageError> {
    // Priority 1: DATABASE_URL environment variable
    let url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            // Priority 2: config.toml session.database_url
            match config_database_url {
                Some(url) => url.to_string(),
                // Priority 3: Default path
                None => {
                    let config_dir = dirs::home_dir()
                        .ok_or_else(|| {
                            StorageError::Database("could not determine home directory".to_string())
                        })?
                        .join(".config/synapse");

                    format!("sqlite:{}", config_dir.join("sessions.db").display())
                }
            }
        }
    };

    let store = SqliteStore::new(&url).await?;
    Ok(Box::new(store))
}

#[cfg(test)]
mod tests;
