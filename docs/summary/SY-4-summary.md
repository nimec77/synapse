# SY-4 Summary: Configuration System

**Status:** Complete
**Date:** 2026-01-11

---

## Overview

SY-4 implements the TOML-based configuration system for Synapse (Phase 3). This feature allows the application to load settings from configuration files with support for multiple file locations, environment variable overrides, and sensible defaults. The configuration system establishes the foundation for LLM provider integration in future phases.

The configuration module resides in `synapse-core` as a shared library component, making it accessible to all Synapse interfaces (CLI, Telegram bot, backend service).

---

## What Was Implemented

### Configuration Loading

The system loads configuration from TOML files with the following priority order:

1. `SYNAPSE_CONFIG` environment variable (highest priority)
2. `./config.toml` (local directory)
3. `~/.config/synapse/config.toml` (user config)
4. Default values (when no config file exists)

### Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `provider` | String | `"deepseek"` | LLM provider name |
| `api_key` | Option<String> | None | API key (optional in this phase) |
| `model` | String | `"deepseek-chat"` | Model identifier |

### CLI Integration

The CLI now loads configuration at startup and displays the provider name:

```bash
synapse "hello"
# Output:
# Provider: deepseek
# Echo: hello
```

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Default provider | `"deepseek"` | Cost-effective default for development |
| Provider validation | Accept any string | Validation deferred to Phase 4 when providers are used |
| api_key type | `Option<String>` | Not required until actual LLM calls |
| Missing config behavior | Return defaults | Graceful degradation for new users |
| Path resolution | `dirs::home_dir()` | Cross-platform consistency |
| Config error handling in CLI | `unwrap_or_default()` | Falls back to defaults on any error |
| Config in .gitignore | Yes | Prevent accidental API key commits |

---

## Files Created

| File | Purpose |
|------|---------|
| `synapse-core/src/config.rs` | Config struct, ConfigError enum, load/load_from methods |
| `config.example.toml` | Documented example configuration |

---

## Files Modified

| File | Change |
|------|--------|
| `synapse-core/Cargo.toml` | Added dependencies: toml, serde, dirs, thiserror |
| `synapse-core/src/lib.rs` | Added `pub mod config;` and re-exports |
| `synapse-cli/Cargo.toml` | Added synapse-core dependency |
| `synapse-cli/src/main.rs` | Load config at startup, print provider name |
| `.gitignore` | Added `/config.toml` to prevent API key commits |

---

## API Contract

### Config Struct

```rust
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub provider: String,      // Default: "deepseek"
    pub api_key: Option<String>, // Default: None
    pub model: String,         // Default: "deepseek-chat"
}
```

### Loading Methods

```rust
impl Config {
    /// Load from file system with priority chain
    pub fn load() -> Result<Self, ConfigError>;

    /// Load from a specific path
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError>;
}
```

### Error Types

```rust
pub enum ConfigError {
    IoError { path: PathBuf, source: std::io::Error },
    ParseError { path: PathBuf, source: toml::de::Error },
}
```

### Public Exports

From `synapse_core`:
- `synapse_core::Config`
- `synapse_core::ConfigError`
- `synapse_core::config` (module)

---

## Configuration File Format

Example `config.toml`:

```toml
# LLM provider to use
provider = "deepseek"

# API key for the provider (keep this file secure!)
# api_key = "your-api-key-here"

# Model to use
model = "deepseek-chat"
```

Security note: The example file documents `chmod 600` for file permissions.

---

## Testing Summary

### Unit Tests (8 tests)

| Test | Coverage |
|------|----------|
| `test_default_config` | Default values verification |
| `test_parse_minimal_toml` | Partial config with defaults |
| `test_parse_full_toml` | All fields specified |
| `test_parse_partial_toml` | Model only, others default |
| `test_parse_empty_toml` | Empty file handling |
| `test_load_from_path` | Load from temp file |
| `test_parse_invalid_toml` | ParseError returned |
| `test_load_from_nonexistent_file` | IoError returned |

Run tests:
```bash
cargo test -p synapse-core
```

### Quality Checks

| Check | Status |
|-------|--------|
| `cargo fmt --check` | Pass |
| `cargo clippy -- -D warnings` | Pass |
| `cargo test` | Pass (8 config tests) |
| `cargo build` | Pass |

---

## Usage Examples

### Basic Configuration

```bash
# Create local config
echo 'provider = "anthropic"' > config.toml

# Run synapse
synapse "hello"
# Output:
# Provider: anthropic
# Echo: hello
```

### Environment Variable Override

```bash
# Create custom config
echo 'provider = "openai"' > /tmp/custom.toml

# Use custom path
SYNAPSE_CONFIG=/tmp/custom.toml synapse "hello"
# Output:
# Provider: openai
# Echo: hello
```

### User Config Directory

```bash
# Create user config
mkdir -p ~/.config/synapse
cat > ~/.config/synapse/config.toml << EOF
provider = "anthropic"
api_key = "sk-your-key"
model = "claude-3-5-sonnet-20241022"
EOF

# Secure the file
chmod 600 ~/.config/synapse/config.toml
```

### No Configuration (Defaults)

```bash
# Without any config file
synapse "hello"
# Output:
# Provider: deepseek
# Echo: hello
```

---

## Dependencies Added

### synapse-core

| Crate | Version | Purpose |
|-------|---------|---------|
| `toml` | 0.8 | TOML parsing |
| `serde` | 1 (with derive) | Deserialization |
| `dirs` | 5 | Home directory resolution |
| `thiserror` | 2 | Error type derivation |

### synapse-cli

| Crate | Path | Purpose |
|-------|------|---------|
| `synapse-core` | `../synapse-core` | Access to Config and ConfigError |

---

## Security Considerations

1. **API key protection**: `config.toml` added to `.gitignore` to prevent accidental commits
2. **File permissions**: `config.example.toml` documents `chmod 600` recommendation
3. **No secret logging**: API key is not printed in CLI output
4. **Optional api_key**: Not required for basic operation

---

## Limitations

1. **Config::load() not unit tested**: Testing requires environment mocking which adds complexity
2. **No provider validation**: Accepts any string; validation happens when provider is actually used
3. **No nested configuration**: Flat structure with three fields only
4. **No hot reload**: Configuration loaded once at startup

---

## Future Enhancements

This configuration system establishes patterns that will be extended:

1. **Phase 4+**: Provider-specific configuration sections
2. **Environment variable overrides**: `SYNAPSE_API_KEY`, `SYNAPSE_PROVIDER`
3. **Additional fields**: `system_prompt`, `session_db_path`, `timeout`
4. **Config subcommand**: `synapse config show`, `synapse config set`
5. **Validation**: Provider name validation against supported providers

---

## QA Status

The QA report (`reports/qa/SY-4.md`) recommends **RELEASE**:

- All 15 tasks complete
- All PRD goals met
- All user stories satisfied
- 8 unit tests passing
- Security considerations addressed
- Implementation matches approved plan
