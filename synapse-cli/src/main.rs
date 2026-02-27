//! Synapse CLI - Command-line interface for the Synapse AI agent.

mod commands;
mod repl;
mod session;

use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;
use uuid::Uuid;

use commands::{Commands, handle_command};
use synapse_core::{Agent, Config, Message, Role, StoredMessage, StreamEvent, init_mcp_client};

/// Synapse CLI - AI agent command-line interface
#[derive(Parser)]
#[command(name = "synapse")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Message to send (reads from stdin if not provided)
    message: Option<String>,

    /// Continue an existing session by ID
    #[arg(short, long)]
    session: Option<Uuid>,

    /// Enter interactive REPL mode
    #[arg(short, long)]
    repl: bool,

    /// Override the LLM provider from config
    #[arg(short = 'p', long)]
    provider: Option<String>,

    /// Path to a custom config file (overrides default search locations)
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    /// Session management commands
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let mut config = Config::load(args.config.as_deref())?;

    // Apply provider override from CLI flag
    if let Some(ref provider) = args.provider {
        config.provider = provider.clone();
    }

    // Handle subcommands
    if let Some(command) = args.command {
        return handle_command(command, args.config.as_deref()).await;
    }

    // Handle REPL mode
    if args.repl {
        let session_config = config.session.clone().unwrap_or_default();
        let storage = session::init_storage(&session_config).await?;
        let (session, history) =
            session::load_or_create_session(storage.as_ref(), &config, args.session).await?;

        let mcp_path = config.mcp.as_ref().and_then(|m| m.config_path.as_deref());
        let mcp_client = init_mcp_client(mcp_path).await;
        return repl::run_repl(&config, storage, session, history, mcp_client).await;
    }

    // Get message or show help
    let message = match get_message(&args) {
        Ok(msg) => msg,
        Err(_) => {
            // No input provided, show help
            Args::parse_from(["synapse", "--help"]);
            return Ok(());
        }
    };

    // Create storage with config database_url
    let session_config = config.session.clone().unwrap_or_default();
    let storage = session::init_storage(&session_config).await?;

    // Load or create session
    let (session, history) =
        session::load_or_create_session(storage.as_ref(), &config, args.session).await?;

    // Build conversation history
    let mut messages: Vec<Message> = history
        .iter()
        .map(|m| Message::new(m.role, &m.content))
        .collect();

    // Add new user message
    messages.push(Message::new(Role::User, &message));

    // Store user message
    let user_msg = StoredMessage::new(session.id, Role::User, &message);
    storage
        .add_message(&user_msg)
        .await
        .context("Failed to store user message")?;

    // Create agent from config and MCP client
    let mcp_path = config.mcp.as_ref().and_then(|m| m.config_path.as_deref());
    let mcp_client = init_mcp_client(mcp_path).await;
    let agent = Agent::from_config(&config, mcp_client).context("Failed to create agent")?;

    // Stream response via agent (scoped to release borrows before shutdown)
    let response_content = {
        let stream = agent.stream(&mut messages);
        tokio::pin!(stream);

        let mut stdout = io::stdout();
        let mut content = String::new();

        loop {
            tokio::select! {
                event = stream.next() => {
                    match event {
                        Some(Ok(StreamEvent::TextDelta(text))) => {
                            content.push_str(&text);
                            print!("{}", text);
                            stdout.flush().context("Failed to flush stdout")?;
                        }
                        Some(Ok(StreamEvent::Done)) | None => {
                            println!(); // Final newline
                            break;
                        }
                        Some(Err(e)) => {
                            return Err(e).context("Agent error");
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\n[Interrupted]");
                    break;
                }
            }
        }

        content
    };

    // Store assistant response
    if !response_content.is_empty() {
        let assistant_msg = StoredMessage::new(session.id, Role::Assistant, &response_content);
        storage
            .add_message(&assistant_msg)
            .await
            .context("Failed to store assistant message")?;
    }

    // Shutdown agent (MCP connections)
    agent.shutdown().await;

    Ok(())
}

/// Retrieves the message from arguments or stdin.
///
/// Priority: positional argument > stdin > error (if TTY)
fn get_message(args: &Args) -> io::Result<String> {
    // Priority 1: Use positional argument if provided
    if let Some(msg) = &args.message {
        return Ok(msg.clone());
    }

    // Priority 2: Check if stdin has piped input
    if io::stdin().is_terminal() {
        // Interactive terminal with no argument - signal to show help
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No message provided",
        ));
    }

    // Read from stdin (piped input)
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parse() {
        // Test with message argument
        let args = Args::parse_from(["synapse", "Hello"]);
        assert_eq!(args.message, Some("Hello".to_string()));
        assert!(args.session.is_none());

        // Test without message argument
        let args = Args::parse_from(["synapse"]);
        assert!(args.message.is_none());
    }

    #[test]
    fn test_args_with_session() {
        let id = Uuid::new_v4();
        let args = Args::parse_from(["synapse", "--session", &id.to_string(), "Hello"]);
        assert_eq!(args.session, Some(id));
        assert_eq!(args.message, Some("Hello".to_string()));
    }

    #[test]
    fn test_args_session_short_flag() {
        let id = Uuid::new_v4();
        let args = Args::parse_from(["synapse", "-s", &id.to_string(), "Hello"]);
        assert_eq!(args.session, Some(id));
    }

    #[test]
    fn test_args_repl_flag() {
        let args = Args::parse_from(["synapse", "--repl"]);
        assert!(args.repl);
        assert!(args.message.is_none());
        assert!(args.session.is_none());
    }

    #[test]
    fn test_args_repl_short_flag() {
        let args = Args::parse_from(["synapse", "-r"]);
        assert!(args.repl);
    }

    #[test]
    fn test_args_repl_with_session() {
        let id = Uuid::new_v4();
        let args = Args::parse_from(["synapse", "--repl", "--session", &id.to_string()]);
        assert!(args.repl);
        assert_eq!(args.session, Some(id));
    }

    #[test]
    fn test_args_repl_default_false() {
        let args = Args::parse_from(["synapse", "Hello"]);
        assert!(!args.repl);
    }

    #[test]
    fn test_args_with_provider_flag() {
        let args = Args::parse_from(["synapse", "-p", "openai", "Hello"]);
        assert_eq!(args.provider, Some("openai".to_string()));
        assert_eq!(args.message, Some("Hello".to_string()));
    }

    #[test]
    fn test_args_with_provider_long_flag() {
        let args = Args::parse_from(["synapse", "--provider", "openai", "Hello"]);
        assert_eq!(args.provider, Some("openai".to_string()));
        assert_eq!(args.message, Some("Hello".to_string()));
    }

    #[test]
    fn test_args_provider_with_repl() {
        let args = Args::parse_from(["synapse", "-p", "openai", "--repl"]);
        assert_eq!(args.provider, Some("openai".to_string()));
        assert!(args.repl);
    }

    #[test]
    fn test_args_provider_default_none() {
        let args = Args::parse_from(["synapse", "Hello"]);
        assert!(args.provider.is_none());
    }
}
