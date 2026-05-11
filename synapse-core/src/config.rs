//! Configuration management for Synapse.
//!
//! Provides configuration loading from TOML files with support for
//! multiple file locations, environment variable overrides, and sensible defaults.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

/// Errors that can occur when loading configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read the configuration file.
    #[error("failed to read config file '{path}': {source}")]
    IoError {
        /// Path to the configuration file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to parse the configuration file as TOML.
    #[error("failed to parse config file '{path}': {source}")]
    ParseError {
        /// Path to the configuration file that could not be parsed.
        path: PathBuf,
        /// The underlying TOML parse error.
        source: toml::de::Error,
    },

    /// No configuration file found in any of the default search locations.
    #[error("config file not found; searched ./config.toml and ~/.config/synapse/config.toml")]
    NotFound,
}

/// Application configuration loaded from TOML file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    /// LLM provider name (e.g., "deepseek", "anthropic", "openai").
    #[serde(default = "default_provider")]
    pub provider: String,

    /// API key for the provider. Optional in this phase.
    #[serde(default)]
    pub api_key: Option<String>,

    /// Model name to use.
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens for LLM responses (default: 4096).
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// System prompt prepended to every LLM conversation.
    ///
    /// Shapes the AI's personality and instructions across all interactions.
    /// Injected on-the-fly via `Agent::build_messages()` and never stored in the
    /// session database.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Path to a file whose contents become the system prompt.
    ///
    /// Useful for long prompts (Markdown, structured instructions, etc.) that are
    /// impractical as inline TOML strings. Resolved during [`Config::load_from`].
    /// If both `system_prompt` and `system_prompt_file` are set, the inline value wins.
    #[serde(default)]
    pub system_prompt_file: Option<String>,

    /// Session storage configuration.
    #[serde(default)]
    pub session: Option<SessionConfig>,

    /// MCP (Model Context Protocol) configuration.
    #[serde(default)]
    pub mcp: Option<McpSettings>,

    /// Telegram bot configuration.
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,

    /// File logging configuration (rotation, directory, max files).
    #[serde(default)]
    pub logging: Option<LoggingConfig>,

    /// Reasoning effort level for thinking-capable models (e.g. DeepSeek V4 Pro).
    ///
    /// Accepted values (per DeepSeek docs): `"low"`, `"medium"`, `"high"`, `"max"`.
    /// Only applied when `provider = "deepseek"` and `model` is in `REASONING_MODELS`.
    /// Ignored for all other providers (a warning is emitted). Defaults to `"high"`
    /// internally when unset.
    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// CLI-only display preferences.
    #[serde(default)]
    pub cli: Option<CliConfig>,
}

/// CLI display preferences.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CliConfig {
    /// Display `reasoning_content` above the final answer in the REPL and on
    /// stderr in one-shot mode. Default: `true` (CLI is developer-facing).
    #[serde(default = "default_cli_show_reasoning")]
    pub show_reasoning: bool,
}

fn default_cli_show_reasoning() -> bool {
    true
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            show_reasoning: true,
        }
    }
}

/// Session storage configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SessionConfig {
    /// Database URL for session storage.
    ///
    /// Priority order:
    /// 1. `DATABASE_URL` environment variable (highest priority)
    /// 2. This field (`session.database_url` in config.toml)
    /// 3. Default: `sqlite:~/.config/synapse/sessions.db`
    #[serde(default)]
    pub database_url: Option<String>,

    /// Maximum number of sessions to keep.
    #[serde(default = "default_max_sessions")]
    pub max_sessions: u32,

    /// Delete sessions older than this many days.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    /// Enable automatic cleanup on startup.
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup: bool,
}

fn default_max_sessions() -> u32 {
    100
}

fn default_retention_days() -> u32 {
    90
}

fn default_auto_cleanup() -> bool {
    true
}

/// MCP (Model Context Protocol) settings.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct McpSettings {
    /// Path to MCP servers configuration file (JSON).
    ///
    /// Priority order:
    /// 1. `SYNAPSE_MCP_CONFIG` environment variable (highest priority)
    /// 2. This field (`mcp.config_path` in config.toml)
    /// 3. Default: `~/.config/synapse/mcp_servers.json`
    #[serde(default)]
    pub config_path: Option<String>,
}

/// Telegram bot configuration.
///
/// Secure by default: an empty `allowed_users` list rejects all users.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TelegramConfig {
    /// Bot token. Overridden by the `TELEGRAM_BOT_TOKEN` environment variable.
    #[serde(default)]
    pub token: Option<String>,
    /// Telegram user IDs allowed to interact with the bot.
    /// An empty list rejects all users (secure by default).
    #[serde(default)]
    pub allowed_users: Vec<u64>,
    /// Maximum number of sessions allowed per Telegram chat (default: 10).
    ///
    /// When the cap is exceeded during `/new`, the oldest session is automatically
    /// deleted before the new one is created.
    #[serde(default = "default_max_sessions_per_chat")]
    pub max_sessions_per_chat: u32,
    /// Prepend reasoning content as `<blockquote>` in outgoing Telegram messages.
    ///
    /// Default: `false` (privacy-preserving — Telegram messages can be screenshotted
    /// or forwarded). Set to `true` to show chain-of-thought in a blockquote above the
    /// answer. Reasoning is always persisted to SQLite regardless of this flag.
    #[serde(default = "default_telegram_show_reasoning")]
    pub show_reasoning: bool,
}

fn default_max_sessions_per_chat() -> u32 {
    10
}

fn default_telegram_show_reasoning() -> bool {
    false
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            token: None,
            allowed_users: vec![],
            max_sessions_per_chat: default_max_sessions_per_chat(),
            show_reasoning: false,
        }
    }
}

/// Log rotation strategy.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rotation {
    /// Rotate logs once per day.
    Daily,
    /// Rotate logs once per hour.
    Hourly,
    /// Never rotate logs.
    Never,
}

/// File-based logging configuration with rotation.
///
/// Deserialized from the `[logging]` section in `config.toml`.
/// Omit the section entirely to disable file logging (stdout only).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LoggingConfig {
    /// Directory to write log files to (relative or absolute path).
    #[serde(default = "default_log_directory")]
    pub directory: String,

    /// Maximum number of rotated log files to keep.
    /// Oldest files are deleted when this limit is exceeded.
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Rotation strategy: `daily`, `hourly`, or `never`.
    #[serde(default = "default_rotation")]
    pub rotation: Rotation,
}

fn default_log_directory() -> String {
    "logs".to_string()
}

fn default_max_files() -> usize {
    7
}

fn default_rotation() -> Rotation {
    Rotation::Daily
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            directory: default_log_directory(),
            max_files: default_max_files(),
            rotation: default_rotation(),
        }
    }
}

fn default_provider() -> String {
    "deepseek".to_string()
}

fn default_model() -> String {
    "deepseek-chat".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

impl Config {
    /// Load configuration from an explicit path or from the default search locations.
    ///
    /// Priority order:
    /// 1. `path` argument (e.g. from `--config` CLI flag) — error if the file is missing.
    /// 2. `./config.toml` (current directory).
    /// 3. `~/.config/synapse/config.toml` (user config directory).
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::IoError`] if the specified or found file cannot be read.
    /// Returns [`ConfigError::ParseError`] if the file is not valid TOML.
    /// Returns [`ConfigError::NotFound`] if no config file is found in the default locations.
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        // 1. Explicit path (highest priority) — delegate directly; IoError covers missing file.
        if let Some(p) = path {
            return Self::load_from(p);
        }

        // 2. Local directory
        let local = PathBuf::from("config.toml");
        if local.exists() {
            return Self::load_from(local);
        }

        // 3. User config directory (~/.config/synapse/)
        if let Some(home) = dirs::home_dir() {
            let user_config = home.join(".config/synapse/config.toml");
            if user_config.exists() {
                return Self::load_from(user_config);
            }
        }

        Err(ConfigError::NotFound)
    }

    /// Load configuration from a specific path.
    ///
    /// Reads the file at the given path and parses it as TOML.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::IoError`] if the file cannot be read.
    /// Returns [`ConfigError::ParseError`] if the file is not valid TOML.
    pub fn load_from(path: impl AsRef<std::path::Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        tracing::debug!(path = %path.display(), "config: loading from path");
        let content = std::fs::read_to_string(path).map_err(|source| ConfigError::IoError {
            path: path.to_path_buf(),
            source,
        })?;
        let mut config: Config =
            toml::from_str(&content).map_err(|source| ConfigError::ParseError {
                path: path.to_path_buf(),
                source,
            })?;
        config.resolve_system_prompt()?;
        Ok(config)
    }

    /// Resolve `system_prompt` from `system_prompt_file` if not already set inline.
    ///
    /// Priority: inline `system_prompt` wins over `system_prompt_file`.
    /// Whitespace-only file contents leave `system_prompt` as `None`.
    fn resolve_system_prompt(&mut self) -> Result<(), ConfigError> {
        if self.system_prompt.is_some() {
            return Ok(());
        }
        if let Some(ref path_str) = self.system_prompt_file {
            let path = PathBuf::from(path_str);
            let content =
                std::fs::read_to_string(&path).map_err(|source| ConfigError::IoError {
                    path: path.clone(),
                    source,
                })?;
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                self.system_prompt = Some(trimmed.to_string());
            }
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            system_prompt: None,
            system_prompt_file: None,
            session: None,
            mcp: None,
            telegram: None,
            logging: None,
            reasoning_effort: None,
            cli: None,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            database_url: None,
            max_sessions: default_max_sessions(),
            retention_days: default_retention_days(),
            auto_cleanup: default_auto_cleanup(),
        }
    }
}

#[cfg(test)]
mod tests;
