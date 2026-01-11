# Tasklist: SY-4 - Configuration System

Status: REVIEW_COMPLETE

## Context

Phase 3 of the Synapse project introduces a TOML-based configuration system in `synapse-core`. This enables loading settings such as provider name, API key, and model from configuration files with support for multiple locations (env var, local, user config) and sensible defaults.

---

## Tasks

### Task 1: Add dependencies to synapse-core

Add required dependencies for configuration parsing to `synapse-core/Cargo.toml`.

- [x] 1.1 Add `toml = "0.8"` dependency
- [x] 1.2 Add `serde = { version = "1", features = ["derive"] }` dependency
- [x] 1.3 Add `dirs = "5"` dependency
- [x] 1.4 Add `thiserror = "2"` dependency

**Acceptance Criteria:**
- `cargo check -p synapse-core` succeeds with no errors
- All four dependencies are listed in `synapse-core/Cargo.toml`

---

### Task 2: Create ConfigError enum

Create the error type for configuration loading failures in `synapse-core/src/config.rs`.

- [x] 2.1 Create `synapse-core/src/config.rs` file
- [x] 2.2 Define `ConfigError` enum with `IoError` and `ParseError` variants using `thiserror`
- [x] 2.3 Both variants include `path: PathBuf` field for context

**Acceptance Criteria:**
- `ConfigError::IoError` contains `path` and `source: std::io::Error`
- `ConfigError::ParseError` contains `path` and `source: toml::de::Error`
- Error messages include the file path (verified via Display impl)

---

### Task 3: Create Config struct with defaults

Define the `Config` struct with serde deserialization and default values.

- [x] 3.1 Define `Config` struct with fields: `provider: String`, `api_key: Option<String>`, `model: String`
- [x] 3.2 Add `#[derive(Debug, Clone, Deserialize)]` to Config
- [x] 3.3 Implement serde defaults: provider = "deepseek", model = "deepseek-chat"
- [x] 3.4 Implement `Default` trait for Config

**Acceptance Criteria:**
- `Config::default()` returns provider = "deepseek", api_key = None, model = "deepseek-chat"
- Unit test `test_default_config` passes verifying default values

---

### Task 4: Implement Config::load_from method

Implement loading configuration from a specific file path.

- [x] 4.1 Implement `Config::load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError>`
- [x] 4.2 Read file contents using `std::fs::read_to_string`
- [x] 4.3 Parse TOML content using `toml::from_str`
- [x] 4.4 Return appropriate `ConfigError` variants on failure

**Acceptance Criteria:**
- Unit test `test_load_from_path` loads config from a temp file successfully
- Unit test `test_parse_invalid_toml` returns `ConfigError::ParseError` for malformed TOML
- Missing fields in TOML file use serde defaults

---

### Task 5: Implement Config::load method with path resolution

Implement the main config loading method with priority-based path resolution.

- [x] 5.1 Implement `Config::load() -> Result<Self, ConfigError>`
- [x] 5.2 Check `SYNAPSE_CONFIG` environment variable first
- [x] 5.3 Check `./config.toml` (local directory) second
- [x] 5.4 Check `~/.config/synapse/config.toml` (user config) third
- [x] 5.5 Return `Config::default()` if no config file exists

**Acceptance Criteria:**
- Config loads from `SYNAPSE_CONFIG` path when env var is set and file exists
- Config loads from `./config.toml` when it exists (and no env var)
- Config loads from `~/.config/synapse/config.toml` when it exists (and no local config)
- Config returns defaults when no config file is found at any location

---

### Task 6: Export config module from synapse-core

Add module declaration and public re-exports to the core library.

- [x] 6.1 Add `pub mod config;` to `synapse-core/src/lib.rs`
- [x] 6.2 Add `pub use config::{Config, ConfigError};` re-export

**Acceptance Criteria:**
- `synapse_core::Config` is accessible from external crates
- `synapse_core::ConfigError` is accessible from external crates
- `cargo doc -p synapse-core` shows Config and ConfigError in documentation

---

### Task 7: Add synapse-core dependency to synapse-cli

Update CLI crate to depend on the core library.

- [x] 7.1 Add `synapse-core = { path = "../synapse-core" }` to `synapse-cli/Cargo.toml`

**Acceptance Criteria:**
- `cargo check -p synapse-cli` succeeds
- `synapse-cli` can import types from `synapse_core`

---

### Task 8: Integrate config loading in CLI

Update the CLI to load configuration and display provider information.

- [x] 8.1 Import `synapse_core::Config` in `synapse-cli/src/main.rs`
- [x] 8.2 Call `Config::load().unwrap_or_default()` at startup
- [x] 8.3 Print "Provider: {provider}" before the echo output

**Acceptance Criteria:**
- `synapse "test"` with no config file prints "Provider: deepseek" and "Echo: test"
- `synapse "test"` with `config.toml` containing `provider = "anthropic"` prints "Provider: anthropic"

---

### Task 9: Create config.example.toml

Create the example configuration file in the repository root.

- [x] 9.1 Create `config.example.toml` with documented configuration options
- [x] 9.2 Include provider, api_key (commented), and model fields
- [x] 9.3 Add comments explaining each option and security warning for api_key
- [x] 9.4 Document file locations and SYNAPSE_CONFIG env var

**Acceptance Criteria:**
- `config.example.toml` exists in repository root
- File is valid TOML (can be parsed without errors)
- Contains provider, api_key, and model fields
- Includes chmod 600 security warning

---

### Task 10: Add unit tests

Add comprehensive unit tests for the configuration module.

- [x] 10.1 Add `#[cfg(test)]` module in `config.rs`
- [x] 10.2 Test `test_default_config` - verify default values
- [x] 10.3 Test `test_parse_minimal_toml` - parse TOML with only provider
- [x] 10.4 Test `test_parse_full_toml` - parse TOML with all fields
- [x] 10.5 Test `test_parse_partial_toml` - verify defaults for missing fields

**Acceptance Criteria:**
- `cargo test -p synapse-core` passes all configuration tests
- Tests cover default values, partial config, and full config scenarios

---

### Task 11: Final verification

Run all quality checks and verify acceptance criteria from PRD.

- [x] 11.1 Run `cargo fmt --check` - no formatting issues
- [x] 11.2 Run `cargo clippy -- -D warnings` - no warnings
- [x] 11.3 Run `cargo test` - all tests pass
- [x] 11.4 Verify PRD acceptance criteria are met

**Acceptance Criteria:**
- All CI checks pass (fmt, clippy, test)
- `synapse "test"` with provider = "anthropic" in config outputs text including "Provider: anthropic"
- Config struct is accessible from `synapse-core` public API
- `config.example.toml` documents all configuration options

---

## Code Review Fixes

- [x] **Task 12: Add missing unit tests for error paths**
  - Add `test_load_from_path` test that creates a temp file, writes valid TOML, and verifies `Config::load_from()` loads it correctly
  - Add `test_parse_invalid_toml` test that verifies `ConfigError::ParseError` is returned for malformed TOML (e.g., `"invalid = ["`)
  - Add `test_load_from_nonexistent_file` test that verifies `ConfigError::IoError` is returned for missing files
  - Acceptance criteria:
    - All three tests exist in `synapse-core/src/config.rs`
    - Tests verify both error variants are correctly constructed with file paths
    - `cargo test -p synapse-core` passes

- [x] **Task 13: Add config.toml to .gitignore**
  - Add `config.toml` to `.gitignore` to prevent accidental commit of API keys
  - Acceptance criteria:
    - `/config.toml` is listed in `.gitignore`
    - Running `git status` does not show `config.toml` as untracked (if it exists locally)

- [x] **Task 14: Add PartialEq derive to Config struct**
  - Add `PartialEq` to the derive macro on `Config` struct for easier testing and comparison
  - Acceptance criteria:
    - `Config` struct derives `PartialEq`
    - `cargo check -p synapse-core` succeeds

- [x] **Task 15: Final verification after review fixes**
  - Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
  - Acceptance criteria:
    - All checks pass
    - All new tests pass

---

## Summary

| Task | Description | Status |
|------|-------------|--------|
| 1 | Add dependencies to synapse-core | Completed |
| 2 | Create ConfigError enum | Completed |
| 3 | Create Config struct with defaults | Completed |
| 4 | Implement Config::load_from method | Completed |
| 5 | Implement Config::load method with path resolution | Completed |
| 6 | Export config module from synapse-core | Completed |
| 7 | Add synapse-core dependency to synapse-cli | Completed |
| 8 | Integrate config loading in CLI | Completed |
| 9 | Create config.example.toml | Completed |
| 10 | Add unit tests | Completed |
| 11 | Final verification | Completed |
| 12 | Add missing unit tests for error paths | Completed |
| 13 | Add config.toml to .gitignore | Completed |
| 14 | Add PartialEq derive to Config struct | Completed |
| 15 | Final verification after review fixes | Completed |

**Total Tasks:** 15
**Completed:** 15
**Pending (Review Fixes):** 0
