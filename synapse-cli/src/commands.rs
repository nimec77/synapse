//! Session management subcommands for the Synapse CLI.
//!
//! Defines the [`Commands`] and [`SessionAction`] enums parsed by `clap`,
//! and the [`handle_command`] dispatcher that executes session list, show,
//! and delete operations.

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use uuid::Uuid;

use synapse_core::{Config, Role, create_storage};

/// Top-level subcommands for the `synapse` binary.
#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Session management commands
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
}

/// Session management actions.
#[derive(Subcommand)]
pub(crate) enum SessionAction {
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

/// Handle session management subcommands.
pub(crate) async fn handle_command(command: Commands) -> Result<()> {
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
                        Role::Tool => "[TOOL]",
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
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hi", 2), "hi");
        assert_eq!(truncate("hello", 3), "...");
    }
}
