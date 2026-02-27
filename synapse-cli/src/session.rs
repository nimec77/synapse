//! CLI session helpers: storage initialisation and session load/create.
//!
//! Extracted shared logic that was previously duplicated between the REPL
//! path and the one-shot path in `main.rs`.

use anyhow::{Context, Result};
use uuid::Uuid;

use synapse_core::config::SessionConfig;
use synapse_core::{Config, Session, SessionStore, StoredMessage, create_storage};

/// Create storage and run auto-cleanup if configured.
///
/// Reads the database URL from `session_config` and delegates construction to
/// [`create_storage`]. If `auto_cleanup` is enabled, triggers a cleanup pass
/// (failures are intentionally ignored to avoid aborting the main operation).
pub async fn init_storage(session_config: &SessionConfig) -> Result<Box<dyn SessionStore>> {
    let storage = create_storage(session_config.database_url.as_deref())
        .await
        .context("Failed to create storage")?;

    if session_config.auto_cleanup {
        let _ = storage.cleanup(session_config).await;
    }

    Ok(storage)
}

/// Load an existing session by ID, or create a new session.
///
/// If `session_id` is `Some`, retrieves the session and its message history
/// from storage. If `session_id` is `None`, creates a new session, persists
/// it, and returns an empty history.
///
/// Returns a tuple of `(Session, Vec<StoredMessage>)`.
pub async fn load_or_create_session(
    storage: &dyn SessionStore,
    config: &Config,
    session_id: Option<Uuid>,
) -> Result<(Session, Vec<StoredMessage>)> {
    if let Some(id) = session_id {
        let session = storage
            .get_session(id)
            .await
            .context("Failed to get session")?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;

        let messages = storage
            .get_messages(id)
            .await
            .context("Failed to get messages")?;

        Ok((session, messages))
    } else {
        let session = Session::new(&config.provider, &config.model);
        storage
            .create_session(&session)
            .await
            .context("Failed to create session")?;
        Ok((session, Vec::new()))
    }
}
