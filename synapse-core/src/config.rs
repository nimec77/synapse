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
}
