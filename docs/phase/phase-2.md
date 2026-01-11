# Phase 2: Echo CLI

**Goal:** CLI accepts input and echoes it back.

## Tasks

- [x] 2.1 Add `clap` to `synapse-cli`, define basic args (message as positional arg)
- [x] 2.2 Implement one-shot mode: `synapse "hello"` → prints "Echo: hello"
- [x] 2.3 Implement stdin mode: `echo "hello" | synapse` → prints "Echo: hello"

## Acceptance Criteria

**Test:** Both invocation methods return echoed input.

```bash
# One-shot mode
synapse "hello"
# Expected output: Echo: hello

# Stdin mode
echo "hello" | synapse
# Expected output: Echo: hello
```

## Dependencies

- Phase 1 complete (workspace compiles)
- `clap` crate for argument parsing

## Implementation Notes

### 2.1 Add clap dependency

Add to `synapse-cli/Cargo.toml`:
```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

### 2.2 One-shot mode

Parse positional argument and echo it back with "Echo: " prefix.

### 2.3 Stdin mode

Detect when no argument is provided and read from stdin instead.
