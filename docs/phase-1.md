# Phase 1: Project Foundation

**Goal:** Workspace compiles, all crates exist.

## Tasks

- [ ] 1.1 Create workspace `Cargo.toml` with members: `synapse-core`, `synapse-cli`, `synapse-telegram`
- [ ] 1.2 Create `synapse-core` crate with `lib.rs` exporting placeholder module
- [ ] 1.3 Create `synapse-cli` crate with `main.rs` printing "Synapse CLI"
- [ ] 1.4 Verify: `cargo build` succeeds for entire workspace

## Test

`cargo run -p synapse-cli` prints "Synapse CLI"
