# Phase 1: Project Foundation

**Goal:** Workspace compiles, all crates exist.

## Tasks

- [x] 1.1 Create workspace `Cargo.toml` with members: `synapse-core`, `synapse-cli`, `synapse-telegram`
- [x] 1.2 Create `synapse-core` crate with `lib.rs` exporting placeholder module
- [x] 1.3 Create `synapse-cli` crate with `main.rs` printing "Synapse CLI"
- [x] 1.4 Verify: `cargo build` succeeds for entire workspace

## Test

`cargo run -p synapse-cli` prints "Synapse CLI"
