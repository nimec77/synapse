//! MCP (Model Context Protocol) integration.
//!
//! Provides configuration loading, client management, tool discovery,
//! and tool execution for MCP servers.

mod protocol;
mod tools;

pub use protocol::{McpConfig, McpServerConfig, ToolDefinition};
pub use tools::McpClient;

use std::path::PathBuf;

/// Errors that occur during MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Failed to load or parse MCP configuration.
    #[error("MCP config error: {0}")]
    ConfigError(String),

    /// Failed to connect to an MCP server.
    #[error("MCP connection error for server '{server}': {message}")]
    ConnectionError {
        /// Name of the server that failed.
        server: String,
        /// Error message.
        message: String,
    },

    /// A tool call failed.
    #[error("MCP tool error: {0}")]
    ToolError(String),

    /// IO error during MCP operations.
    #[error("MCP IO error: {0}")]
    IoError(String),
}

/// Load MCP configuration from file.
///
/// Path resolution priority:
/// 1. `SYNAPSE_MCP_CONFIG` env var (highest)
/// 2. `config_path` parameter (from `mcp.config_path` in config.toml)
/// 3. `~/.config/synapse/mcp_servers.json` (default)
///
/// Returns `None` if no config file exists (graceful degradation).
///
/// # Errors
///
/// Returns [`McpError::ConfigError`] if the file exists but cannot be parsed.
pub fn load_mcp_config(config_path: Option<&str>) -> Result<Option<McpConfig>, McpError> {
    let path = resolve_config_path(config_path);

    match path {
        Some(p) if p.exists() => {
            let content = std::fs::read_to_string(&p)
                .map_err(|e| McpError::IoError(format!("failed to read {}: {}", p.display(), e)))?;
            let config: McpConfig = serde_json::from_str(&content).map_err(|e| {
                McpError::ConfigError(format!("failed to parse {}: {}", p.display(), e))
            })?;
            Ok(Some(config))
        }
        _ => Ok(None),
    }
}

/// Resolve the MCP config file path.
///
/// Priority:
/// 1. `SYNAPSE_MCP_CONFIG` env var
/// 2. `config_path` parameter (from config.toml)
/// 3. `~/.config/synapse/mcp_servers.json`
fn resolve_config_path(config_path: Option<&str>) -> Option<PathBuf> {
    // Priority 1: Environment variable
    if let Ok(path) = std::env::var("SYNAPSE_MCP_CONFIG")
        && !path.is_empty()
    {
        return Some(PathBuf::from(path));
    }

    // Priority 2: config.toml setting
    if let Some(path) = config_path {
        return Some(PathBuf::from(path));
    }

    // Priority 3: Default path
    dirs::home_dir().map(|home| home.join(".config/synapse/mcp_servers.json"))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Guards tests that mutate the `SYNAPSE_MCP_CONFIG` env var so they don't
    /// race against each other when cargo runs tests in parallel.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_mcp_error_display() {
        let err = McpError::ConfigError("bad config".to_string());
        assert_eq!(err.to_string(), "MCP config error: bad config");

        let err = McpError::ConnectionError {
            server: "fs-server".to_string(),
            message: "connection refused".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "MCP connection error for server 'fs-server': connection refused"
        );

        let err = McpError::ToolError("tool not found".to_string());
        assert_eq!(err.to_string(), "MCP tool error: tool not found");

        let err = McpError::IoError("file not found".to_string());
        assert_eq!(err.to_string(), "MCP IO error: file not found");
    }

    #[test]
    fn test_load_mcp_config_missing_file() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Set env to a non-existent path
        unsafe { std::env::set_var("SYNAPSE_MCP_CONFIG", "/tmp/nonexistent_synapse_mcp.json") };
        let result = load_mcp_config(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        unsafe { std::env::remove_var("SYNAPSE_MCP_CONFIG") };
    }

    #[test]
    fn test_load_mcp_config_valid_file() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let tmp_path = std::env::temp_dir().join("synapse_test_mcp_config.json");
        let config_json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
                    "env": {}
                }
            }
        }"#;
        std::fs::write(&tmp_path, config_json).unwrap();

        unsafe { std::env::set_var("SYNAPSE_MCP_CONFIG", tmp_path.to_str().unwrap()) };
        let result = load_mcp_config(None);
        assert!(result.is_ok());
        let config = result.unwrap().unwrap();
        assert!(config.mcp_servers.contains_key("filesystem"));

        // Cleanup
        unsafe { std::env::remove_var("SYNAPSE_MCP_CONFIG") };
        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_load_mcp_config_from_config_path() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let tmp_path = std::env::temp_dir().join("synapse_test_mcp_config_path.json");
        let config_json = r#"{
            "mcpServers": {
                "test-server": {
                    "command": "echo",
                    "args": ["hello"],
                    "env": {}
                }
            }
        }"#;
        std::fs::write(&tmp_path, config_json).unwrap();

        // Ensure env var is not set
        unsafe { std::env::remove_var("SYNAPSE_MCP_CONFIG") };

        let result = load_mcp_config(Some(tmp_path.to_str().unwrap()));
        assert!(result.is_ok());
        let config = result.unwrap().unwrap();
        assert!(config.mcp_servers.contains_key("test-server"));

        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_load_mcp_config_env_overrides_config_path() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let env_path = std::env::temp_dir().join("synapse_test_mcp_env.json");
        let config_json = r#"{
            "mcpServers": {
                "env-server": {
                    "command": "echo",
                    "args": ["env"],
                    "env": {}
                }
            }
        }"#;
        std::fs::write(&env_path, config_json).unwrap();

        unsafe { std::env::set_var("SYNAPSE_MCP_CONFIG", env_path.to_str().unwrap()) };
        let result = load_mcp_config(Some("/nonexistent/config_path.json"));
        assert!(result.is_ok());
        let config = result.unwrap().unwrap();
        assert!(config.mcp_servers.contains_key("env-server"));

        unsafe { std::env::remove_var("SYNAPSE_MCP_CONFIG") };
        let _ = std::fs::remove_file(&env_path);
    }
}
