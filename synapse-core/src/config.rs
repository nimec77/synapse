//! Configuration management for Synapse.
//!
//! Provides configuration loading from TOML files with support for
//! multiple file locations, environment variable overrides, and sensible defaults.

use std::path::PathBuf;

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

    /// System prompt prepended to every LLM conversation.
    ///
    /// Shapes the AI's personality and instructions across all interactions.
    /// Injected on-the-fly via `Agent::build_messages()` and never stored in the
    /// session database.
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Session storage configuration.
    #[serde(default)]
    pub session: Option<SessionConfig>,

    /// MCP (Model Context Protocol) configuration.
    #[serde(default)]
    pub mcp: Option<McpSettings>,

    /// Telegram bot configuration.
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,
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
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct TelegramConfig {
    /// Bot token. Overridden by the `TELEGRAM_BOT_TOKEN` environment variable.
    #[serde(default)]
    pub token: Option<String>,
    /// Telegram user IDs allowed to interact with the bot.
    /// An empty list rejects all users (secure by default).
    #[serde(default)]
    pub allowed_users: Vec<u64>,
}

fn default_provider() -> String {
    "deepseek".to_string()
}

fn default_model() -> String {
    "deepseek-chat".to_string()
}

impl Config {
    /// Load configuration from file system.
    ///
    /// Priority order:
    /// 1. SYNAPSE_CONFIG environment variable
    /// 2. ./config.toml (local directory)
    /// 3. ~/.config/synapse/config.toml (user config)
    ///
    /// Returns default config if no config file found.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::IoError`] if a found file cannot be read.
    /// Returns [`ConfigError::ParseError`] if a found file is not valid TOML.
    pub fn load() -> Result<Self, ConfigError> {
        // 1. Environment variable (highest priority)
        if let Ok(path) = std::env::var("SYNAPSE_CONFIG") {
            let p = PathBuf::from(&path);
            if p.exists() {
                return Self::load_from(p);
            }
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

        // No config file found, return defaults
        Ok(Self::default())
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
        let content = std::fs::read_to_string(path).map_err(|source| ConfigError::IoError {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&content).map_err(|source| ConfigError::ParseError {
            path: path.to_path_buf(),
            source,
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: None,
            model: default_model(),
            system_prompt: None,
            session: None,
            mcp: None,
            telegram: None,
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
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.provider, "deepseek");
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, "deepseek-chat");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml = r#"provider = "anthropic""#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, "deepseek-chat"); // default
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
    fn test_parse_partial_toml() {
        let toml = r#"model = "claude-3-opus""#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "deepseek"); // default
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, "claude-3-opus");
    }

    #[test]
    fn test_parse_empty_toml() {
        let toml = "";
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, "deepseek");
        assert_eq!(config.api_key, None);
        assert_eq!(config.model, "deepseek-chat");
    }

    #[test]
    fn test_load_from_path() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("synapse_test_config.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, r#"provider = "test-provider""#).unwrap();
        drop(file);

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config.provider, "test-provider");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_invalid_toml() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let path = dir.join("synapse_invalid_config.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, r#"invalid = ["#).unwrap();
        drop(file);

        let result = Config::load_from(&path);
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_load_from_nonexistent_file() {
        let result = Config::load_from("/nonexistent/path/config.toml");
        assert!(matches!(result, Err(ConfigError::IoError { .. })));
    }

    #[test]
    fn test_session_config_defaults() {
        let config = SessionConfig::default();
        assert_eq!(config.database_url, None);
        assert_eq!(config.max_sessions, 100);
        assert_eq!(config.retention_days, 90);
        assert!(config.auto_cleanup);
    }

    #[test]
    fn test_config_without_session_section() {
        let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.session.is_none());
    }

    #[test]
    fn test_config_with_session_section() {
        let toml = r#"
provider = "deepseek"
model = "deepseek-chat"

[session]
max_sessions = 50
retention_days = 30
auto_cleanup = false
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.session.is_some());
        let session = config.session.unwrap();
        assert_eq!(session.max_sessions, 50);
        assert_eq!(session.retention_days, 30);
        assert!(!session.auto_cleanup);
    }

    #[test]
    fn test_session_config_partial_defaults() {
        let toml = r#"
[session]
max_sessions = 200
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let session = config.session.unwrap();
        assert_eq!(session.max_sessions, 200);
        assert_eq!(session.retention_days, 90); // default
        assert!(session.auto_cleanup); // default
    }

    #[test]
    fn test_session_config_with_database_url() {
        let toml = r#"
[session]
database_url = "sqlite:/custom/path/sessions.db"
max_sessions = 50
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let session = config.session.unwrap();
        assert_eq!(
            session.database_url,
            Some("sqlite:/custom/path/sessions.db".to_string())
        );
        assert_eq!(session.max_sessions, 50);
    }

    #[test]
    fn test_config_without_mcp_section() {
        let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.mcp.is_none());
    }

    #[test]
    fn test_config_with_mcp_section() {
        let toml = r#"
provider = "deepseek"

[mcp]
config_path = "/custom/path/mcp_servers.json"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.mcp.is_some());
        let mcp = config.mcp.unwrap();
        assert_eq!(
            mcp.config_path,
            Some("/custom/path/mcp_servers.json".to_string())
        );
    }

    #[test]
    fn test_config_with_mcp_section_no_path() {
        let toml = r#"
[mcp]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.mcp.is_some());
        assert!(config.mcp.unwrap().config_path.is_none());
    }

    #[test]
    fn test_config_without_telegram_section() {
        let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.telegram.is_none());
    }

    #[test]
    fn test_config_with_telegram_section() {
        let toml = r#"
provider = "deepseek"

[telegram]
token = "123456:ABC-DEF"
allowed_users = [123456789, 987654321]
"#;
        let config: Config = toml::from_str(toml).unwrap();
        assert!(config.telegram.is_some());
        let tg = config.telegram.unwrap();
        assert_eq!(tg.token, Some("123456:ABC-DEF".to_string()));
        assert_eq!(tg.allowed_users, vec![123456789u64, 987654321u64]);
    }

    #[test]
    fn test_config_telegram_partial_defaults() {
        let toml = r#"
[telegram]
token = "bot-token-only"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let tg = config.telegram.unwrap();
        assert_eq!(tg.token, Some("bot-token-only".to_string()));
        assert!(tg.allowed_users.is_empty());
    }

    #[test]
    fn test_telegram_config_default() {
        let tg = TelegramConfig::default();
        assert!(tg.token.is_none());
        assert!(tg.allowed_users.is_empty());
    }

    #[test]
    fn test_config_with_system_prompt() {
        let toml = r#"system_prompt = "You are helpful.""#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.system_prompt, Some("You are helpful.".to_string()));
    }

    #[test]
    fn test_config_without_system_prompt() {
        let toml = r#"provider = "deepseek""#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.system_prompt, None);
    }

    #[test]
    fn test_config_default_system_prompt() {
        let config = Config::default();
        assert_eq!(config.system_prompt, None);
    }
}
