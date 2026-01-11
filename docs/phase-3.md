# Phase 3: Configuration

**Goal:** Load settings from TOML file.

## Tasks

- [ ] 3.1 Create `synapse-core/src/config.rs` with `Config` struct (provider, api_key, model)
- [ ] 3.2 Add `toml` + `serde` dependencies, implement TOML parsing
- [ ] 3.3 Create `config.example.toml` in repo root
- [ ] 3.4 Load config in CLI, print loaded provider name

## Acceptance Criteria

**Test:** `synapse "test"` prints "Provider: anthropic" (from config)

```bash
# With config.toml containing provider = "anthropic"
synapse "test"
# Expected output includes: Provider: anthropic
```

## Dependencies

- Phase 2 complete (Echo CLI works)
- `toml` crate for TOML parsing
- `serde` crate for deserialization

## Implementation Notes

### 3.1 Create Config struct

Create `synapse-core/src/config.rs` with:
- `Config` struct with fields: `provider`, `api_key`, `model`
- Export from `lib.rs`

### 3.2 Add dependencies and implement parsing

Add to `synapse-core/Cargo.toml`:
```toml
[dependencies]
toml = "0.8"
serde = { version = "1", features = ["derive"] }
```

Implement `Config::load()` method that:
- Reads from `~/.config/synapse/config.toml` or `./config.toml`
- Falls back to defaults if file not found

### 3.3 Create example config

Create `config.example.toml` with documented fields.

### 3.4 Wire into CLI

Load config at startup and print provider name to verify it works.
