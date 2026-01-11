# Plan: SY-4 - Configuration System

Status: PLAN_APPROVED

## Overview

This plan describes the implementation of the configuration system for Synapse. The system enables loading settings from TOML files with support for multiple file locations, environment variable overrides, and sensible defaults.

---

## 1. Components

### 1.1 Files to Create

| File | Purpose |
|------|---------|
| `synapse-core/src/config.rs` | Config struct, error types, and loading logic |
| `config.example.toml` | Example configuration with documentation |

### 1.2 Files to Modify

| File | Changes |
|------|---------|
| `synapse-core/Cargo.toml` | Add dependencies: toml, serde, dirs, thiserror |
| `synapse-core/src/lib.rs` | Add `pub mod config;` and re-exports |
| `synapse-cli/Cargo.toml` | Add dependency on synapse-core |
| `synapse-cli/src/main.rs` | Load config, print provider name |

### 1.3 Module Structure

```
synapse-core/src/
  lib.rs              # pub mod config; pub use config::{Config, ConfigError};
  config.rs           # Config struct and loading logic
  placeholder.rs      # (existing, unchanged)
```

Per conventions.md: No `mod.rs` files. Use new Rust module system.

---

## 2. API Contract

### 2.1 Config Struct

```rust
use serde::Deserialize;

/// Application configuration loaded from TOML file.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// LLM provider name (e.g., "deepseek", "anthropic", "openai").
    /// No validation - any string accepted.
    #[serde(default = "default_provider")]
    pub provider: String,

    /// API key for the provider. Optional in this phase.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model name to use (e.g., "deepseek-chat", "claude-3-5-sonnet-20241022").
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_provider() -> String {
    "deepseek".to_string()
}

fn default_model() -> String {
    "deepseek-chat".to_string()
}
```

### 2.2 Default Implementation

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            provider: "deepseek".to_string(),
            api_key: None,
            model: "deepseek-chat".to_string(),
        }
    }
}
```

### 2.3 Config Methods

```rust
impl Config {
    /// Load configuration from file system.
    ///
    /// Priority order:
    /// 1. SYNAPSE_CONFIG environment variable
    /// 2. ./config.toml (local directory)
    /// 3. ~/.config/synapse/config.toml (user config)
    ///
    /// Returns default config if no config file found.
    pub fn load() -> Result<Self, ConfigError>;

    /// Load configuration from a specific path.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError>;
}
```

### 2.4 Error Types

```rust
use thiserror::Error;

/// Errors that can occur when loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read the configuration file.
    #[error("failed to read config file '{path}': {source}")]
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Failed to parse the configuration file as TOML.
    #[error("failed to parse config file '{path}': {source}")]
    ParseError {
        path: PathBuf,
        source: toml::de::Error,
    },
}
```

### 2.5 Public Exports from synapse-core

```rust
// In lib.rs
pub mod config;

pub use config::{Config, ConfigError};
```

---

## 3. Data Flow

### 3.1 Configuration Loading Sequence

```
CLI main()
    |
    v
Config::load()
    |
    +---> Check SYNAPSE_CONFIG env var
    |         |
    |         +---> If set and file exists: load from that path
    |         |
    |         +---> If set but file missing: fall through to next
    |
    +---> Check ./config.toml
    |         |
    |         +---> If exists: load from local path
    |
    +---> Check ~/.config/synapse/config.toml
    |         |
    |         +---> If exists: load from user config
    |
    +---> No file found: return Config::default()
    |
    v
Config::load_from(path)
    |
    +---> Read file contents (fs::read_to_string)
    |         |
    |         +---> Error: return ConfigError::IoError
    |
    +---> Parse TOML (toml::from_str)
    |         |
    |         +---> Error: return ConfigError::ParseError
    |
    +---> Apply serde defaults for missing fields
    |
    v
Return Ok(Config)
```

### 3.2 CLI Integration Flow

```
synapse "hello"
    |
    v
Parse CLI args (clap)
    |
    v
Config::load().unwrap_or_default()
    |
    v
Print "Provider: {provider}"
    |
    v
Get message (arg or stdin)
    |
    v
Print "Echo: {message}"
```

### 3.3 Path Resolution Logic

```rust
fn resolve_config_path() -> Option<PathBuf> {
    // 1. Environment variable (highest priority)
    if let Ok(path) = std::env::var("SYNAPSE_CONFIG") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Local directory
    let local = PathBuf::from("config.toml");
    if local.exists() {
        return Some(local);
    }

    // 3. User config directory (~/.config/synapse/)
    // Use home_dir for cross-platform consistency per vision.md
    if let Some(home) = dirs::home_dir() {
        let user_config = home.join(".config/synapse/config.toml");
        if user_config.exists() {
            return Some(user_config);
        }
    }

    None
}
```

---

## 4. Non-Functional Requirements

### 4.1 Performance

| Requirement | Target | Rationale |
|-------------|--------|-----------|
| Config load time | < 10ms | Per vision.md targets |
| Memory footprint | < 1KB | Config is small struct with 3 fields |

The implementation uses synchronous file I/O which is acceptable for a single small file read at startup.

### 4.2 Reliability

| Requirement | Implementation |
|-------------|----------------|
| Graceful degradation | Return defaults if no config file exists |
| Clear error messages | Include file path and parse error details |
| Partial config support | Use serde defaults for missing fields |

### 4.3 Security

| Requirement | Implementation |
|-------------|----------------|
| API key handling | Store as Option<String>, document chmod 600 |
| No secret logging | Do not log api_key value |
| Clear warnings | Document security in config.example.toml |

### 4.4 Cross-Platform

| Platform | Config Path |
|----------|-------------|
| Linux | `~/.config/synapse/config.toml` |
| macOS | `~/.config/synapse/config.toml` |
| Windows | `~/.config/synapse/config.toml` |

Note: Use `dirs::home_dir()` + `.config/synapse/` for consistency across platforms per vision.md, rather than platform-specific dirs.

---

## 5. Dependencies

### 5.1 synapse-core/Cargo.toml

```toml
[dependencies]
toml = "0.8"
serde = { version = "1", features = ["derive"] }
dirs = "5"
thiserror = "2"
```

### 5.2 synapse-cli/Cargo.toml

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
```

---

## 6. Testing Strategy

### 6.1 Unit Tests (in config.rs)

| Test | Description |
|------|-------------|
| `test_default_config` | Verify default values (deepseek, None, deepseek-chat) |
| `test_parse_minimal_toml` | Parse TOML with only provider and model |
| `test_parse_full_toml` | Parse TOML with all fields including api_key |
| `test_parse_partial_toml` | Parse TOML with missing fields, verify defaults applied |
| `test_parse_invalid_toml` | Verify ParseError on malformed TOML |
| `test_load_from_path` | Load config from specific file path |

### 6.2 Integration Tests

```bash
# Test with local config
echo 'provider = "anthropic"' > config.toml
synapse "test"
# Expected: Provider: anthropic

# Test with SYNAPSE_CONFIG
SYNAPSE_CONFIG=/tmp/test.toml synapse "test"

# Test with no config (defaults)
rm -f config.toml
synapse "test"
# Expected: Provider: deepseek
```

---

## 7. Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Config file with insecure permissions | Medium | High | Document chmod 600 in example file; future: runtime warning |
| Invalid TOML crashes application | Low | Medium | Return ConfigError with file path and parse details |
| SYNAPSE_CONFIG points to non-existent file | Low | Low | Fall through to next location in priority |
| Missing required fields in config | Medium | Low | Use serde defaults for all fields |
| Different path conventions on Windows | Low | Low | Use explicit ~/.config/synapse path via dirs::home_dir() |

---

## 8. Trade-offs

### 8.1 Synchronous vs Asynchronous File I/O

**Decision**: Use synchronous `std::fs::read_to_string`

**Rationale**:
- Config loading happens once at startup
- File is small (< 1KB typically)
- Async would add complexity without benefit
- Matches conventions.md (no blocking in async functions - but this is sync context)

### 8.2 Path Resolution Strategy

**Decision**: Use `dirs::home_dir()` + `.config/synapse/` across all platforms

**Rationale**:
- Consistent path across Linux, macOS, Windows
- Matches vision.md specification
- Alternative (dirs::config_dir) would give different paths per platform

### 8.3 Provider Validation

**Decision**: Accept any string for provider field, no validation

**Rationale**:
- Per research document user answer
- Validation happens when provider is actually used (Phase 4+)
- Allows forward compatibility with new providers

### 8.4 Error Handling for Missing Config

**Decision**: Return `Config::default()` instead of error when no config file exists

**Rationale**:
- Per PRD US-4: users should be able to run without creating config
- Reduces friction for new users
- Error only returned for actual failures (IO error, parse error)

---

## 9. Example Configuration File

`config.example.toml` (to be created in repo root):

```toml
# Synapse Configuration
# Copy this file to one of:
#   - ./config.toml (local, highest priority after SYNAPSE_CONFIG env var)
#   - ~/.config/synapse/config.toml (user default)
#
# You can also set SYNAPSE_CONFIG environment variable to a custom path.

# LLM provider to use
# Options: deepseek, anthropic, openai (more providers coming)
provider = "deepseek"

# API key for the selected provider
# You can also use provider-specific environment variables in future phases
# WARNING: Keep this file secure! Run: chmod 600 ~/.config/synapse/config.toml
# api_key = "your-api-key-here"

# Model to use
# DeepSeek: deepseek-chat, deepseek-coder
# Anthropic: claude-3-5-sonnet-20241022, claude-3-opus-20240229
# OpenAI: gpt-4, gpt-4-turbo, gpt-3.5-turbo
model = "deepseek-chat"
```

---

## 10. Open Questions

None - all questions from PRD have been resolved in the research phase:

| Question | Resolution |
|----------|------------|
| Environment variable override | Yes, implement SYNAPSE_CONFIG |
| Provider validation | Accept any string |
| Default provider | "deepseek" |
| Default model | "deepseek-chat" |
| api_key handling | Option<String> |
| Additional fields | Minimal scope (provider, api_key, model only) |
| Path resolution crate | Use `dirs` crate |

---

## 11. Implementation Order

1. Add dependencies to `synapse-core/Cargo.toml`
2. Create `synapse-core/src/config.rs` with Config struct and ConfigError
3. Implement `Config::load()` and `Config::load_from()`
4. Update `synapse-core/src/lib.rs` with module declaration and re-exports
5. Add synapse-core dependency to `synapse-cli/Cargo.toml`
6. Update `synapse-cli/src/main.rs` to load config and print provider
7. Create `config.example.toml` in repo root
8. Add unit tests
9. Run `cargo fmt`, `cargo clippy`, `cargo test`
10. Verify acceptance criteria
