//! Anthropic Claude LLM provider.
//!
//! Implements the [`LlmProvider`] trait for Anthropic's Messages API,
//! enabling real Claude completions through the Synapse agent.
//!
//! Serde request/response structs live in the [`types`] submodule.

mod types;

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::{Message, Role, ToolCallData};
use types::{
    AnthropicTool, ApiContent, ApiError, ApiMessage, ApiRequest, ApiResponse, ContentBlock,
    ErrorDetail,
};

/// Anthropic API version header value.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Messages API endpoint.
const API_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// Anthropic Claude provider.
///
/// Sends messages to the Anthropic Messages API and returns Claude's responses.
///
/// # Examples
///
/// ```no_run
/// use synapse_core::provider::{AnthropicProvider, LlmProvider};
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = AnthropicProvider::new("sk-ant-...", "claude-3-5-sonnet-20241022", 4096);
/// let messages = vec![Message::new(Role::User, "Hello, Claude!")];
///
/// let response = provider.complete(&messages).await?;
/// println!("{}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct AnthropicProvider {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// Anthropic API key.
    api_key: String,
    /// Model identifier (e.g., "claude-3-5-sonnet-20241022").
    model: String,
    /// Maximum tokens to generate in API responses.
    max_tokens: u32,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Anthropic API key
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022")
    /// * `max_tokens` - Maximum tokens to generate in API responses
    pub fn new(api_key: impl Into<String>, model: impl Into<String>, max_tokens: u32) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            max_tokens,
        }
    }

    /// Build API messages from conversation messages, handling Role::Tool translation.
    fn build_api_messages(messages: &[Message]) -> Vec<ApiMessage> {
        messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                if m.role == Role::Tool {
                    // Anthropic handles tool results as user messages with tool_result content blocks
                    let tool_call_id = m.tool_call_id.clone().unwrap_or_default();
                    ApiMessage {
                        role: "user".to_string(),
                        content: ApiContent::Blocks(vec![ContentBlock {
                            content_type: "tool_result".to_string(),
                            text: None,
                            id: None,
                            name: None,
                            input: None,
                            tool_use_id: Some(tool_call_id),
                            content: Some(m.content.clone()),
                        }]),
                    }
                } else if let Some(tool_calls) = m.tool_calls.as_ref()
                    && m.role == Role::Assistant
                    && !tool_calls.is_empty()
                {
                    // Assistant message with tool calls: serialize as content blocks
                    let mut blocks = Vec::new();

                    // Include text content if non-empty
                    if !m.content.is_empty() {
                        blocks.push(ContentBlock {
                            content_type: "text".to_string(),
                            text: Some(m.content.clone()),
                            id: None,
                            name: None,
                            input: None,
                            tool_use_id: None,
                            content: None,
                        });
                    }

                    // Add tool_use blocks
                    for tc in tool_calls {
                        blocks.push(ContentBlock {
                            content_type: "tool_use".to_string(),
                            text: None,
                            id: Some(tc.id.clone()),
                            name: Some(tc.name.clone()),
                            input: Some(tc.input.clone()),
                            tool_use_id: None,
                            content: None,
                        });
                    }

                    ApiMessage {
                        role: "assistant".to_string(),
                        content: ApiContent::Blocks(blocks),
                    }
                } else {
                    ApiMessage {
                        role: match m.role {
                            Role::User => "user".to_string(),
                            Role::Assistant => "assistant".to_string(),
                            _ => "user".to_string(),
                        },
                        content: ApiContent::Text(m.content.clone()),
                    }
                }
            })
            .collect()
    }

    /// Extract system prompt from messages.
    fn extract_system(messages: &[Message]) -> Option<String> {
        let system_messages: Vec<&Message> =
            messages.iter().filter(|m| m.role == Role::System).collect();
        if system_messages.is_empty() {
            None
        } else {
            Some(
                system_messages
                    .iter()
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            )
        }
    }

    /// Send request and parse response.
    async fn send_request(&self, request: &ApiRequest) -> Result<Message, ProviderError> {
        tracing::debug!(endpoint = API_ENDPOINT, "anthropic: POST complete request");
        let response = self
            .client
            .post(API_ENDPOINT)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();
        tracing::debug!(status = status.as_u16(), "anthropic: response status");

        if status == reqwest::StatusCode::UNAUTHORIZED {
            let error_body: ApiError = response.json().await.unwrap_or_else(|_| ApiError {
                error: ErrorDetail {
                    error_type: "authentication_error".to_string(),
                    message: "invalid x-api-key".to_string(),
                },
            });
            return Err(ProviderError::AuthenticationError(error_body.error.message));
        }

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(ProviderError::RequestFailed(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let api_response: ApiResponse =
            response
                .json()
                .await
                .map_err(|e| ProviderError::ProviderError {
                    message: format!("failed to parse response: {}", e),
                })?;

        // Check for tool_use blocks
        let tool_calls: Vec<ToolCallData> = api_response
            .content
            .iter()
            .filter(|block| block.content_type == "tool_use")
            .filter_map(|block| {
                let id = block.id.clone()?;
                let name = block.name.clone()?;
                let input = block.input.clone().unwrap_or(serde_json::json!({}));
                Some(ToolCallData { id, name, input })
            })
            .collect();

        // Extract text content
        let text_content = api_response
            .content
            .iter()
            .filter(|block| block.content_type == "text")
            .filter_map(|block| block.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        let mut msg = Message::new(Role::Assistant, text_content);
        if !tool_calls.is_empty() {
            msg.tool_calls = Some(tool_calls);
        }

        Ok(msg)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        let system = Self::extract_system(messages);
        let api_messages = Self::build_api_messages(messages);

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: api_messages,
            system,
            tools: None,
        };

        self.send_request(&request).await
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        let system = Self::extract_system(messages);
        let api_messages = Self::build_api_messages(messages);

        let api_tools = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .map(|t| AnthropicTool {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        input_schema: t.input_schema.clone(),
                    })
                    .collect(),
            )
        };

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: api_messages,
            system,
            tools: api_tools,
        };

        self.send_request(&request).await
    }

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        // Clone messages for the async stream
        let messages = messages.to_vec();

        Box::pin(async_stream::stream! {
            // Fallback: call complete() and yield as single delta
            match self.complete(&messages).await {
                Ok(msg) => {
                    yield Ok(StreamEvent::TextDelta(msg.content));
                    yield Ok(StreamEvent::Done);
                }
                Err(e) => {
                    yield Err(e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_request_serialization() {
        let request = ApiRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1024,
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: ApiContent::Text("Hello, Claude".to_string()),
            }],
            system: None,
            tools: None,
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello, Claude");
        assert!(json.get("system").is_none());
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_api_request_serialization_with_system() {
        let request = ApiRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1024,
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: ApiContent::Text("Hello".to_string()),
            }],
            system: Some("You are a helpful assistant.".to_string()),
            tools: None,
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["system"], "You are a helpful assistant.");
    }

    #[test]
    fn test_api_response_parsing() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "Hello! How can I help you today?"}
            ]
        }"#;

        let response: ApiResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].content_type, "text");
        assert_eq!(
            response.content[0].text,
            Some("Hello! How can I help you today?".to_string())
        );
    }

    #[test]
    fn test_api_response_parsing_multiple_blocks() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "First part. "},
                {"type": "text", "text": "Second part."}
            ]
        }"#;

        let response: ApiResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.content.len(), 2);
    }

    #[test]
    fn test_system_message_extraction() {
        let messages = [
            Message::new(Role::System, "You are a helpful assistant."),
            Message::new(Role::User, "Hello"),
        ];

        let system = AnthropicProvider::extract_system(&messages);
        assert_eq!(system, Some("You are a helpful assistant.".to_string()));

        let api_messages = AnthropicProvider::build_api_messages(&messages);
        assert_eq!(api_messages.len(), 1);
    }

    #[test]
    fn test_system_message_extraction_multiple() {
        let messages = [
            Message::new(Role::System, "System prompt 1"),
            Message::new(Role::System, "System prompt 2"),
            Message::new(Role::User, "Hello"),
        ];

        let system = AnthropicProvider::extract_system(&messages);
        assert_eq!(
            system,
            Some("System prompt 1\n\nSystem prompt 2".to_string())
        );
    }

    #[test]
    fn test_anthropic_provider_new() {
        let provider = AnthropicProvider::new("test-key", "test-model", 4096);

        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "test-model");
        assert_eq!(provider.max_tokens, 4096);
    }

    #[test]
    fn test_api_error_parsing() {
        let json = r#"{
            "error": {
                "type": "authentication_error",
                "message": "invalid x-api-key"
            }
        }"#;

        let error: ApiError = serde_json::from_str(json).unwrap();

        assert_eq!(error.error.error_type, "authentication_error");
        assert_eq!(error.error.message, "invalid x-api-key");
    }

    #[test]
    fn test_anthropic_provider_implements_stream() {
        fn assert_stream_impl<T: LlmProvider>() {}
        assert_stream_impl::<AnthropicProvider>();
    }

    #[test]
    fn test_complete_with_tools_serialization() {
        let tools = vec![AnthropicTool {
            name: "get_weather".to_string(),
            description: Some("Get weather for a location".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"location": {"type": "string"}},
                "required": ["location"]
            }),
        }];

        let request = ApiRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1024,
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: ApiContent::Text("What's the weather?".to_string()),
            }],
            system: None,
            tools: Some(tools),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("tools").is_some());
        assert_eq!(json["tools"][0]["name"], "get_weather");
        assert_eq!(
            json["tools"][0]["description"],
            "Get weather for a location"
        );
        assert!(json["tools"][0]["input_schema"].is_object());
    }

    #[test]
    fn test_tool_call_response_parsing() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "I'll check the weather."},
                {"type": "tool_use", "id": "call_1", "name": "get_weather", "input": {"location": "London"}}
            ]
        }"#;

        let response: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.content.len(), 2);

        // First block is text
        assert_eq!(response.content[0].content_type, "text");

        // Second block is tool_use
        assert_eq!(response.content[1].content_type, "tool_use");
        assert_eq!(response.content[1].id, Some("call_1".to_string()));
        assert_eq!(response.content[1].name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_tool_role_message_serialization() {
        let messages = vec![
            Message::new(Role::User, "What's the weather?"),
            Message::tool_result("call_1", "Sunny, 20C"),
        ];

        let api_messages = AnthropicProvider::build_api_messages(&messages);
        assert_eq!(api_messages.len(), 2);

        // Tool result is serialized as user role with tool_result content block
        assert_eq!(api_messages[1].role, "user");
        if let ApiContent::Blocks(blocks) = &api_messages[1].content {
            assert_eq!(blocks[0].content_type, "tool_result");
            assert_eq!(blocks[0].tool_use_id, Some("call_1".to_string()));
            assert_eq!(blocks[0].content, Some("Sunny, 20C".to_string()));
        } else {
            panic!("Expected Blocks content for tool result");
        }
    }

    #[test]
    fn test_complete_with_tools_no_tools() {
        let request = ApiRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1024,
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: ApiContent::Text("Hello".to_string()),
            }],
            system: None,
            tools: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_assistant_tool_call_message_serialization() {
        let mut assistant_msg = Message::new(Role::Assistant, "I'll check the weather.");
        assistant_msg.tool_calls = Some(vec![ToolCallData {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: serde_json::json!({"location": "London"}),
        }]);

        let messages = vec![
            Message::new(Role::User, "What's the weather?"),
            assistant_msg,
        ];

        let api_messages = AnthropicProvider::build_api_messages(&messages);
        assert_eq!(api_messages.len(), 2);

        // Assistant message with tool calls should have content blocks
        assert_eq!(api_messages[1].role, "assistant");
        if let ApiContent::Blocks(blocks) = &api_messages[1].content {
            assert_eq!(blocks.len(), 2);

            // First block is text
            assert_eq!(blocks[0].content_type, "text");
            assert_eq!(blocks[0].text, Some("I'll check the weather.".to_string()));

            // Second block is tool_use
            assert_eq!(blocks[1].content_type, "tool_use");
            assert_eq!(blocks[1].id, Some("call_1".to_string()));
            assert_eq!(blocks[1].name, Some("get_weather".to_string()));
            assert_eq!(
                blocks[1].input,
                Some(serde_json::json!({"location": "London"}))
            );
        } else {
            panic!("Expected Blocks content for assistant tool call message");
        }
    }
}
