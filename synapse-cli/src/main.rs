//! Synapse CLI - Command-line interface for the Synapse AI agent.

use std::io::{self, IsTerminal, Read};

use anyhow::{Context, Result};
use clap::Parser;

use synapse_core::{AnthropicProvider, Config, LlmProvider, Message, Role};

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

    // Validate API key is present
    let api_key = config
        .api_key
        .context("API key not configured. Add api_key to config.toml")?;

    // Create provider
    let provider = AnthropicProvider::new(api_key, &config.model);

    // Send request
    let messages = vec![Message::new(Role::User, message)];
    let response = provider
        .complete(&messages)
        .await
        .context("Failed to get response from Claude")?;

    println!("{}", response.content);
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
