//! MCP client and tool registry.
//!
//! Provides [`McpClient`] which manages connections to MCP servers,
//! tool discovery, and tool execution.

use std::collections::HashMap;
use std::process::Stdio;

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use rmcp::transport::TokioChildProcess;

use super::McpError;
use super::protocol::{McpConfig, ToolDefinition};

/// A connected MCP server client.
struct RunningClient {
    /// The rmcp client handle.
    client: rmcp::service::RunningService<rmcp::RoleClient, ()>,
}

/// Manages connections to MCP servers and provides tool execution.
///
/// Handles tool discovery, registration, and routing tool calls
/// to the appropriate MCP server.
pub struct McpClient {
    /// Connected server clients, keyed by server name.
    servers: HashMap<String, RunningClient>,
    /// Unified tool registry: tool name -> server name.
    tool_registry: HashMap<String, String>,
    /// All discovered tool definitions.
    tool_definitions: Vec<ToolDefinition>,
}

impl McpClient {
    /// Create a new MCP client from configuration.
    ///
    /// Spawns child processes for each configured server, connects via stdio,
    /// and discovers available tools. Servers that fail to start are logged
    /// as warnings but do not prevent initialization.
    pub async fn new(config: &McpConfig) -> Result<Self, McpError> {
        let mut servers = HashMap::new();
        let mut tool_registry = HashMap::new();
        let mut tool_definitions = Vec::new();

        for (name, server_config) in &config.mcp_servers {
            match Self::connect_server(name, server_config).await {
                Ok((client, tools)) => {
                    // Register all discovered tools
                    for tool in &tools {
                        tool_registry.insert(tool.name.clone(), name.clone());
                    }
                    tool_definitions.extend(tools);
                    servers.insert(name.clone(), RunningClient { client });
                }
                Err(e) => {
                    eprintln!("Warning: MCP server '{}' failed to start: {}", name, e);
                }
            }
        }

        Ok(Self {
            servers,
            tool_registry,
            tool_definitions,
        })
    }

    /// Connect to a single MCP server and discover its tools.
    async fn connect_server(
        name: &str,
        config: &super::protocol::McpServerConfig,
    ) -> Result<
        (
            rmcp::service::RunningService<rmcp::RoleClient, ()>,
            Vec<ToolDefinition>,
        ),
        McpError,
    > {
        // Build the child process command
        let mut cmd = tokio::process::Command::new(&config.command);
        cmd.args(&config.args);
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Spawn child process and create transport (stderr suppressed to avoid TUI corruption)
        let (transport, _stderr) = TokioChildProcess::builder(cmd)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| McpError::ConnectionError {
                server: name.to_string(),
                message: format!("failed to spawn process: {}", e),
            })?;

        // Connect via rmcp
        let client =
            ().serve(transport)
                .await
                .map_err(|e| McpError::ConnectionError {
                    server: name.to_string(),
                    message: format!("failed to connect: {}", e),
                })?;

        // Discover tools
        let tools_result =
            client
                .list_tools(None)
                .await
                .map_err(|e| McpError::ConnectionError {
                    server: name.to_string(),
                    message: format!("failed to list tools: {}", e),
                })?;

        // Convert to our ToolDefinition format
        let tools: Vec<ToolDefinition> = tools_result
            .tools
            .into_iter()
            .map(|t| ToolDefinition {
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                input_schema: serde_json::to_value(&t.input_schema)
                    .unwrap_or(serde_json::json!({})),
            })
            .collect();

        Ok((client, tools))
    }

    /// Create an MCP client with no servers (for testing).
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            servers: HashMap::new(),
            tool_registry: HashMap::new(),
            tool_definitions: Vec::new(),
        }
    }

    /// Create an MCP client with pre-registered tool definitions (for testing).
    #[cfg(test)]
    pub fn with_test_tools(tools: Vec<ToolDefinition>) -> Self {
        let mut tool_registry = HashMap::new();
        for tool in &tools {
            tool_registry.insert(tool.name.clone(), "test-server".to_string());
        }
        Self {
            servers: HashMap::new(),
            tool_registry,
            tool_definitions: tools,
        }
    }

    /// Execute a tool call on the appropriate MCP server.
    ///
    /// Routes the call to the server that registered the tool.
    ///
    /// # Errors
    ///
    /// Returns [`McpError::ToolError`] if the tool is not found or execution fails.
    pub async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let server_name = self
            .tool_registry
            .get(name)
            .ok_or_else(|| McpError::ToolError(format!("unknown tool: {}", name)))?;

        let server = self.servers.get(server_name).ok_or_else(|| {
            McpError::ToolError(format!("server '{}' not connected", server_name))
        })?;

        let arguments = if let serde_json::Value::Object(map) = input {
            Some(map)
        } else {
            None
        };

        let result = server
            .client
            .call_tool(CallToolRequestParams {
                name: std::borrow::Cow::Owned(name.to_string()),
                arguments,
                meta: None,
                task: None,
            })
            .await
            .map_err(|e| McpError::ToolError(format!("tool call failed: {}", e)))?;

        // Extract text content from the tool result
        let content: Vec<String> = result
            .content
            .iter()
            .filter_map(|c| c.raw.as_text().map(|t| t.text.clone()))
            .collect();

        Ok(serde_json::Value::String(content.join("\n")))
    }

    /// Get all discovered tool definitions.
    pub fn tool_definitions(&self) -> &[ToolDefinition] {
        &self.tool_definitions
    }

    /// Check if any tools are available.
    pub fn has_tools(&self) -> bool {
        !self.tool_definitions.is_empty()
    }

    /// Gracefully shut down all MCP server connections.
    pub async fn shutdown(self) {
        for (_name, server) in self.servers {
            let _ = server.client.cancel().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_client_no_servers() {
        let client = McpClient::empty();
        assert!(!client.has_tools());
        assert!(client.tool_definitions().is_empty());
    }

    #[tokio::test]
    async fn test_call_tool_unknown_name() {
        let client = McpClient::empty();
        let result = client
            .call_tool("nonexistent_tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ToolError(msg) => assert!(msg.contains("unknown tool")),
            other => panic!("Expected ToolError, got: {:?}", other),
        }
    }

    #[test]
    fn test_has_tools_empty() {
        let client = McpClient::empty();
        assert!(!client.has_tools());
    }

    #[test]
    fn test_has_tools_populated() {
        let tools = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let client = McpClient::with_test_tools(tools);
        assert!(client.has_tools());
        assert_eq!(client.tool_definitions().len(), 1);
        assert_eq!(client.tool_definitions()[0].name, "test_tool");
    }
}
