//! Synapse Telegram Bot — Telegram interface for the Synapse AI agent.
//!
//! Connects the Telegram Bot API to `synapse-core`, sharing the same Agent,
//! SessionStore, and MCP subsystems as the CLI interface. Validates the
//! hexagonal architecture by proving a second frontend can reuse all core logic.

mod commands;
mod format;
mod handlers;
mod startup;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use handlers::ChatSessionMap;
use startup::{rebuild_chat_map, resolve_bot_token};
use synapse_core::config::Rotation;
use synapse_core::{Agent, Config, SessionStore, create_storage, init_mcp_client};
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::RwLock;
use tracing_subscriber::prelude::*;

/// Synapse Telegram Bot — AI agent Telegram interface
#[derive(Parser)]
#[command(name = "synapse-telegram", version)]
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

        // Map rotation enum to the tracing-appender rotation type.
        let rotation = match lc.rotation {
            Rotation::Daily => tracing_appender::rolling::Rotation::DAILY,
            Rotation::Hourly => tracing_appender::rolling::Rotation::HOURLY,
            Rotation::Never => tracing_appender::rolling::Rotation::NEVER,
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

    let storage: Arc<dyn SessionStore> = Arc::from(
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
    let initial_map = rebuild_chat_map(storage.as_ref()).await;
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
