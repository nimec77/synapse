//! Synapse CLI - Command-line interface for the Synapse AI agent.

use std::io::{self, IsTerminal, Read};

use clap::Parser;

use synapse_core::Config;

/// Synapse CLI - AI agent command-line interface
#[derive(Parser)]
#[command(name = "synapse")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Message to send (reads from stdin if not provided)
    message: Option<String>,
}

fn main() {
    let args = Args::parse();
    let config = Config::load().unwrap_or_default();

    match get_message(&args) {
        Ok(message) => {
            println!("Provider: {}", config.provider);
            println!("{}", format_echo(&message));
        }
        Err(_) => {
            // No input provided, show help
            Args::parse_from(["synapse", "--help"]);
        }
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

/// Formats the echo output with the "Echo: " prefix.
fn format_echo(message: &str) -> String {
    format!("Echo: {}", message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_echo_simple() {
        assert_eq!(format_echo("hello"), "Echo: hello");
    }

    #[test]
    fn test_format_echo_with_spaces() {
        assert_eq!(format_echo("Hello, world!"), "Echo: Hello, world!");
    }

    #[test]
    fn test_format_echo_multiline() {
        assert_eq!(format_echo("Line 1\nLine 2"), "Echo: Line 1\nLine 2");
    }

    #[test]
    fn test_format_echo_empty() {
        assert_eq!(format_echo(""), "Echo: ");
    }
}
