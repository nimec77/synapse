# SY-3 Summary: Echo CLI

**Status:** Complete
**Date:** 2026-01-11

---

## Overview

SY-3 implements the Echo CLI feature for Synapse, establishing the CLI input/output foundation. This phase adds command-line argument parsing using the `clap` crate and supports two input modes: one-shot (positional argument) and stdin (piped input). The CLI echoes back any input with an "Echo: " prefix.

This is a foundational step that validates the CLI can receive input through multiple methods before integrating with LLM providers in later phases.

---

## What Was Implemented

### Input Modes

1. **One-shot mode**: Pass a message as a positional argument
   ```bash
   synapse "Hello, world!"
   # Output: Echo: Hello, world!
   ```

2. **Stdin mode**: Pipe text into synapse
   ```bash
   echo "Hello from stdin" | synapse
   # Output: Echo: Hello from stdin
   ```

3. **TTY detection**: When no input is provided and stdin is an interactive terminal, the CLI displays help instead of waiting indefinitely

### Built-in Flags

- `--help` / `-h`: Display usage information
- `--version` / `-V`: Display version (synapse 0.1.0)

### Input Priority

When both a positional argument and piped input are provided, the positional argument takes precedence:
```bash
echo "stdin" | synapse "arg"
# Output: Echo: arg
```

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| TTY detection library | `std::io::IsTerminal` | Standard library (stable since Rust 1.70), avoids external dependency |
| Module structure | Keep in `main.rs` | Implementation is minimal (~80 lines); extract when CLI grows |
| Error handling | `std::io::Result` | Minimal error handling needed for echo; anyhow deferred |
| Empty input behavior | Show help message | User-friendly behavior when no arguments and stdin is a TTY |

---

## Files Modified

| File | Change |
|------|--------|
| `synapse-cli/Cargo.toml` | Added `clap = { version = "4", features = ["derive"] }` dependency |
| `synapse-cli/src/main.rs` | Replaced placeholder with full echo CLI implementation |

---

## Code Structure

The implementation in `synapse-cli/src/main.rs` consists of:

- **Args struct**: Clap-derived argument parser with optional `message` field
- **get_message()**: Retrieves input from positional argument or stdin with TTY detection
- **format_echo()**: Formats output with "Echo: " prefix
- **main()**: Orchestrates parsing and output
- **tests module**: 4 unit tests for `format_echo()` function

---

## Usage Examples

### Basic Usage

```bash
# One-shot mode
synapse "hello"
# Output: Echo: hello

# Stdin mode
echo "hello" | synapse
# Output: Echo: hello

# Multiline stdin
echo -e "Line 1\nLine 2" | synapse
# Output: Echo: Line 1
#         Line 2

# Help
synapse --help

# Version
synapse --version
```

### Using cargo run

During development, use `cargo run` with `--` to separate cargo arguments from synapse arguments:

```bash
cargo run -p synapse-cli -- "hello"
echo "hello" | cargo run -p synapse-cli
cargo run -p synapse-cli -- --help
```

---

## Testing

### Unit Tests

4 unit tests verify `format_echo()` behavior:
- Simple message: `"hello"` -> `"Echo: hello"`
- Message with spaces: `"Hello, world!"` -> `"Echo: Hello, world!"`
- Multiline: `"Line 1\nLine 2"` -> `"Echo: Line 1\nLine 2"`
- Empty: `""` -> `"Echo: "`

Run tests with:
```bash
cargo test -p synapse-cli
```

### Manual Acceptance Tests

All 6 scenarios from the implementation plan were verified:
1. One-shot mode with simple message
2. Stdin mode with piped input
3. Multiline stdin input
4. No input shows help
5. `--version` flag
6. `--help` flag

---

## Limitations

1. **No unit tests for get_message()**: The input retrieval function requires stdin mocking and TTY simulation, which was deferred
2. **Memory usage**: Reads entire stdin into memory before processing; acceptable for echo mode but will need consideration for large inputs in future phases
3. **Single-file structure**: All code remains in `main.rs`; module extraction will occur when the CLI grows

---

## Future Enhancements

The Echo CLI establishes patterns that will be extended in future phases:

1. **SY-4+**: Replace echo output with actual LLM API calls
2. **REPL mode**: Interactive terminal mode with `ratatui`
3. **Configuration**: TOML-based configuration for API keys and settings
4. **Subcommands**: Add commands for different operations (chat, config, etc.)

---

## Quality Metrics

| Check | Status |
|-------|--------|
| `cargo build -p synapse-cli` | Pass |
| `cargo clippy -p synapse-cli -- -D warnings` | Pass |
| `cargo fmt --check` | Pass |
| `cargo test -p synapse-cli` | Pass (4 tests) |
