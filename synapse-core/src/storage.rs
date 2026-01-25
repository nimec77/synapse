//! Storage abstraction for session persistence.
//!
//! Provides the [`SessionStore`] trait as a port for storage implementations,
//! along with error types and the SQLite adapter.

pub mod sqlite;

pub use sqlite::{SqliteStore, create_storage};

use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::config::SessionConfig;
use crate::session::{Session, SessionSummary, StoredMessage};

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A database operation failed.
    #[error("database error: {0}")]
    Database(String),

    /// The requested session was not found.
    #[error("session not found: {0}")]
    NotFound(Uuid),

    /// A migration operation failed.
    #[error("migration error: {0}")]
    Migration(String),

    /// Invalid data was encountered.
    #[error("invalid data: {0}")]
    InvalidData(String),
}

/// Result of a cleanup operation.
///
/// Tracks how many sessions were deleted and the reasons for deletion.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CleanupResult {
    /// Total number of sessions deleted.
    pub sessions_deleted: u32,
    /// Sessions deleted due to exceeding max_sessions limit.
    pub by_max_limit: u32,
    /// Sessions deleted due to exceeding retention_days.
    pub by_retention: u32,
}

/// Port for session storage implementations.
///
/// Provides an abstraction over different storage backends (SQLite, PostgreSQL, etc.)
/// following the hexagonal architecture pattern.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session in the store.
    ///
    /// # Arguments
    ///
    /// * `session` - The session to create
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the insert fails.
    async fn create_session(&self, session: &Session) -> Result<(), StorageError>;

    /// Get a session by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The session UUID to look up
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(session))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the query fails.
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError>;

    /// List all sessions with summary information.
    ///
    /// Returns sessions ordered by `updated_at` descending (most recent first).
    /// Includes message count and preview of first user message.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the query fails.
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError>;

    /// Update a session's `updated_at` timestamp to the current time.
    ///
    /// # Arguments
    ///
    /// * `id` - The session UUID to touch
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::NotFound`] if the session doesn't exist.
    /// Returns [`StorageError::Database`] if the update fails.
    async fn touch_session(&self, id: Uuid) -> Result<(), StorageError>;

    /// Delete a session and all its messages.
    ///
    /// # Arguments
    ///
    /// * `id` - The session UUID to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the session was deleted, `Ok(false)` if not found.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the delete fails.
    async fn delete_session(&self, id: Uuid) -> Result<bool, StorageError>;

    /// Add a message to a session.
    ///
    /// Also updates the session's `updated_at` timestamp.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to add
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the insert fails.
    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError>;

    /// Get all messages for a session.
    ///
    /// Returns messages ordered by `timestamp` ascending (oldest first).
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session UUID to get messages for
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if the query fails.
    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError>;

    /// Run cleanup based on configuration.
    ///
    /// Deletes sessions that exceed the `max_sessions` limit (oldest first)
    /// and sessions older than `retention_days`.
    ///
    /// # Arguments
    ///
    /// * `config` - Session configuration with cleanup parameters
    ///
    /// # Returns
    ///
    /// Returns a [`CleanupResult`] with counts of deleted sessions.
    ///
    /// # Errors
    ///
    /// Returns [`StorageError::Database`] if cleanup operations fail.
    async fn cleanup(&self, config: &SessionConfig) -> Result<CleanupResult, StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_display() {
        let db_err = StorageError::Database("connection failed".to_string());
        assert_eq!(db_err.to_string(), "database error: connection failed");

        let id = Uuid::new_v4();
        let not_found = StorageError::NotFound(id);
        assert_eq!(not_found.to_string(), format!("session not found: {}", id));

        let migration_err = StorageError::Migration("version mismatch".to_string());
        assert_eq!(
            migration_err.to_string(),
            "migration error: version mismatch"
        );

        let invalid_err = StorageError::InvalidData("corrupt record".to_string());
        assert_eq!(invalid_err.to_string(), "invalid data: corrupt record");
    }

    #[test]
    fn test_cleanup_result_default() {
        let result = CleanupResult::default();
        assert_eq!(result.sessions_deleted, 0);
        assert_eq!(result.by_max_limit, 0);
        assert_eq!(result.by_retention, 0);
    }

    #[test]
    fn test_cleanup_result_construction() {
        let result = CleanupResult {
            sessions_deleted: 5,
            by_max_limit: 3,
            by_retention: 2,
        };
        assert_eq!(result.sessions_deleted, 5);
        assert_eq!(result.by_max_limit, 3);
        assert_eq!(result.by_retention, 2);
    }

    #[test]
    fn test_cleanup_result_clone() {
        let original = CleanupResult {
            sessions_deleted: 10,
            by_max_limit: 6,
            by_retention: 4,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
