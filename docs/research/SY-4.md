# Research: SY-4 - Configuration System

## Overview

This document captures the technical research for ticket SY-4, which implements the configuration system for the Synapse project. This phase enables loading settings from a TOML file, supporting multiple configuration file locations with sensible defaults.

---

## 1. Resolved Questions (User Answers)

The following questions from the PRD have been resolved by user input:

| Question | User Answer |
|----------|-------------|
| **Environment Variable Override** | Include in this phase - implement `SYNAPSE_CONFIG` env var support |
| **Validation Scope** | Accept any string - validation happens when provider is used |
| **API Key Handling** | Optional - use `api_key: Option<String>` |
| **Additional Fields** | Minimal scope - only `provider`, `api_key`, `model` as specified |
| **Default Provider** | DeepSeek (not anthropic as originally suggested in PRD) |
| **Path Resolution Crate** | Use `dirs` crate |

---

## 2. Current Code Structure

### synapse-core Crate

Location: `/Users/comrade77/RustroverProjects/synapse/synapse-core/`

**Current Cargo.toml:**
```toml
[package]
name = "synapse-core"
version = "0.1.0"
edition.workspace = true

[dependencies]
# No dependencies for Phase 1
```

**Current lib.rs:**
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

### synapse-cli Crate

Location: `/Users/comrade77/RustroverProjects/synapse/synapse-cli/`

**Current Cargo.toml:**
```toml
[package]
name = "synapse-cli"
version = "0.1.0"
edition.workspace = true
description = "Synapse CLI - AI agent command-line interface"

[[bin]]
name = "synapse"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
```

**Current main.rs** (key parts):
- Uses `clap::Parser` for argument parsing
- Implements `get_message()` function for input acquisition (argument or stdin)
- Implements `format_echo()` function for output formatting
- Currently prints "Echo: {message}" - this needs modification to include provider info

### Observations

1. **synapse-core has no dependencies** - needs `toml`, `serde`, and `dirs` crates
2. **synapse-cli does not depend on synapse-core** - needs to add dependency
3. **Current CLI only echoes** - needs to load config and print provider name
4. **Module system uses new style** - no `mod.rs` files, which is correct
5. **Edition 2024 with nightly** - modern Rust features available

---

## 3. Dependencies Needed

### synapse-core Dependencies

```toml
[dependencies]
toml = "0.8"
serde = { version = "1", features = ["derive"] }
dirs = "5"
thiserror = "2"
```

| Crate | Version | Purpose |
|-------|---------|---------|
| `toml` | 0.8 | TOML parsing and deserialization |
| `serde` | 1 (with derive) | Serialization/deserialization traits |
| `dirs` | 5 | Cross-platform standard paths (config_dir, etc.) |
| `thiserror` | 2 | Error type definitions for library code |

### synapse-cli Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
```

The CLI needs to depend on synapse-core to use the Config module.

---

## 4. Configuration File Locations

### Priority Order (per PRD and vision.md)

1. **Environment Variable**: `SYNAPSE_CONFIG` - highest priority
2. **Local Directory**: `./config.toml` - project-specific override
3. **User Config Directory**: `~/.config/synapse/config.toml` - user default

### Platform-Specific Paths (via `dirs` crate)

| Platform | Config Path |
|----------|-------------|
| Linux | `~/.config/synapse/config.toml` |
| macOS | `~/.config/synapse/config.toml` (or `~/Library/Application Support/synapse/config.toml` per dirs) |
| Windows | `%APPDATA%\synapse\config.toml` |

**Note**: The `dirs::config_dir()` function returns:
- Linux: `~/.config`
- macOS: `~/Library/Application Support`
- Windows: `%APPDATA%` (e.g., `C:\Users\<User>\AppData\Roaming`)

Per vision.md, macOS should use `~/.config/synapse/` for consistency. Implementation should handle this explicitly.

### Resolution Logic

```rust
fn resolve_config_path() -> Option<PathBuf> {
    // 1. Check SYNAPSE_CONFIG env var
    if let Ok(path) = std::env::var("SYNAPSE_CONFIG") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Check local ./config.toml
    let local = PathBuf::from("config.toml");
    if local.exists() {
        return Some(local);
    }

    // 3. Check user config directory
    // Use ~/.config/synapse/config.toml across all platforms for consistency
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

## 5. Config Struct Design

### Minimal Fields (per user answer)

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// LLM provider name (e.g., "deepseek", "anthropic", "openai")
    pub provider: String,

    /// API key for the provider (optional in this phase)
    pub api_key: Option<String>,

    /// Model name to use (e.g., "deepseek-chat", "claude-3-5-sonnet-20241022")
    pub model: String,
}
```

### Default Values

Per user answer, default provider is DeepSeek:

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

### TOML Structure

```toml
# Provider to use for LLM calls
provider = "deepseek"

# API key (optional - can also use environment variables)
# api_key = "your-api-key-here"

# Model to use
model = "deepseek-chat"
```

---

## 6. Error Handling

### Error Types (thiserror)

Per conventions.md, use `thiserror` in library code:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),
}
```

### Fallback Behavior

Per PRD Scenario 3:
- If no config file exists at any location, use defaults
- If config file exists but is invalid TOML, return error with line number
- If config file exists but lacks required fields, use defaults for missing fields (serde `#[serde(default)]`)

---

## 7. Module System Pattern

Per CLAUDE.md and conventions.md, use new Rust module system:

```
synapse-core/src/
  lib.rs              # pub mod config; pub use config::Config;
  config.rs           # Config struct and loading logic
```

**lib.rs additions:**
```rust
pub mod config;

// Re-export for convenience
pub use config::{Config, ConfigError};
```

---

## 8. CLI Integration

### Changes to synapse-cli/src/main.rs

1. Add dependency on `synapse-core`
2. Load config at startup
3. Modify output to include provider name

**Proposed output format:**
```
Provider: deepseek
Echo: hello
```

Or per the PRD acceptance criteria:
```
Provider: deepseek
<existing echo output>
```

### Implementation Approach

```rust
use synapse_core::Config;

fn main() {
    let args = Args::parse();

    // Load configuration (falls back to defaults)
    let config = Config::load().unwrap_or_default();

    // Print provider info
    println!("Provider: {}", config.provider);

    // Continue with existing echo logic...
}
```

---

## 9. Testing Strategy

### Unit Tests for Config

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.provider, "deepseek");
        assert!(config.api_key.is_none());
        assert_eq!(config.model, "deepseek-chat");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml = r#"
            provider = "anthropic"
            model = "claude-3-5-sonnet"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "anthropic");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_parse_full_toml() {
        let toml = r#"
            provider = "openai"
            api_key = "sk-test-key"
            model = "gpt-4"
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.api_key, Some("sk-test-key".to_string()));
        assert_eq!(config.model, "gpt-4");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let toml = "invalid = [";
        let result: Result<Config, _> = toml::from_str(toml);
        assert!(result.is_err());
    }
}
```

### Integration Tests

```bash
# Test with config file
echo 'provider = "anthropic"' > config.toml
synapse "test"
# Expected: Provider: anthropic\nEcho: test

# Test with SYNAPSE_CONFIG env var
SYNAPSE_CONFIG=/path/to/config.toml synapse "test"

# Test with no config (defaults)
rm -f config.toml
synapse "test"
# Expected: Provider: deepseek\nEcho: test
```

---

## 10. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `synapse-core/Cargo.toml` | Modify | Add toml, serde, dirs, thiserror dependencies |
| `synapse-core/src/config.rs` | Create | Config struct and loading logic |
| `synapse-core/src/lib.rs` | Modify | Add config module, re-exports |
| `synapse-cli/Cargo.toml` | Modify | Add synapse-core dependency |
| `synapse-cli/src/main.rs` | Modify | Load config, print provider name |
| `config.example.toml` | Create | Example configuration with documentation |

---

## 11. Example Configuration File

```toml
# Synapse Configuration
# Copy this file to ~/.config/synapse/config.toml and customize

# LLM provider to use
# Options: deepseek, anthropic, openai (more to come)
provider = "deepseek"

# API key for the selected provider
# You can also set this via environment variable (e.g., DEEPSEEK_API_KEY)
# WARNING: Keep this file secure (chmod 600 on Unix systems)
# api_key = "your-api-key-here"

# Model to use
# DeepSeek: deepseek-chat, deepseek-coder
# Anthropic: claude-3-5-sonnet-20241022, claude-3-opus-20240229
# OpenAI: gpt-4, gpt-4-turbo, gpt-3.5-turbo
model = "deepseek-chat"
```

---

## 12. Limitations and Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Config file with wrong permissions (security) | Medium | High | Document chmod 600; future: warn at runtime |
| `dirs` crate path inconsistency | Low | Low | Use explicit ~/.config/synapse path |
| Invalid TOML crashes app | Low | Medium | Return error with parse location |
| Missing required fields | Medium | Low | Use serde defaults for all fields |
| SYNAPSE_CONFIG points to non-existent file | Low | Low | Fall back to next location in priority |

### Security Considerations

Per vision.md section 11:
- API keys stored in plain text (document warning in example file)
- Recommend chmod 600 permissions on Unix
- Future enhancement: warn if config file has open permissions

---

## 13. API Surface Changes

### New Public API from synapse-core

```rust
// synapse_core::config module
pub struct Config {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
}

impl Config {
    /// Load configuration from file system.
    /// Returns default config if no config file found.
    pub fn load() -> Result<Self, ConfigError>;

    /// Load configuration from a specific path.
    pub fn load_from(path: &Path) -> Result<Self, ConfigError>;
}

impl Default for Config { ... }

#[derive(Debug, Error)]
pub enum ConfigError {
    IoError(#[from] std::io::Error),
    ParseError(#[from] toml::de::Error),
}
```

### CLI Output Change

Current:
```
Echo: hello
```

After SY-4:
```
Provider: deepseek
Echo: hello
```

---

## 14. Dependency Analysis

### Crate Sizes and Security

| Crate | Size | Dependencies | Security Notes |
|-------|------|--------------|----------------|
| `toml` 0.8 | ~200 KB | serde, winnow | Well-maintained, widely used |
| `serde` 1.x | ~60 KB | proc-macro | Core ecosystem crate |
| `dirs` 5.x | ~15 KB | Platform-specific | Minimal, well-audited |
| `thiserror` 2.x | ~10 KB | proc-macro | Standard error handling |

All recommended crates are widely used in the Rust ecosystem and have good security track records.

---

## 15. New Technical Questions Discovered

These questions arose during research but do not block implementation:

1. **macOS config path**: Should we use `~/Library/Application Support/synapse/` (per dirs crate default) or `~/.config/synapse/` (per vision.md)?
   - **Recommendation**: Use `~/.config/synapse/` for cross-platform consistency as stated in vision.md.

2. **Config file creation**: Should `Config::load()` create the config directory if it does not exist?
   - **Recommendation**: No, just return defaults. A future `synapse config init` command can create the file.

3. **Partial config parsing**: If the TOML file exists but is missing fields, should we use defaults for missing fields?
   - **Recommendation**: Yes, use `#[serde(default)]` to fill in missing fields with defaults.

4. **Environment variable for API key**: Should we also check `DEEPSEEK_API_KEY` / `ANTHROPIC_API_KEY` etc. in this phase?
   - **Recommendation**: Defer to provider implementation phase. This phase only handles config file and `SYNAPSE_CONFIG` path override.

---

## 16. Implementation Recommendations

1. **Keep scope minimal** - Only implement provider, api_key, model fields as specified

2. **Use serde defaults** - Ensure partial TOML files work by applying defaults to missing fields

3. **Explicit path for config** - Use `~/.config/synapse/config.toml` across all platforms for consistency

4. **Environment variable first** - Check `SYNAPSE_CONFIG` before other paths

5. **Graceful fallback** - If no config file exists, return default config without error

6. **Clear error messages** - Include file path and parse error details when TOML parsing fails

7. **Document security** - Include chmod 600 recommendation in config.example.toml

---

## 17. Implementation Checklist

- [ ] Add dependencies to `synapse-core/Cargo.toml` (toml, serde, dirs, thiserror)
- [ ] Create `synapse-core/src/config.rs` with Config struct
- [ ] Implement `Config::load()` with path resolution logic
- [ ] Implement `Config::load_from(path)` for direct path loading
- [ ] Implement `ConfigError` error type
- [ ] Update `synapse-core/src/lib.rs` to export config module
- [ ] Add `synapse-core` dependency to `synapse-cli/Cargo.toml`
- [ ] Update `synapse-cli/src/main.rs` to load config and print provider
- [ ] Create `config.example.toml` in repo root
- [ ] Add unit tests for config parsing
- [ ] Run `cargo fmt`, `cargo clippy`, `cargo test`
- [ ] Verify acceptance criteria: `synapse "test"` shows provider name
