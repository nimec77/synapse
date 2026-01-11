# SY-1: Phase 1: Project Foundation

Status: PRD_READY

## Context / Idea

**Title:** Phase 1: Project Foundation

**Reference:** docs/phase-1.md

**Goal:** Workspace compiles, all crates exist.

This phase establishes the foundational structure for the Synapse project - a Rust-based AI agent that serves as a unified interface to interact with multiple LLM providers. The project follows a workspace layout with three crates:
- `synapse-core`: Core library with agent logic, providers, storage, and MCP support
- `synapse-cli`: CLI binary for REPL and one-shot modes
- `synapse-telegram`: Telegram bot interface

The architecture follows Hexagonal Architecture (Ports and Adapters) pattern to ensure testability, flexibility, and maintainability. This foundational phase creates the skeleton workspace that all subsequent development will build upon.

**Tasks from phase-1.md:**
- 1.1 Create workspace `Cargo.toml` with members: `synapse-core`, `synapse-cli`, `synapse-telegram`
- 1.2 Create `synapse-core` crate with `lib.rs` exporting placeholder module
- 1.3 Create `synapse-cli` crate with `main.rs` printing "Synapse CLI"
- 1.4 Verify: `cargo build` succeeds for entire workspace

**Verification Test:** `cargo run -p synapse-cli` prints "Synapse CLI"

## Goals

1. **Establish Rust Workspace Structure**: Create a multi-crate workspace that enables modular development and code sharing between interface implementations.

2. **Enable Compilation**: Ensure the entire workspace compiles successfully with `cargo build`.

3. **Prepare for Incremental Development**: Lay groundwork for subsequent phases by having proper crate structure in place.

4. **Follow Project Conventions**: Use Rust 2024 edition, nightly toolchain, and avoid `mod.rs` files per project conventions.

## User Stories

1. **As a developer**, I want the workspace to compile so that I can begin implementing core functionality in subsequent phases.

2. **As a developer**, I want separate crates for core logic and interfaces so that I can develop and test them independently.

3. **As a developer**, I want to run `cargo run -p synapse-cli` and see output so that I can verify the project setup is correct.

## Main Scenarios

### Scenario 1: Initial Workspace Setup
**Given** an empty project directory
**When** the workspace `Cargo.toml` and all crate structures are created
**Then** `cargo build` succeeds without errors

### Scenario 2: CLI Verification
**Given** the workspace is set up correctly
**When** running `cargo run -p synapse-cli`
**Then** the output displays "Synapse CLI"

### Scenario 3: Core Library Placeholder
**Given** the `synapse-core` crate exists
**When** building the workspace
**Then** `synapse-core` compiles with a placeholder module export

## Success / Metrics

| Metric | Success Criteria |
|--------|------------------|
| Compilation | `cargo build` completes with exit code 0 |
| CLI Output | `cargo run -p synapse-cli` prints "Synapse CLI" |
| Workspace Members | All three crates (`synapse-core`, `synapse-cli`, `synapse-telegram`) are recognized |
| Linting | `cargo clippy` passes without errors |
| Formatting | `cargo fmt --check` passes |

## Constraints and Assumptions

### Constraints
- **Rust Edition**: Must use Edition 2024 (nightly toolchain required)
- **Module Style**: Must use new Rust module system (no `mod.rs` files)
- **Workspace Layout**: Flat workspace with crates at root level

### Assumptions
- Developer has Rust nightly toolchain installed
- No external dependencies required for this phase (minimal Cargo.toml)
- The `synapse-telegram` crate only needs a placeholder `main.rs` for this phase

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Nightly toolchain instability | Build failures on certain nightly versions | Document known working nightly version |
| Edition 2024 features not stabilized | Compilation errors | Monitor Rust release notes, fallback to Edition 2021 if critical |

## Open Questions

None. This phase is straightforward and all requirements are clearly defined in the project documentation.
