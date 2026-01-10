# Tasklist: SY-1 - Phase 1: Project Foundation

Status: TASKLIST_READY

## Context

Establish the foundational Rust workspace structure for the Synapse project. This phase creates a multi-crate workspace with three members (`synapse-core`, `synapse-cli`, `synapse-telegram`) that compiles successfully.

---

## Tasks

### 1.1 Create workspace Cargo.toml

- [ ] Create `Cargo.toml` at workspace root with workspace members and shared package configuration

**Acceptance Criteria:**
- File `Cargo.toml` exists at project root
- Contains `[workspace]` section with resolver = "2" and members array listing all three crates
- Contains `[workspace.package]` section with edition = "2024" and rust-version = "1.85"

---

### 1.2 Create synapse-core crate

- [ ] Create `synapse-core/Cargo.toml` with package configuration
- [ ] Create `synapse-core/src/lib.rs` with placeholder module

**Acceptance Criteria:**
- `synapse-core/Cargo.toml` exists with `edition.workspace = true`
- `synapse-core/src/lib.rs` exports a `placeholder` module with a `hello()` function
- Doc comments are present on the module and function

---

### 1.3a Create synapse-cli crate

- [ ] Create `synapse-cli/Cargo.toml` with binary configuration
- [ ] Create `synapse-cli/src/main.rs` that prints "Synapse CLI"

**Acceptance Criteria:**
- `synapse-cli/Cargo.toml` exists with `[[bin]]` section defining `name = "synapse"`
- Running `cargo run -p synapse-cli` prints "Synapse CLI" to stdout

---

### 1.3b Create synapse-telegram crate

- [ ] Create `synapse-telegram/Cargo.toml` with binary configuration
- [ ] Create `synapse-telegram/src/main.rs` that prints "Synapse Telegram Bot"

**Acceptance Criteria:**
- `synapse-telegram/Cargo.toml` exists with `[[bin]]` section defining `name = "synapse-telegram"`
- Running `cargo run -p synapse-telegram` prints "Synapse Telegram Bot" to stdout

---

### 1.4 Verify build and quality checks

- [ ] Run `cargo build` and confirm exit code 0
- [ ] Run `cargo fmt --check` and confirm no formatting issues
- [ ] Run `cargo clippy` and confirm no errors or warnings

**Acceptance Criteria:**
- `cargo build` completes with exit code 0
- `cargo fmt --check` passes (no changes required)
- `cargo clippy` passes (no errors or warnings)

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1.1 | Workspace Cargo.toml | `Cargo.toml` |
| 1.2 | synapse-core crate | `synapse-core/Cargo.toml`, `synapse-core/src/lib.rs` |
| 1.3a | synapse-cli crate | `synapse-cli/Cargo.toml`, `synapse-cli/src/main.rs` |
| 1.3b | synapse-telegram crate | `synapse-telegram/Cargo.toml`, `synapse-telegram/src/main.rs` |
| 1.4 | Build verification | N/A (verification commands) |
