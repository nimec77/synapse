# Research: SY-3 - Echo CLI

## Overview

This document captures the technical research for ticket SY-3, which implements the Echo CLI feature for the Synapse project. This phase establishes the CLI input/output foundation by adding argument parsing with `clap` and implementing both one-shot and stdin input modes.

---

## 1. Current CLI Structure

### synapse-cli Crate

Location: `/Users/comrade77/RustroverProjects/synapse/synapse-cli/`

**Cargo.toml:**
```toml
[package]
name = "synapse-cli"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "synapse"
path = "src/main.rs"

[dependencies]
# No dependencies for Phase 1
```

**Current main.rs:**
```rust
//! Synapse CLI - Command-line interface for the Synapse AI agent.

fn main() {
    println!("Synapse CLI");
}
```

### Observations

1. **No dependencies** - Phase 1 only established the workspace structure
2. **Binary name is `synapse`** - Matches the expected CLI invocation pattern
3. **Edition is inherited from workspace** - Uses Edition 2024 (nightly required)
4. **Minimal implementation** - Just prints "Synapse CLI"

---

## 2. Dependencies Needed

### Primary Dependency: clap

Per PRD and `docs/phase-2.md`, add clap with derive features:

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

### clap Features

| Feature | Purpose |
|---------|---------|
| `derive` | Enables `#[derive(Parser)]` for declarative argument definition |
| `cargo` | (Optional) Read version from Cargo.toml - included by default with derive |

### Version Considerations

- **clap 4.x**: Current stable version, recommended by PRD
- **Rust 2024 compatibility**: clap 4.x is compatible with nightly/Edition 2024
- **No additional dependencies needed** for the echo functionality

### Optional: TTY Detection

The PRD mentions `atty` crate for detecting if stdin is a TTY. However:
- Modern clap can handle optional arguments naturally
- Standard library `std::io::IsTerminal` (stabilized in Rust 1.70) can detect TTY
- Recommendation: Use `std::io::IsTerminal` to avoid adding another dependency

---

## 3. Implementation Patterns

### CLI Argument Structure

Based on `docs/vision.md` section on CLI Interface and PRD requirements:

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

### Project Structure per vision.md

The vision document specifies these files for synapse-cli:
```
synapse-cli/
├── Cargo.toml
└── src/
    ├── main.rs            # Entry point
    ├── args.rs            # CLI arguments (clap) - future
    ├── repl.rs            # Interactive REPL - future
    ├── oneshot.rs         # One-shot mode - future
    └── ui.rs              # Terminal UI (ratatui) - future
```

For SY-3, the implementation is simple enough to keep in `main.rs`. Splitting into modules (args.rs, etc.) can be deferred to when the CLI grows more complex.

### Error Handling Pattern

Per `docs/conventions.md`:
- Use `anyhow` in CLI (ergonomic error chains)
- Propagate errors with `?` operator
- Write user-facing messages that are actionable

For SY-3, errors are minimal (just stdin reading). Using `anyhow` or standard `Result` is acceptable.

---

## 4. Stdin Handling Approach

### Detection Strategy

Three scenarios to handle:

1. **Positional argument provided**: `synapse "hello"` - Use the argument
2. **Stdin piped**: `echo "hello" | synapse` - Read from stdin
3. **Interactive terminal, no argument**: `synapse` - Show help or wait

### Implementation Options

**Option A: Simple stdin check with IsTerminal**
```rust
use std::io::{self, IsTerminal, Read};

fn get_message(args: &Args) -> io::Result<String> {
    if let Some(msg) = &args.message {
        return Ok(msg.clone());
    }

    // No argument provided, check stdin
    if io::stdin().is_terminal() {
        // Interactive mode - no input provided
        // Option: show help, or prompt for input
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "No message provided. Use: synapse \"message\" or echo \"message\" | synapse"
        ));
    }

    // Stdin is piped, read it
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.trim_end().to_string())
}
```

**Option B: Just read stdin if no argument**
```rust
fn get_message(args: &Args) -> io::Result<String> {
    if let Some(msg) = &args.message {
        return Ok(msg.clone());
    }

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer.trim_end().to_string())
}
```

### Recommendation

Use Option A with TTY detection:
- Better user experience when running without arguments
- Clear error message guides the user
- Matches PRD Scenario 4 behavior

### Multiline Handling

Per PRD Scenario 3:
```bash
$ echo -e "Line 1\nLine 2" | synapse
Echo: Line 1
Line 2
```

The `read_to_string` approach handles multiline input naturally. The "Echo: " prefix is only added to the beginning.

---

## 5. Module System

Per `docs/conventions.md` and `CLAUDE.md`:

**DO:**
- Use new Rust module system (Rust 2018+)
- Place module file at `src/module.rs` with submodules in `src/module/`

**DO NOT:**
- Use `mod.rs` files

For SY-3, keeping everything in `main.rs` is appropriate given the minimal scope. Future phases may introduce `args.rs` for argument definitions.

---

## 6. Testing Approach

### Unit Tests

Test argument parsing and echo logic:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_format() {
        let result = format_echo("hello");
        assert_eq!(result, "Echo: hello");
    }

    #[test]
    fn test_echo_multiline() {
        let result = format_echo("Line 1\nLine 2");
        assert_eq!(result, "Echo: Line 1\nLine 2");
    }
}
```

### Integration Tests

The PRD acceptance criteria define the integration tests:

```bash
# Test 1: One-shot mode
synapse "hello"
# Expected output: Echo: hello

# Test 2: Stdin mode
echo "hello" | synapse
# Expected output: Echo: hello
```

These can be run manually or via a shell script.

### Test Naming Convention

Per `docs/conventions.md`:
- Name tests: `test_<function>_<scenario>`

---

## 7. Output Format

### Echo Prefix

Per PRD:
- Prefix: `"Echo: "`
- No trailing newline manipulation beyond what `println!` provides

### Examples

| Input | Output |
|-------|--------|
| `"hello"` | `Echo: hello` |
| `"Hello, world!"` | `Echo: Hello, world!` |
| `"Line 1\nLine 2"` | `Echo: Line 1\nLine 2` |
| Empty string | `Echo: ` |

---

## 8. Help and Version Flags

clap automatically provides:
- `--help` / `-h`: Show usage information
- `--version` / `-V`: Show version (from Cargo.toml)

Per PRD Decisions section:
- Help flag: Yes (clap provides automatically)
- Version flag: Yes (clap provides automatically)

---

## 9. Technical Considerations

### Rust Edition 2024

The workspace uses Edition 2024 on nightly. Key considerations:
- `std::io::IsTerminal` is stable since Rust 1.70
- clap 4.x is compatible with current nightly

### UTF-8 Handling

Per PRD assumptions:
- UTF-8 encoding for all text input
- Rust strings are UTF-8 by default, no special handling needed

### Empty Input

When stdin is empty or only whitespace:
- `read_to_string` returns empty or whitespace
- `trim_end()` removes trailing whitespace
- Empty string results in `Echo: ` output

### Large Input

For very large stdin input:
- `read_to_string` loads everything into memory
- For SY-3 (echo mode), this is acceptable
- Future phases with LLM integration will need to consider message size limits

---

## 10. Related Files

| File | Purpose | Modifications Needed |
|------|---------|---------------------|
| `synapse-cli/Cargo.toml` | Crate manifest | Add clap dependency |
| `synapse-cli/src/main.rs` | Entry point | Replace with echo implementation |

---

## 11. Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| clap version conflicts | Low | Low | Pin clap 4.x explicitly |
| Edition 2024 compatibility | Low | Medium | clap 4.x tested with nightly |
| Stdin blocking on TTY | Low | Low | Use IsTerminal to detect and handle |
| Large stdin memory usage | Low | Low | Acceptable for echo; address in future phases |

---

## 12. Implementation Checklist

- [ ] Add `clap = { version = "4", features = ["derive"] }` to synapse-cli/Cargo.toml
- [ ] Define `Args` struct with Parser derive
- [ ] Implement message acquisition (argument or stdin)
- [ ] Implement TTY detection for empty argument case
- [ ] Format and print echo output
- [ ] Add unit tests for echo formatting
- [ ] Verify with acceptance criteria tests
- [ ] Run `cargo fmt`, `cargo clippy`, `cargo test`

---

## 13. Open Technical Questions

### Resolved by PRD

1. **Q: What to do when no input and interactive terminal?**
   A: Show help/usage message (PRD Decisions section).

2. **Q: How to handle empty input?**
   A: Print `Echo: ` (empty echo is valid).

3. **Q: Trim whitespace from stdin?**
   A: Yes, trim trailing whitespace (newlines from piping).

### Remaining Considerations

1. **Separate args.rs module?**
   - Recommendation: Keep in main.rs for SY-3; extract when complexity grows.

2. **anyhow vs std::io::Result?**
   - Recommendation: Use std::io::Result for SY-3; add anyhow when error handling becomes more complex.

3. **Stdin read timeout?**
   - Not needed for SY-3; read_to_string blocks until EOF which is appropriate for piped input.

---

## 14. Recommendations

1. **Start simple** - Keep implementation in main.rs, defer module extraction.

2. **Use standard library** - Use `std::io::IsTerminal` instead of adding `atty` crate.

3. **Follow conventions** - Use `thiserror`/`anyhow` patterns from conventions.md when error handling grows.

4. **Test thoroughly** - Cover both one-shot and stdin modes, including edge cases.

5. **Document public items** - Add doc comments per conventions.md requirements.
