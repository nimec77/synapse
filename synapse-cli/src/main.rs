//! Synapse CLI - Command-line interface for the Synapse AI agent.

use std::io::{self, IsTerminal, Read, Write};

use anyhow::{Context, Result};
use clap::Parser;
use futures::StreamExt;

use synapse_core::{Config, Message, Role, StreamEvent, create_provider};

/// Synapse CLI - AI agent command-line interface
#[derive(Parser)]
#[command(name = "synapse")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Message to send (reads from stdin if not provided)
    message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    let message = match get_message(&args) {
        Ok(msg) => msg,
        Err(_) => {
            // No input provided, show help
            Args::parse_from(["synapse", "--help"]);
            return Ok(());
        }
    };

    // Create provider using factory (handles API key lookup)
    let provider = create_provider(&config).context("Failed to create LLM provider")?;

    // Send request and stream response
    let messages = vec![Message::new(Role::User, message)];
    let stream = provider.stream(&messages);
    tokio::pin!(stream);

    let mut stdout = io::stdout();

    loop {
        tokio::select! {
            event = stream.next() => {
                match event {
                    Some(Ok(StreamEvent::TextDelta(text))) => {
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
    #[test]
    fn test_args_parse() {
        use super::Args;
        use clap::Parser;

        // Test with message argument
        let args = Args::parse_from(["synapse", "Hello"]);
        assert_eq!(args.message, Some("Hello".to_string()));

        // Test without message argument
        let args = Args::parse_from(["synapse"]);
        assert!(args.message.is_none());
    }
}
