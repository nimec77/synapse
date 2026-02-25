//! Synapse Telegram Bot — Telegram interface for the Synapse AI agent.
//!
//! Connects the Telegram Bot API to `synapse-core`, sharing the same Agent,
//! SessionStore, and MCP subsystems as the CLI interface. Validates the
//! hexagonal architecture by proving a second frontend can reuse all core logic.

mod commands;
mod format;
mod handlers;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use handlers::{ChatSessionMap, ChatSessions};
use synapse_core::{Agent, Config, SessionStore, SessionSummary, create_storage, init_mcp_client};
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::RwLock;
use tracing_subscriber::prelude::*;

/// Synapse Telegram Bot — AI agent Telegram interface
#[derive(Parser)]
#[command(name = "synapse-telegram")]
struct Args {
    /// Path to a custom config file (overrides default search locations)
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,
}

/// Default tracing directives enabling info-level logs for this crate and synapse-core.
const DEFAULT_DIRECTIVES: &[&str] = &["synapse_telegram=info", "synapse_core=info"];

/// Build the default `EnvFilter`: RUST_LOG (if set) plus our default directives.
fn default_env_filter() -> anyhow::Result<tracing_subscriber::EnvFilter> {
    let mut filter = tracing_subscriber::EnvFilter::from_default_env();
    for directive in DEFAULT_DIRECTIVES {
        filter = filter.add_directive(directive.parse()?);
    }
    Ok(filter)
}

/// Initialize the tracing subscriber.
///
/// When `config.logging` is `Some`, creates a layered subscriber with both
/// stdout and rolling file output. When `None`, uses stdout-only (current behavior).
///
/// Returns the non-blocking writer guard that must be held for the process lifetime.
fn init_tracing(
    config: &Config,
) -> anyhow::Result<Option<tracing_appender::non_blocking::WorkerGuard>> {
    if let Some(ref lc) = config.logging {
        // Attempt to create the log directory; fall back to stdout-only on failure.
        if let Err(e) = std::fs::create_dir_all(&lc.directory) {
            eprintln!(
                "Warning: Failed to create log directory '{}': {}. Falling back to stdout-only.",
                lc.directory, e
            );
            tracing_subscriber::fmt()
                .with_env_filter(default_env_filter()?)
                .init();
            return Ok(None);
        }

        // Map rotation string to the tracing-appender rotation type.
        let rotation = match lc.rotation.as_str() {
            "daily" => tracing_appender::rolling::Rotation::DAILY,
            "hourly" => tracing_appender::rolling::Rotation::HOURLY,
            "never" => tracing_appender::rolling::Rotation::NEVER,
            other => {
                eprintln!(
                    "Warning: Unknown rotation '{}', falling back to daily",
                    other
                );
                tracing_appender::rolling::Rotation::DAILY
            }
        };

        // Build the rolling file appender.
        let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
            .rotation(rotation)
            .filename_prefix("synapse-telegram")
            .filename_suffix("log")
            .max_log_files(lc.max_files)
            .build(&lc.directory)
            .context("Failed to create rolling file appender")?;

        // Wrap in a non-blocking writer; guard must be kept alive for the process lifetime.
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let env_filter = default_env_filter()?;

        let stdout_layer = tracing_subscriber::fmt::layer();

        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .with(file_layer)
            .init();

        Ok(Some(guard))
    } else {
        // No [logging] section — preserve current stdout-only behavior exactly.
        tracing_subscriber::fmt()
            .with_env_filter(default_env_filter()?)
            .init();
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // 1. Load application configuration FIRST (tracing init depends on config).
    let config = Config::load(args.config.as_deref()).context("Failed to load config")?;

    // 2. Initialize tracing (stdout-only or stdout+file based on config).
    let _guard = init_tracing(&config)?;

    tracing::info!("Starting Synapse Telegram Bot");

    // 3. Resolve bot token (env var > config file). Token is never logged.
    let token = resolve_bot_token(&config).context("Failed to obtain bot token")?;

    // 4. Create the teloxide Bot instance.
    let bot = Bot::new(token);

    // 5. Create storage and run auto-cleanup.
    let db_url = config
        .session
        .as_ref()
        .and_then(|s| s.database_url.as_deref());

    let storage: Arc<Box<dyn SessionStore>> = Arc::new(
        create_storage(db_url)
            .await
            .context("Failed to initialize session storage")?,
    );

    if config
        .session
        .as_ref()
        .map(|s| s.auto_cleanup)
        .unwrap_or(true)
    {
        let session_config = config.session.clone().unwrap_or_default();
        match storage.cleanup(&session_config).await {
            Ok(result) => {
                if result.sessions_deleted > 0 {
                    tracing::info!("Cleaned up {} old sessions", result.sessions_deleted);
                }
            }
            Err(e) => tracing::warn!("Session cleanup failed: {}", e),
        }
    }

    // 6. Initialize MCP client.
    let mcp_path = config.mcp.as_ref().and_then(|m| m.config_path.as_deref());
    let mcp_client = init_mcp_client(mcp_path).await;

    // 7. Create Agent from config and wrap in Arc.
    let agent =
        Arc::new(Agent::from_config(&config, mcp_client).context("Failed to create agent")?);

    // 9. Rebuild chat-to-session map from persisted sessions.
    let initial_map = rebuild_chat_map(storage.as_ref().as_ref()).await;
    let total_sessions: usize = initial_map.values().map(|cs| cs.sessions.len()).sum();
    tracing::info!(
        "Restored {} Telegram sessions across {} chats from storage",
        total_sessions,
        initial_map.len()
    );
    let chat_map: ChatSessionMap = Arc::new(RwLock::new(initial_map));

    // 10. Wrap shared config in Arc for handler injection.
    let config = Arc::new(config);

    // 11. Fetch the bot's own identity (required for filter_command parsing).
    let me = bot.get_me().await.context("Failed to fetch bot identity")?;

    // 12. Register slash commands with Telegram (for autocomplete UI). Non-fatal on failure.
    if let Err(e) = bot.set_my_commands(commands::Command::bot_commands()).await {
        tracing::warn!("Failed to register bot commands: {}", e);
    }

    // 13. Set up branched handler: commands and callback queries route separately from messages.
    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter_command::<commands::Command>()
                        .endpoint(commands::handle_command),
                )
                .branch(dptree::entry().endpoint(handlers::handle_message)),
        )
        .branch(Update::filter_callback_query().endpoint(commands::handle_callback));

    tracing::info!("Dispatcher ready — polling for updates");

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![
            me,
            Arc::clone(&config),
            Arc::clone(&agent),
            Arc::clone(&storage),
            chat_map
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    // 12. Graceful shutdown: release MCP connections if possible.
    tracing::info!("Dispatcher stopped — shutting down");
    if let Ok(a) = Arc::try_unwrap(agent) {
        a.shutdown().await;
        tracing::info!("Agent shutdown complete");
    } else {
        tracing::warn!("Could not unwrap Agent for graceful shutdown — connections will drop");
    }

    Ok(())
}

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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use synapse_core::config::SessionConfig;
    use synapse_core::session::{Session, StoredMessage};
    use synapse_core::storage::{CleanupResult, SessionStore, StorageError};
    use synapse_core::{Config, TelegramConfig};
    use uuid::Uuid;

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

        async fn get_messages(
            &self,
            _session_id: Uuid,
        ) -> Result<Vec<StoredMessage>, StorageError> {
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
}
