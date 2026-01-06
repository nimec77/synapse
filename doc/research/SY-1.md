# Research: SY-1 - Phase 1: Project Foundation

**Ticket:** SY-1
**Status:** Research Complete
**Date:** 2026-01-06

---

## 1. Summary

SY-1 establishes the foundational workspace structure for the Synapse project. The goal is to create a Rust workspace with three crates (`synapse-core`, `synapse-cli`, `synapse-telegram`) that compiles successfully.

---

## 2. Existing Endpoints and Contracts

### 2.1 Current State

**No code exists yet.** The project is documentation-only at this stage:

| Directory/File | Status | Purpose |
|----------------|--------|---------|
| `src/` | Does not exist | No source code |
| `Cargo.toml` (root) | Does not exist | Workspace manifest needed |
| `synapse-core/` | Does not exist | Core library crate needed |
| `synapse-cli/` | Does not exist | CLI binary crate needed |
| `synapse-telegram/` | Does not exist | Telegram bot crate needed |

### 2.2 Target Structure (from vision.md)

```
synapse/
├── Cargo.toml                 # Workspace manifest
├── synapse-core/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs             # Placeholder export
├── synapse-cli/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # Prints "Synapse CLI"
└── synapse-telegram/
    ├── Cargo.toml
    └── src/
        └── main.rs            # Placeholder
```

### 2.3 Public API Contracts (Phase 1 scope)

Phase 1 defines no public APIs. The only contract is:

- `synapse-cli` binary outputs `"Synapse CLI"` when run

---

## 3. Layers and Dependencies

### 3.1 Workspace Architecture

```
┌─────────────────────────────────────────────────────┐
│           Interface Layer (CLI/Telegram)            │
│  synapse-cli         synapse-telegram               │
│        │                    │                       │
│        └────────┬───────────┘                       │
│                 ▼                                   │
├─────────────────────────────────────────────────────┤
│         Core Library (synapse-core)                 │
└─────────────────────────────────────────────────────┘
```

### 3.2 Crate Dependencies (Phase 1)

| Crate | Dependencies | Purpose |
|-------|--------------|---------|
| `synapse-core` | None | Placeholder library |
| `synapse-cli` | `synapse-core` (path) | CLI binary |
| `synapse-telegram` | `synapse-core` (path) | Telegram bot binary |

### 3.3 Rust Toolchain Requirements

| Requirement | Value | Source |
|-------------|-------|--------|
| Edition | 2024 | CLAUDE.md |
| Toolchain | Nightly | CLAUDE.md |
| Module style | New (no `mod.rs`) | conventions.md |

---

## 4. Patterns Used

### 4.1 Workspace Pattern (Cargo)

The project uses a **flat workspace layout** with crates at the root level:

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "3"
members = [
    "synapse-core",
    "synapse-cli",
    "synapse-telegram",
]
```

### 4.2 Shared Dependencies Pattern

Future phases will use workspace-level dependency definitions:

```toml
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
# ... etc
```

### 4.3 Module System Pattern (Rust 2018+)

From conventions.md - must NOT use `mod.rs`:

```
# Correct pattern:
src/
├── provider.rs        # declares: mod anthropic;
└── provider/
    └── anthropic.rs

# Incorrect pattern (prohibited):
src/
└── provider/
    ├── mod.rs         # ❌ Never use
    └── anthropic.rs
```

---

## 5. Limitations and Risks

### 5.1 Technical Risks

| Risk | Impact | Mitigation | Likelihood |
|------|--------|------------|------------|
| Nightly toolchain instability | Build failures | Pin known working nightly | Low |
| Edition 2024 features unstabilized | Compilation errors | Fallback to Edition 2021 | Low |
| Resolver version 3 not supported | Workspace won't compile | Use resolver "2" | Low |

### 5.2 Constraints

| Constraint | Description | Source |
|------------|-------------|--------|
| No `mod.rs` files | Use new module system | conventions.md |
| No external dependencies | Phase 1 is minimal | PRD |
| Rust nightly required | Edition 2024 needs nightly | CLAUDE.md |

### 5.3 Assumptions

- Developer has Rust nightly installed (`rustup default nightly`)
- No external crate dependencies needed for Phase 1
- `synapse-telegram` only needs placeholder `main.rs`

---

## 6. Open Technical Questions

### 6.1 Resolved Questions

| Question | Answer | Source |
|----------|--------|--------|
| What Rust edition? | 2024 | CLAUDE.md |
| Workspace layout? | Flat (crates at root) | vision.md §3 |
| Module system? | New style (no mod.rs) | conventions.md |
| Resolver version? | "3" (edition 2024 default) | Rust docs |

### 6.2 Questions for Implementation

| Question | Options | Recommendation |
|----------|---------|----------------|
| What placeholder to export in `synapse-core`? | Empty module / const / function | Empty `pub mod placeholder;` with single const |
| Verify Edition 2024 resolver? | "2" or "3" | Try "3" first, fallback to "2" |

---

## 7. Implementation Checklist

Based on PRD and phase-1.md:

| Task | Description | Files |
|------|-------------|-------|
| 1.1 | Create workspace Cargo.toml | `Cargo.toml` |
| 1.2 | Create synapse-core crate | `synapse-core/Cargo.toml`, `synapse-core/src/lib.rs` |
| 1.3 | Create synapse-cli crate | `synapse-cli/Cargo.toml`, `synapse-cli/src/main.rs` |
| 1.3 | Create synapse-telegram crate | `synapse-telegram/Cargo.toml`, `synapse-telegram/src/main.rs` |
| 1.4 | Verify build | Run `cargo build` |

### Verification Commands

```bash
# Build entire workspace
cargo build

# Run CLI (should print "Synapse CLI")
cargo run -p synapse-cli

# Linting (must pass)
cargo clippy -- -D warnings

# Formatting check
cargo fmt --check
```

---

## 8. Reference Files

| Document | Purpose |
|----------|---------|
| `doc/prd/SY-1.prd.md` | PRD with goals, stories, success criteria |
| `doc/phase-1.md` | Task list for Phase 1 |
| `doc/vision.md` | Technical architecture and project structure |
| `doc/conventions.md` | Coding rules and prohibitions |
| `doc/tasklist.md` | Overall project progress tracker |
| `doc/workflow.md` | Development workflow (propose → implement → verify → commit) |

---

## 9. Conclusion

Phase 1 (SY-1) is a straightforward workspace setup task with no external dependencies. The main considerations are:

1. **Use Rust 2024 edition** with nightly toolchain
2. **Flat workspace layout** with three crates at root level
3. **No `mod.rs` files** - use new module system
4. **Minimal placeholders** - just enough to compile

After Phase 1 completion, the workspace will be ready for Phase 2 (Echo CLI) where actual functionality begins.
