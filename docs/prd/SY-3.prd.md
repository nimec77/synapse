# SY-3: Phase 2 - Echo CLI

Status: PRD_READY

## Context / Idea

This phase establishes the CLI input/output foundation for Synapse. The goal is to make the CLI accept user input and echo it back, demonstrating that the argument parsing infrastructure works correctly before integrating with LLM providers.

From Phase 2 specification:
- Add `clap` to `synapse-cli` and define basic argument structure
- Implement one-shot mode: `synapse "hello"` prints "Echo: hello"
- Implement stdin mode: `echo "hello" | synapse` prints "Echo: hello"

This is a foundational step that validates the CLI can receive input through multiple methods (command-line arguments and stdin), which will later be extended to send messages to LLM providers.

## Goals

1. **Add clap dependency**: Integrate `clap` crate with derive features for declarative argument parsing
2. **Define CLI argument structure**: Create the `Args` struct in `synapse-cli` with a positional message argument
3. **Implement one-shot mode**: Accept a message as a positional argument and echo it with "Echo: " prefix
4. **Implement stdin mode**: When no positional argument is provided, read from stdin and echo the input
5. **Establish CLI patterns**: Set up the argument parsing patterns that will be extended in future phases

## User Stories

1. **As a user**, I want to run `synapse "hello"` and see "Echo: hello" printed to stdout, so that I can quickly send single messages.

2. **As a user**, I want to pipe text into synapse via `echo "hello" | synapse` and see "Echo: hello" printed to stdout, so that I can integrate synapse with other command-line tools.

3. **As a user**, I want clear error messages if I provide invalid input, so that I understand what went wrong.

## Main Scenarios

### Scenario 1: One-shot mode with positional argument
```bash
$ synapse "Hello, world!"
Echo: Hello, world!
```

### Scenario 2: Stdin mode with piped input
```bash
$ echo "Hello from stdin" | synapse
Echo: Hello from stdin
```

### Scenario 3: Stdin mode with multiline input
```bash
$ echo -e "Line 1\nLine 2" | synapse
Echo: Line 1
Line 2
```

### Scenario 4: No input provided (empty stdin, no argument)
```bash
$ synapse
# Should either wait for stdin input or show usage/help
```

## Success / Metrics

1. **Functional correctness**: Both invocation methods (positional arg and stdin) return the echoed input with "Echo: " prefix
2. **Build validation**: `cargo build -p synapse-cli` completes successfully
3. **Test coverage**: Unit tests for argument parsing and echo logic pass
4. **Lint compliance**: `cargo clippy -p synapse-cli` reports no warnings
5. **Code formatting**: `cargo fmt --check` passes

### Acceptance Criteria

```bash
# Test 1: One-shot mode
synapse "hello"
# Expected output: Echo: hello

# Test 2: Stdin mode
echo "hello" | synapse
# Expected output: Echo: hello

# Test 3: Build and lint
cargo build -p synapse-cli
cargo clippy -p synapse-cli -- -D warnings
cargo test -p synapse-cli
```

## Constraints and Assumptions

### Constraints

1. **Dependency**: Phase 1 must be complete (workspace compiles successfully)
2. **Clap version**: Use clap 4.x with derive feature for idiomatic Rust argument parsing
3. **Module style**: Follow new Rust module system (no mod.rs files) as per project conventions
4. **Rust edition**: Must work with Rust 2024 edition on nightly

### Assumptions

1. The user has a terminal that supports standard input/output
2. UTF-8 encoding for all text input
3. The echo functionality is a stepping stone; the "Echo: " prefix will be replaced with actual LLM responses in later phases

## Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Stdin detection complexity | Low | Low | Use `atty` crate or standard `isatty()` to detect if stdin is a TTY |
| Clap version conflicts | Low | Low | Pin clap version explicitly in Cargo.toml |
| Multiline stdin handling | Low | Low | Read all stdin before processing, handle newlines appropriately |

## Decisions

1. **Empty input behavior**: Show help/usage message when no arguments and stdin is a TTY
2. **Help flag**: Yes, implement `synapse --help` (clap provides this automatically)
3. **Version flag**: Yes, implement `synapse --version` (clap provides this automatically)
