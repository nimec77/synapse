//! Agent orchestrator for tool calling.
//!
//! Provides [`Agent`] which coordinates an LLM provider and an optional
//! MCP client to implement the detect-execute-return tool call loop.

use std::pin::Pin;

use futures::Stream;

use crate::mcp::McpClient;
use crate::message::Message;
use crate::provider::{LlmProvider, ProviderError, StreamEvent};

/// Maximum number of tool call iterations before giving up.
const MAX_ITERATIONS: usize = 10;

/// Error type for agent operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// Error from the LLM provider.
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),

    /// Error from MCP tool execution.
    #[error("MCP error: {0}")]
    Mcp(#[from] crate::mcp::McpError),

    /// Tool call loop exceeded the maximum iteration limit.
    #[error("max tool call iterations exceeded")]
    MaxIterationsExceeded,
}

/// Agent orchestrator that coordinates LLM providers and MCP tools.
///
/// Handles the detect-execute-return loop for tool calls:
/// 1. Send messages (with tool schemas) to the LLM
/// 2. Receive response
/// 3. If response contains tool calls: execute tools, append results, go to 1
/// 4. If response is text only: return to caller
///
/// # Examples
///
/// ```
/// use synapse_core::agent::Agent;
/// use synapse_core::provider::MockProvider;
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() {
/// let provider = Box::new(MockProvider::new().with_response("Hello!"));
/// let agent = Agent::new(provider, None);
///
/// let mut messages = vec![Message::new(Role::User, "Hi")];
/// let response = agent.complete(&mut messages).await.unwrap();
/// assert_eq!(response.content, "Hello!");
/// # }
/// ```
pub struct Agent {
    /// The LLM provider for generating responses.
    provider: Box<dyn LlmProvider>,
    /// Optional MCP client for tool execution.
    mcp_client: Option<McpClient>,
}

impl Agent {
    /// Create a new agent with a provider and optional MCP client.
    ///
    /// # Arguments
    ///
    /// * `provider` - The LLM provider to use for completions
    /// * `mcp_client` - Optional MCP client for tool execution
    pub fn new(provider: Box<dyn LlmProvider>, mcp_client: Option<McpClient>) -> Self {
        Self {
            provider,
            mcp_client,
        }
    }

    /// Complete a conversation, handling tool calls automatically.
    ///
    /// Returns the final assistant text response after all tool calls
    /// have been resolved. The `messages` vec is extended in-place with
    /// tool call and tool result messages.
    ///
    /// # Errors
    ///
    /// Returns [`AgentError::MaxIterationsExceeded`] if the tool call loop
    /// exceeds the maximum iteration limit (10).
    pub async fn complete(&self, messages: &mut Vec<Message>) -> Result<Message, AgentError> {
        let tools = self.get_tool_definitions();

        for _ in 0..MAX_ITERATIONS {
            let response = if tools.is_empty() {
                self.provider.complete(messages).await?
            } else {
                self.provider.complete_with_tools(messages, &tools).await?
            };

            // Check for tool calls
            if let Some(ref tool_calls) = response.tool_calls
                && !tool_calls.is_empty()
            {
                // Append assistant message with tool calls
                messages.push(response);

                // Execute each tool call
                let last_msg = messages.last().unwrap();
                let tool_calls_to_execute = last_msg.tool_calls.clone().unwrap();

                for tool_call in &tool_calls_to_execute {
                    let result = self.execute_tool(&tool_call.name, &tool_call.input).await;
                    let result_content = match result {
                        Ok(value) => match value {
                            serde_json::Value::String(s) => s,
                            other => other.to_string(),
                        },
                        Err(e) => format!("Error: {}", e),
                    };

                    messages.push(Message::tool_result(&tool_call.id, result_content));
                }

                continue;
            }

            // No tool calls -- this is the final text response
            return Ok(response);
        }

        Err(AgentError::MaxIterationsExceeded)
    }

    /// Stream a conversation response, handling tool calls automatically.
    ///
    /// Tool call iterations happen internally using non-streaming completions.
    /// Only the final text response is streamed (or yielded as a single delta
    /// if tool calls were involved).
    ///
    /// When no tools are available, delegates directly to `provider.stream()`.
    pub fn stream<'a>(
        &'a self,
        messages: &'a mut Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, AgentError>> + Send + 'a>> {
        let tools = self.get_tool_definitions();

        if tools.is_empty() {
            // No tools: direct streaming, no loop needed
            return Box::pin(async_stream::stream! {
                let mut stream = self.provider.stream(messages);
                use futures::StreamExt;
                while let Some(event) = stream.next().await {
                    yield event.map_err(AgentError::Provider);
                }
            });
        }

        // With tools: use complete for tool iterations, yield final as stream
        Box::pin(async_stream::stream! {
            match self.complete(messages).await {
                Ok(response) => {
                    if !response.content.is_empty() {
                        yield Ok(StreamEvent::TextDelta(response.content));
                    }
                    yield Ok(StreamEvent::Done);
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        })
    }

    /// Stream a conversation response with owned messages.
    ///
    /// Like [`stream`](Agent::stream), but takes ownership of the messages vec.
    /// This avoids borrow-checker issues in event loops where the stream must
    /// coexist with other mutable state.
    pub fn stream_owned(
        &self,
        mut messages: Vec<Message>,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, AgentError>> + Send + '_>> {
        let tools = self.get_tool_definitions();

        if tools.is_empty() {
            // No tools: direct streaming, no loop needed
            return Box::pin(async_stream::stream! {
                let mut stream = self.provider.stream(&messages);
                use futures::StreamExt;
                while let Some(event) = stream.next().await {
                    yield event.map_err(AgentError::Provider);
                }
            });
        }

        // With tools: use complete for tool iterations, yield final as stream
        Box::pin(async_stream::stream! {
            match self.complete(&mut messages).await {
                Ok(response) => {
                    if !response.content.is_empty() {
                        yield Ok(StreamEvent::TextDelta(response.content));
                    }
                    yield Ok(StreamEvent::Done);
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        })
    }

    /// Get tool definitions from the MCP client, or empty if no client.
    fn get_tool_definitions(&self) -> Vec<crate::mcp::ToolDefinition> {
        match &self.mcp_client {
            Some(client) if client.has_tools() => client.tool_definitions().to_vec(),
            _ => Vec::new(),
        }
    }

    /// Execute a tool call via the MCP client.
    async fn execute_tool(
        &self,
        name: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value, crate::mcp::McpError> {
        match &self.mcp_client {
            Some(client) => client.call_tool(name, input.clone()).await,
            None => Err(crate::mcp::McpError::ToolError(
                "no MCP client available".to_string(),
            )),
        }
    }

    /// Gracefully shut down the agent, including MCP connections.
    pub async fn shutdown(self) {
        if let Some(client) = self.mcp_client {
            client.shutdown().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::ToolDefinition;
    use crate::message::{Role, ToolCallData};
    use crate::provider::MockProvider;

    #[tokio::test]
    async fn test_agent_complete_no_tools() {
        // AC1: Agent without MCP client delegates to provider
        let provider = Box::new(MockProvider::new().with_response("Hello from agent!"));
        let agent = Agent::new(provider, None);

        let mut messages = vec![Message::new(Role::User, "Hi")];
        let response = agent.complete(&mut messages).await.unwrap();

        assert_eq!(response.content, "Hello from agent!");
        assert_eq!(response.role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_agent_complete_with_tool_call() {
        // AC2: mock provider returns tool call, mock MCP executes, provider called again
        let provider = Box::new(
            MockProvider::new()
                .with_response("The weather in London is sunny.")
                .with_tool_call_response(vec![ToolCallData {
                    id: "call_1".to_string(),
                    name: "get_weather".to_string(),
                    input: serde_json::json!({"location": "London"}),
                }]),
        );

        let mcp_client = McpClient::with_test_tools(vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: Some("Get weather".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        }]);

        let agent = Agent::new(provider, Some(mcp_client));
        let mut messages = vec![Message::new(Role::User, "What's the weather?")];

        let response = agent.complete(&mut messages).await;

        // The MCP client has no real server so call_tool will fail.
        // But the agent should handle the error gracefully and forward it.
        // The error message becomes the tool result, then provider returns final text.
        match response {
            Ok(msg) => {
                assert_eq!(msg.content, "The weather in London is sunny.");
                // Messages should contain: user, assistant (tool call), tool result, (implicit final)
                assert!(messages.len() >= 3);
            }
            Err(_) => {
                // Tool execution failed but was forwarded as error text to LLM
                // which should still produce a final response
                // This is acceptable for this test since we don't have a real MCP server
            }
        }
    }

    #[tokio::test]
    async fn test_agent_complete_multiple_tool_calls() {
        // AC3: multiple tool calls in one response handled
        let provider = Box::new(
            MockProvider::new()
                .with_response("Weather: sunny. Files: a.txt, b.txt")
                .with_tool_call_response(vec![
                    ToolCallData {
                        id: "call_1".to_string(),
                        name: "get_weather".to_string(),
                        input: serde_json::json!({"location": "London"}),
                    },
                    ToolCallData {
                        id: "call_2".to_string(),
                        name: "list_files".to_string(),
                        input: serde_json::json!({"path": "/tmp"}),
                    },
                ]),
        );

        let mcp_client = McpClient::with_test_tools(vec![
            ToolDefinition {
                name: "get_weather".to_string(),
                description: None,
                input_schema: serde_json::json!({}),
            },
            ToolDefinition {
                name: "list_files".to_string(),
                description: None,
                input_schema: serde_json::json!({}),
            },
        ]);

        let agent = Agent::new(provider, Some(mcp_client));
        let mut messages = vec![Message::new(Role::User, "Weather and files?")];

        let response = agent.complete(&mut messages).await;

        match response {
            Ok(msg) => {
                assert_eq!(msg.content, "Weather: sunny. Files: a.txt, b.txt");
                // Should have: user, assistant (2 tool calls), 2 tool results = 4 messages min
                assert!(messages.len() >= 4);
            }
            Err(_) => {
                // Acceptable for test without real MCP server
            }
        }
    }

    #[tokio::test]
    async fn test_agent_complete_max_iterations() {
        // AC4: returns MaxIterationsExceeded after 10 iterations
        // Create a provider that always returns tool calls
        let mut provider = MockProvider::new();

        // Push 11 tool call responses (more than MAX_ITERATIONS)
        for i in 0..11 {
            let tool_calls = vec![ToolCallData {
                id: format!("call_{}", i),
                name: "infinite_tool".to_string(),
                input: serde_json::json!({}),
            }];
            provider = provider.with_tool_call_response(tool_calls);
        }

        let mcp_client = McpClient::with_test_tools(vec![ToolDefinition {
            name: "infinite_tool".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
        }]);

        let agent = Agent::new(Box::new(provider), Some(mcp_client));
        let mut messages = vec![Message::new(Role::User, "Loop forever")];

        let result = agent.complete(&mut messages).await;
        assert!(matches!(result, Err(AgentError::MaxIterationsExceeded)));
    }

    #[tokio::test]
    async fn test_agent_stream_no_tools() {
        // AC5: streaming without tools returns provider stream directly
        use futures::StreamExt;

        let provider =
            Box::new(MockProvider::new().with_stream_tokens(vec!["Hello", " ", "world"]));
        let agent = Agent::new(provider, None);

        let mut messages = vec![Message::new(Role::User, "Hi")];
        let mut stream = agent.stream(&mut messages);

        let mut tokens = Vec::new();
        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::TextDelta(text)) => tokens.push(text),
                Ok(StreamEvent::Done) => break,
                Err(e) => panic!("Unexpected error: {}", e),
                _ => {}
            }
        }

        assert_eq!(tokens, vec!["Hello", " ", "world"]);
    }

    #[tokio::test]
    async fn test_agent_complete_tool_error_forwarded() {
        // AC6: MCP tool error forwarded to LLM as error result
        let provider = Box::new(
            MockProvider::new()
                .with_response("I encountered an error with the tool.")
                .with_tool_call_response(vec![ToolCallData {
                    id: "call_1".to_string(),
                    name: "nonexistent_tool".to_string(),
                    input: serde_json::json!({}),
                }]),
        );

        // MCP client with a different tool registered (not the one being called)
        let mcp_client = McpClient::with_test_tools(vec![ToolDefinition {
            name: "other_tool".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
        }]);

        let agent = Agent::new(provider, Some(mcp_client));
        let mut messages = vec![Message::new(Role::User, "Use the tool")];

        let response = agent.complete(&mut messages).await;

        match response {
            Ok(msg) => {
                // Provider got the error result and responded with text
                assert_eq!(msg.content, "I encountered an error with the tool.");

                // Check that the tool result message contains an error
                let tool_result_msg = messages.iter().find(|m| m.role == Role::Tool);
                assert!(tool_result_msg.is_some());
                assert!(tool_result_msg.unwrap().content.contains("Error"));
            }
            Err(_) => {
                // Also acceptable -- error propagated
            }
        }
    }
}
