# Plan: SY-3 - Echo CLI

**Status: PLAN_APPROVED**

---

## Overview

This plan implements the Echo CLI feature for Synapse, establishing the CLI input/output foundation. The implementation adds `clap` for argument parsing and supports two input modes: one-shot (positional argument) and stdin (piped input). The output format is `Echo: <message>`.

---

## Components

### Files to Modify

| File | Change |
|------|--------|
| `synapse-cli/Cargo.toml` | Add clap dependency with derive feature |
| `synapse-cli/src/main.rs` | Replace placeholder with echo implementation |

### No New Files

The implementation is simple enough to remain in `main.rs`. Module extraction (args.rs, etc.) is deferred per research recommendation.

---

## Implementation Details

### 1. Cargo.toml Update

```toml
[package]
name = "synapse-cli"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "synapse"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
```

### 2. Args Struct

```rust
use clap::Parser;

/// Synapse CLI - AI agent command-line interface
#[derive(Parser)]
#[command(name = "synapse")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Message to send (reads from stdin if not provided)
    message: Option<String>,
}
```

Key attributes:
- `#[command(name = "synapse")]` - Sets the binary name in help text
- `#[command(author, version, about)]` - Auto-generates from Cargo.toml
- `message: Option<String>` - Optional positional argument

### 3. Main Function Logic

```rust
use std::io::{self, IsTerminal, Read};

use clap::Parser;

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

    match get_message(&args) {
        Ok(message) => println!("{}", format_echo(&message)),
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
```

### 4. Unit Tests

```rust
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
```

---

## Data Flow

```
                    +----------------+
                    |   User Input   |
                    +----------------+
                           |
           +---------------+---------------+
           |                               |
           v                               v
   +---------------+               +---------------+
   | Positional    |               | Stdin (pipe)  |
   | Argument      |               | Input         |
   +---------------+               +---------------+
           |                               |
           +---------------+---------------+
                           |
                           v
                    +----------------+
                    | get_message()  |
                    | - Check args   |
                    | - Check stdin  |
                    | - TTY detect   |
                    +----------------+
                           |
           +---------------+---------------+
           |                               |
           v                               v
   +---------------+               +---------------+
   | Message found |               | No message    |
   | format_echo() |               | Show --help   |
   +---------------+               +---------------+
           |                               |
           v                               v
   +---------------+               +---------------+
   | println!      |               | clap help     |
   | "Echo: ..."   |               | output        |
   +---------------+               +---------------+
```

### Input Priority

1. **Positional argument**: `synapse "hello"` - Uses "hello"
2. **Stdin piped**: `echo "hello" | synapse` - Reads stdin
3. **TTY, no argument**: `synapse` - Shows help message

---

## NFR (Non-Functional Requirements)

### Performance

- **Memory**: Reads entire stdin into memory. Acceptable for echo mode; future LLM phases will address large input handling.
- **Startup**: Minimal overhead from clap argument parsing (sub-millisecond).

### Maintainability

- Single file implementation for Phase 2 simplicity.
- Clear separation: `get_message()` for input, `format_echo()` for output.
- Functions are testable in isolation.

### Code Quality

- Doc comments on all public items per conventions.
- Import grouping: std -> external -> internal.
- No `unwrap()` or `expect()` - uses Result propagation.

---

## Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| clap compatibility with Edition 2024 | Low | Medium | clap 4.x tested with nightly; pin version |
| Stdin blocking indefinitely | Low | Low | TTY detection prevents blocking on interactive terminal |
| Large stdin memory usage | Low | Low | Acceptable for echo; document for future consideration |

---

## Verification Checklist

### Build and Lint

```bash
cargo build -p synapse-cli
cargo clippy -p synapse-cli -- -D warnings
cargo fmt --check
cargo test -p synapse-cli
```

### Manual Acceptance Tests

```bash
# Test 1: One-shot mode
synapse "hello"
# Expected: Echo: hello

# Test 2: Stdin mode
echo "hello" | synapse
# Expected: Echo: hello

# Test 3: Multiline stdin
echo -e "Line 1\nLine 2" | synapse
# Expected: Echo: Line 1
#           Line 2

# Test 4: No input (should show help)
synapse
# Expected: Help message

# Test 5: Version flag
synapse --version
# Expected: synapse 0.1.0

# Test 6: Help flag
synapse --help
# Expected: Usage information
```

---

## Implementation Order

1. Update `synapse-cli/Cargo.toml` - Add clap dependency
2. Update `synapse-cli/src/main.rs` - Replace with full implementation
3. Run `cargo build -p synapse-cli` - Verify compilation
4. Run `cargo test -p synapse-cli` - Verify unit tests pass
5. Run `cargo clippy -p synapse-cli -- -D warnings` - Verify no warnings
6. Run `cargo fmt` - Ensure formatting
7. Manual acceptance testing - All 6 test scenarios

---

## Alternatives Considered

### 1. atty crate vs std::io::IsTerminal

**Chosen**: `std::io::IsTerminal` (standard library)

**Rationale**: Avoids external dependency. `IsTerminal` is stable since Rust 1.70 and provides the same functionality.

### 2. Separate args.rs module vs inline in main.rs

**Chosen**: Keep in `main.rs`

**Rationale**: Implementation is minimal (~50 lines). Module extraction adds complexity without benefit. Will extract when CLI grows (REPL, multiple subcommands).

### 3. anyhow vs std::io::Result

**Chosen**: `std::io::Result`

**Rationale**: Error handling is minimal for SY-3 (only stdin read errors). Adding `anyhow` deferred to when error chains become more complex.
