# Plan: SY-1 - Phase 1: Project Foundation

Status: PLAN_APPROVED

## Overview

This plan covers the initial workspace setup for the Synapse project. The implementation creates a Rust workspace with three crates that compile successfully.

---

## 1. Components

### 1.1 Workspace Root

**File:** `Cargo.toml` (workspace manifest)

**Purpose:** Define the workspace members and shared configuration.

**Configuration:**
- Resolver version 3 (default for Edition 2024)
- Three workspace members: `synapse-core`, `synapse-cli`, `synapse-telegram`
- Shared package metadata (edition 2024, rust-version 1.85)

### 1.2 synapse-core Crate

**Files:**
- `synapse-core/Cargo.toml`
- `synapse-core/src/lib.rs`

**Purpose:** Core library that will contain agent logic, providers, storage, and MCP support.

**Phase 1 Scope:** Placeholder module with a simple function to verify compilation.

### 1.3 synapse-cli Crate

**Files:**
- `synapse-cli/Cargo.toml`
- `synapse-cli/src/main.rs`

**Purpose:** CLI binary for REPL and one-shot modes.

**Phase 1 Scope:** Print "Synapse CLI" to stdout.

### 1.4 synapse-telegram Crate

**Files:**
- `synapse-telegram/Cargo.toml`
- `synapse-telegram/src/main.rs`

**Purpose:** Telegram bot interface.

**Phase 1 Scope:** Print "Synapse Telegram Bot" to stdout.

---

## 2. Target Interfaces and Contracts

### 2.1 Workspace Cargo.toml

```toml
[workspace]
resolver = "3"
members = [
    "synapse-core",
    "synapse-cli",
    "synapse-telegram",
]

[workspace.package]
edition = "2024"
rust-version = "1.85"
authors = ["Synapse Contributors"]
license = "MIT"
```

### 2.2 synapse-core/Cargo.toml

```toml
[package]
name = "synapse-core"
version = "0.1.0"
edition.workspace = true

[dependencies]
# No dependencies for Phase 1
```

### 2.3 synapse-core/src/lib.rs

```rust
//! Synapse core library.
//!
//! Provides the agent orchestrator, LLM provider abstraction,
//! session management, and MCP integration.

/// Placeholder module for initial setup.
pub mod placeholder {
    /// Returns a greeting message.
    pub fn hello() -> &'static str {
        "Hello from synapse-core!"
    }
}
```

### 2.4 synapse-cli/Cargo.toml

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

### 2.5 synapse-cli/src/main.rs

```rust
//! Synapse CLI - Command-line interface for the Synapse AI agent.

fn main() {
    println!("Synapse CLI");
}
```

### 2.6 synapse-telegram/Cargo.toml

```toml
[package]
name = "synapse-telegram"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "synapse-telegram"
path = "src/main.rs"

[dependencies]
# No dependencies for Phase 1
```

### 2.7 synapse-telegram/src/main.rs

```rust
//! Synapse Telegram Bot - Telegram interface for the Synapse AI agent.

fn main() {
    println!("Synapse Telegram Bot");
}
```

---

## 3. Data Flows

Not applicable for Phase 1. This phase establishes the project structure without any data processing.

---

## 4. Implementation Tasks

| Order | Task ID | Description | Files |
|-------|---------|-------------|-------|
| 1 | 1.1 | Create workspace Cargo.toml | `Cargo.toml` |
| 2 | 1.2 | Create synapse-core crate | `synapse-core/Cargo.toml`, `synapse-core/src/lib.rs` |
| 3 | 1.3a | Create synapse-cli crate | `synapse-cli/Cargo.toml`, `synapse-cli/src/main.rs` |
| 4 | 1.3b | Create synapse-telegram crate | `synapse-telegram/Cargo.toml`, `synapse-telegram/src/main.rs` |
| 5 | 1.4 | Verify build and run | Run verification commands |

---

## 5. Non-Functional Requirements (NFRs)

### 5.1 Toolchain Compatibility

- **Requirement:** Must compile with Rust nightly toolchain
- **Edition:** 2024 (requires Rust 1.85+)
- **Verification:** `rustc +nightly --version` shows 1.85 or later

### 5.2 Code Quality

- **Formatting:** `cargo fmt --check` must pass
- **Linting:** `cargo clippy` must pass without errors
- **Documentation:** All public items must have doc comments

### 5.3 Build Performance

- **Target:** Initial build should complete in under 5 seconds (no dependencies)
- **Incremental:** Rebuilds should be near-instant

---

## 6. Verification Criteria

| Check | Command | Expected Result |
|-------|---------|-----------------|
| Workspace build | `cargo build` | Exit code 0, no errors |
| CLI execution | `cargo run -p synapse-cli` | Prints "Synapse CLI" |
| Telegram execution | `cargo run -p synapse-telegram` | Prints "Synapse Telegram Bot" |
| Formatting | `cargo fmt --check` | No changes required |
| Linting | `cargo clippy` | No errors or warnings |

---

## 7. Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Nightly toolchain not installed | Low | High | Document installation: `rustup default nightly` |
| Edition 2024 unavailable | Very Low | Medium | Fallback to Edition 2021 if necessary |
| Workspace path resolution issues | Low | Medium | Use relative paths from workspace root |

---

## 8. Dependencies

### 8.1 External Dependencies

None for Phase 1. All crates are dependency-free.

### 8.2 Internal Dependencies

None for Phase 1. Crates are independent at this stage.

### 8.3 Future Dependencies (for context)

The workspace structure is designed to support:
- `synapse-cli` depending on `synapse-core` (Phase 2+)
- `synapse-telegram` depending on `synapse-core` (Phase 11)

---

## 9. Open Questions

None. This phase is straightforward with well-defined requirements from the project documentation.

---

## 10. Alternatives Considered

### 10.1 Single Crate vs Workspace

**Chosen:** Workspace with multiple crates

**Rationale:**
- Enables code sharing between CLI and Telegram interfaces
- Supports independent compilation and testing
- Aligns with hexagonal architecture goals
- Documented in vision.md as the target structure

### 10.2 Edition 2021 vs 2024

**Chosen:** Edition 2024

**Rationale:**
- Uses latest Rust features and idioms
- Documented as a project requirement in vision.md
- Minimal risk for a new project (no migration needed)

No ADR is required as these decisions were already made in the vision document and this plan simply implements them.
