# Research: SY-1 - Phase 1: Project Foundation

## Overview

This document captures the technical research for ticket SY-1, which establishes the foundational workspace structure for the Synapse project.

---

## 1. Existing Codebase State

### Current Project Structure

```
synapse/
├── .claude/              # Claude Code configuration
├── .git/                 # Git repository
├── .gitignore
├── .vscode/              # VS Code settings
├── CLAUDE.md             # Claude Code guidance
└── docs/
    ├── .active_ticket    # Current: SY-1
    ├── conventions.md    # Coding rules
    ├── idea.md           # Project concept
    ├── phase-1.md        # Phase 1 task breakdown
    ├── prd/
    │   └── SY-1.prd.md   # PRD for this ticket
    ├── prd.template.md   # PRD template
    ├── research/         # Research documents (this folder)
    ├── tasklist.md       # Development plan
    ├── vision.md         # Technical architecture
    └── workflow.md       # Collaboration process
```

### Key Finding

**No source code exists yet.** The project contains only documentation. No `Cargo.toml`, no `src/` directories, no `.rs` files. This is a greenfield implementation.

---

## 2. Target Workspace Structure

Per `docs/vision.md` §3, the target structure is:

```
synapse/
├── Cargo.toml                 # Workspace manifest
├── synapse-core/              # Core library crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs             # Public API exports
├── synapse-cli/               # CLI binary crate
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # Entry point
└── synapse-telegram/          # Telegram bot crate
    ├── Cargo.toml
    └── src/
        └── main.rs            # Bot entry point
```

---

## 3. Rust Toolchain Requirements

### Edition & Toolchain

| Requirement | Value | Source |
|-------------|-------|--------|
| Rust Edition | 2024 | vision.md §1 |
| Toolchain | Nightly | vision.md §1 |

### Verification Command

```bash
rustc +nightly --version
```

### Risk: Edition 2024

Edition 2024 is the latest Rust edition (stabilized in Rust 1.85, February 2025). Key changes:
- `gen` is now a reserved keyword
- RPIT (Return Position Impl Trait) captures more lifetimes by default
- `unsafe extern` blocks are now the default

These changes are unlikely to affect this minimal Phase 1 setup.

---

## 4. Module System Convention

Per `docs/conventions.md` and `CLAUDE.md`:

**Required:** New Rust module system (Rust 2018+)

```
# Correct (new style)
src/
├── provider.rs        # declares: mod anthropic; mod openai;
└── provider/
    ├── anthropic.rs
    └── openai.rs

# Incorrect (DO NOT USE)
src/
└── provider/
    └── mod.rs         # ❌ Prohibited
```

For Phase 1, only `lib.rs` and `main.rs` are needed - no submodules yet.

---

## 5. Minimal Cargo Configuration

### Workspace `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "synapse-core",
    "synapse-cli",
    "synapse-telegram",
]

[workspace.package]
edition = "2024"
rust-version = "1.85"
authors = ["Your Name <email@example.com>"]
license = "MIT"
```

### Crate `Cargo.toml` Pattern

```toml
[package]
name = "synapse-core"
version = "0.1.0"
edition.workspace = true

[dependencies]
# No dependencies for Phase 1
```

---

## 6. Implementation Tasks

| Task | Description | Files to Create |
|------|-------------|-----------------|
| 1.1 | Workspace Cargo.toml | `Cargo.toml` |
| 1.2 | synapse-core crate | `synapse-core/Cargo.toml`, `synapse-core/src/lib.rs` |
| 1.3 | synapse-cli crate | `synapse-cli/Cargo.toml`, `synapse-cli/src/main.rs` |
| 1.3 | synapse-telegram crate | `synapse-telegram/Cargo.toml`, `synapse-telegram/src/main.rs` |
| 1.4 | Verification | Run `cargo build` and `cargo run -p synapse-cli` |

---

## 7. Placeholder Content

### `synapse-core/src/lib.rs`

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

### `synapse-cli/src/main.rs`

```rust
//! Synapse CLI - Command-line interface for the Synapse AI agent.

fn main() {
    println!("Synapse CLI");
}
```

### `synapse-telegram/src/main.rs`

```rust
//! Synapse Telegram Bot - Telegram interface for the Synapse AI agent.

fn main() {
    println!("Synapse Telegram Bot");
}
```

---

## 8. Success Criteria

| Check | Command | Expected Result |
|-------|---------|-----------------|
| Build succeeds | `cargo build` | Exit code 0, no errors |
| CLI runs | `cargo run -p synapse-cli` | Prints "Synapse CLI" |
| Telegram runs | `cargo run -p synapse-telegram` | Prints "Synapse Telegram Bot" |
| Linting passes | `cargo clippy` | No errors |
| Formatting passes | `cargo fmt --check` | No changes needed |

---

## 9. Dependencies & Patterns

### Phase 1 Dependencies

None. All three crates are dependency-free for this phase.

### Future Dependencies (from vision.md)

| Crate | Purpose | Phase |
|-------|---------|-------|
| `clap` | CLI arguments | Phase 2 |
| `toml`, `serde` | Configuration | Phase 3 |
| `thiserror`, `anyhow` | Error handling | Phase 4+ |
| `tokio` | Async runtime | Phase 5 |
| `reqwest` | HTTP client | Phase 5 |
| `sqlx` | Database | Phase 7 |
| `ratatui` | REPL UI | Phase 8 |
| `rmcp` | MCP protocol | Phase 10 |
| `teloxide` | Telegram bot | Phase 11 |

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Nightly toolchain not installed | Build fails | Document: `rustup default nightly` |
| Edition 2024 features break | Compilation errors | Can fallback to Edition 2021 if needed |
| Workspace path issues | Members not found | Use relative paths from workspace root |

---

## 11. Open Technical Questions

**None.** This phase is straightforward and all requirements are clearly defined:

- Workspace structure is documented in `vision.md` §3
- Module conventions are documented in `CLAUDE.md` and `conventions.md`
- Toolchain requirements are documented in `vision.md` §1
- Success criteria are documented in `phase-1.md`

---

## 12. Recommendations

1. **Start with workspace Cargo.toml** - This defines the structure for all crates.

2. **Create crates in order**: core → cli → telegram - Core first as it will be a dependency of the others.

3. **Verify after each crate** - Run `cargo check` incrementally to catch issues early.

4. **Use `cargo fmt` and `cargo clippy`** after creating files to ensure compliance with conventions.

5. **Commit atomically** - One commit for the complete Phase 1 setup, or split by task if preferred.
