//! Synapse CLI - Command-line interface for the Synapse AI agent.

use std::io::{self, IsTerminal, Read, Write};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use uuid::Uuid;

use synapse_core::{
    Config, Message, Role, Session, StoredMessage, StreamEvent, create_provider, create_storage,
};

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

    /// Session management commands
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Session management commands
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List,
    /// Show messages in a session
    Show {
        /// Session ID to show
        id: Uuid,
    },
    /// Delete a session
    Delete {
        /// Session ID to delete
        id: Uuid,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    // Handle subcommands
    if let Some(command) = args.command {
        return handle_command(command).await;
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
    let storage = create_storage(session_config.database_url.as_deref())
        .await
        .context("Failed to create storage")?;

    // Run auto-cleanup if enabled
    if session_config.auto_cleanup {
        let _ = storage.cleanup(&session_config).await;
    }

    // Load or create session
    let (session, history) = if let Some(session_id) = args.session {
        // Continue existing session
        let session = storage
            .get_session(session_id)
            .await
            .context("Failed to get session")?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        let messages = storage
            .get_messages(session_id)
            .await
            .context("Failed to get messages")?;

        (session, messages)
    } else {
        // Create new session
        let session = Session::new(&config.provider, &config.model);
        storage
            .create_session(&session)
            .await
            .context("Failed to create session")?;
        (session, Vec::new())
    };

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

    // Create provider and stream response
    let provider = create_provider(&config).context("Failed to create LLM provider")?;
    let stream = provider.stream(&messages);
    tokio::pin!(stream);

    let mut stdout = io::stdout();
    let mut response_content = String::new();

    loop {
        tokio::select! {
            event = stream.next() => {
                match event {
                    Some(Ok(StreamEvent::TextDelta(text))) => {
                        response_content.push_str(&text);
                        print!("{}", text);
                        stdout.flush().context("Failed to flush stdout")?;
                    }
                    Some(Ok(StreamEvent::Done)) | None => {
                        println!(); // Final newline
                        break;
                    }
                    Some(Ok(StreamEvent::Error(e))) => {
                        return Err(e).context("Streaming error");
                    }
                    Some(Ok(_)) => {
                        // Ignore ToolCall/ToolResult for now
                    }
                    Some(Err(e)) => {
                        return Err(e).context("Stream error");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n[Interrupted]");
                break;
            }
        }
    }

    // Store assistant response
    if !response_content.is_empty() {
        let assistant_msg = StoredMessage::new(session.id, Role::Assistant, &response_content);
        storage
            .add_message(&assistant_msg)
            .await
            .context("Failed to store assistant message")?;
    }

    Ok(())
}

/// Handle session management subcommands.
async fn handle_command(command: Commands) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let session_config = config.session.unwrap_or_default();
    let storage = create_storage(session_config.database_url.as_deref())
        .await
        .context("Failed to create storage")?;

    match command {
        Commands::Sessions { action } => match action {
            SessionAction::List => {
                let sessions = storage
                    .list_sessions()
                    .await
                    .context("Failed to list sessions")?;

                if sessions.is_empty() {
                    println!("No sessions found.");
                    return Ok(());
                }

                // Print header
                println!(
                    "{:<36}  {:<15}  {:<15}  {:<10}  PREVIEW",
                    "ID", "PROVIDER", "MODEL", "MESSAGES"
                );
                println!("{:-<100}", "");

                // Print sessions
                for session in sessions {
                    let preview = session.preview.as_deref().unwrap_or("-").replace('\n', " ");
                    println!(
                        "{:<36}  {:<15}  {:<15}  {:<10}  {}",
                        session.id,
                        truncate(&session.provider, 15),
                        truncate(&session.model, 15),
                        session.message_count,
                        truncate(&preview, 40)
                    );
                }
            }
            SessionAction::Show { id } => {
                let session = storage
                    .get_session(id)
                    .await
                    .context("Failed to get session")?;

                let Some(session) = session else {
                    bail!("Session not found: {}", id);
                };

                let messages = storage
                    .get_messages(id)
                    .await
                    .context("Failed to get messages")?;

                // Print session info
                println!("Session: {}", session.id);
                println!("Provider: {}", session.provider);
                println!("Model: {}", session.model);
                println!(
                    "Created: {}",
                    session.created_at.format("%Y-%m-%d %H:%M:%S")
                );
                println!();

                if messages.is_empty() {
                    println!("No messages in this session.");
                    return Ok(());
                }

                // Print messages
                for msg in messages {
                    let role_label = match msg.role {
                        Role::System => "[SYSTEM]",
                        Role::User => "[USER]",
                        Role::Assistant => "[ASSISTANT]",
                    };
                    println!("{}", role_label);
                    println!("{}", msg.content);
                    println!();
                }
            }
            SessionAction::Delete { id } => {
                let deleted = storage
                    .delete_session(id)
                    .await
                    .context("Failed to delete session")?;

                if deleted {
                    println!("Session {} deleted.", id);
                } else {
                    bail!("Session not found: {}", id);
                }
            }
        },
    }

    Ok(())
}

/// Truncate a string to a maximum length, adding "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
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
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hi", 2), "hi");
        assert_eq!(truncate("hello", 3), "...");
    }
}
