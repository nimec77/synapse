//! MCP protocol types and configuration.
//!
//! Provides types for MCP server configuration and tool definitions
//! in the standard format compatible with Claude Desktop and Windsurf.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Configuration for a single MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to execute to start the server.
    pub command: String,
    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set for the server process.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Top-level MCP configuration file format.
///
/// Compatible with Claude Desktop / Windsurf standard format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Map of server names to their configurations.
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// A tool definition in provider-agnostic format.
///
/// Each provider serializes this to its own API format:
/// - Anthropic: `{ name, description, input_schema }`
/// - OpenAI/DeepSeek: `{ type: "function", function: { name, description, parameters } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name.
    pub name: String,
    /// Optional description of the tool.
    pub description: Option<String>,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_parse() {
        let json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
                    "env": {}
                }
            }
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);

        let fs_server = config.mcp_servers.get("filesystem").unwrap();
        assert_eq!(fs_server.command, "npx");
        assert_eq!(fs_server.args.len(), 3);
        assert!(fs_server.env.is_empty());
    }

    #[test]
    fn test_mcp_config_empty() {
        let json = r#"{"mcpServers": {}}"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_mcp_config_with_env() {
        let json = r#"{
            "mcpServers": {
                "test-server": {
                    "command": "/usr/bin/test-server",
                    "args": ["--port", "3000"],
                    "env": {
                        "API_KEY": "secret123",
                        "DEBUG": "true"
                    }
                }
            }
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        let server = config.mcp_servers.get("test-server").unwrap();
        assert_eq!(server.command, "/usr/bin/test-server");
        assert_eq!(server.args, vec!["--port", "3000"]);
        assert_eq!(server.env.len(), 2);
        assert_eq!(server.env.get("API_KEY").unwrap(), "secret123");
        assert_eq!(server.env.get("DEBUG").unwrap(), "true");
    }

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "get_weather".to_string(),
            description: Some("Get weather for a location".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                },
                "required": ["location"]
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: ToolDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, "get_weather");
        assert_eq!(
            deserialized.description,
            Some("Get weather for a location".to_string())
        );
        assert_eq!(
            deserialized.input_schema["properties"]["location"]["type"],
            "string"
        );
    }

    #[test]
    fn test_mcp_config_multiple_servers() {
        let json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
                },
                "web-search": {
                    "command": "python",
                    "args": ["-m", "mcp_server_web_search"]
                }
            }
        }"#;

        let config: McpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.mcp_servers.len(), 2);
        assert!(config.mcp_servers.contains_key("filesystem"));
        assert!(config.mcp_servers.contains_key("web-search"));
    }
}
