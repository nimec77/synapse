# Tasklist: SY-3 - Echo CLI

Status: TASKLIST_READY

## Context

Implement the Echo CLI feature for Synapse, establishing the CLI input/output foundation. The implementation adds `clap` for argument parsing and supports two input modes: one-shot (positional argument) and stdin (piped input). The output format is `Echo: <message>`.

This is a foundational step that validates the CLI can receive input through multiple methods before integrating with LLM providers in later phases.

---

## Tasks

### 3.1 Add clap dependency to Cargo.toml

- [x] Update `synapse-cli/Cargo.toml` to add `clap` with derive feature

**Acceptance Criteria:**
- File `synapse-cli/Cargo.toml` contains `clap = { version = "4", features = ["derive"] }` in `[dependencies]`
- Running `cargo check -p synapse-cli` succeeds without errors

---

### 3.2 Implement Args struct with clap derive

- [x] Add `use clap::Parser;` import to `main.rs`
- [x] Create `Args` struct with `#[derive(Parser)]` and `message: Option<String>` field
- [x] Add appropriate `#[command(...)]` attributes for name, author, version, about

**Acceptance Criteria:**
- `Args` struct exists in `synapse-cli/src/main.rs`
- Struct has `#[derive(Parser)]` attribute
- Struct has `message: Option<String>` field with doc comment
- Running `cargo build -p synapse-cli` succeeds

---

### 3.3 Implement get_message() function

- [x] Add `use std::io::{self, IsTerminal, Read};` imports
- [x] Implement `get_message(args: &Args) -> io::Result<String>` function
- [x] Handle priority: positional argument > stdin > error (if TTY)

**Acceptance Criteria:**
- Function `get_message()` exists and takes `&Args` parameter
- Returns `Ok(message)` when positional argument is provided
- Returns `Ok(stdin_content)` when stdin has piped input
- Returns `Err` when no argument and stdin is a terminal

---

### 3.4 Implement format_echo() function

- [x] Implement `format_echo(message: &str) -> String` function
- [x] Return message prefixed with "Echo: "

**Acceptance Criteria:**
- Function `format_echo()` exists and takes `&str` parameter
- `format_echo("hello")` returns `"Echo: hello"`
- `format_echo("")` returns `"Echo: "`

---

### 3.5 Update main() function

- [x] Replace placeholder implementation with `Args::parse()` call
- [x] Call `get_message()` and handle result
- [x] Print formatted output or show help on error

**Acceptance Criteria:**
- `main()` calls `Args::parse()` to parse command-line arguments
- Successful message retrieval prints `format_echo()` result via `println!`
- Failed message retrieval (no input) triggers help display

---

### 3.6 Add unit tests

- [x] Add `#[cfg(test)]` module with tests for `format_echo()`
- [x] Test simple message: `"hello"` -> `"Echo: hello"`
- [x] Test message with spaces: `"Hello, world!"` -> `"Echo: Hello, world!"`
- [x] Test multiline message: `"Line 1\nLine 2"` -> `"Echo: Line 1\nLine 2"`
- [x] Test empty message: `""` -> `"Echo: "`

**Acceptance Criteria:**
- Test module exists in `synapse-cli/src/main.rs`
- Running `cargo test -p synapse-cli` shows 4 tests passing
- All tests verify `format_echo()` behavior correctly

---

### 3.7 Verify build and lint compliance

- [x] Run `cargo build -p synapse-cli` and confirm success
- [x] Run `cargo clippy -p synapse-cli -- -D warnings` and confirm no warnings
- [x] Run `cargo fmt --check` and confirm formatting is correct
- [x] Run `cargo test -p synapse-cli` and confirm all tests pass

**Acceptance Criteria:**
- `cargo build -p synapse-cli` exits with code 0
- `cargo clippy -p synapse-cli -- -D warnings` exits with code 0
- `cargo fmt --check` exits with code 0
- `cargo test -p synapse-cli` shows all tests passing

---

### 3.8 Manual acceptance testing

- [x] Test one-shot mode: `cargo run -p synapse-cli -- "hello"` outputs `Echo: hello`
- [x] Test stdin mode: `echo "hello" | cargo run -p synapse-cli` outputs `Echo: hello`
- [x] Test multiline stdin: `echo -e "Line 1\nLine 2" | cargo run -p synapse-cli` outputs `Echo: Line 1` followed by `Line 2`
- [x] Test no input: `cargo run -p synapse-cli` shows help message
- [x] Test version flag: `cargo run -p synapse-cli -- --version` shows version
- [x] Test help flag: `cargo run -p synapse-cli -- --help` shows usage information

**Acceptance Criteria:**
- All 6 manual test scenarios produce expected output
- One-shot and stdin modes both work correctly
- Help and version flags are functional

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 3.1 | Add clap dependency | `synapse-cli/Cargo.toml` |
| 3.2 | Implement Args struct | `synapse-cli/src/main.rs` |
| 3.3 | Implement get_message() | `synapse-cli/src/main.rs` |
| 3.4 | Implement format_echo() | `synapse-cli/src/main.rs` |
| 3.5 | Update main() function | `synapse-cli/src/main.rs` |
| 3.6 | Add unit tests | `synapse-cli/src/main.rs` |
| 3.7 | Build and lint verification | N/A (verification commands) |
| 3.8 | Manual acceptance testing | N/A (CLI execution) |
