//! Anthropic Claude LLM provider.
//!
//! Implements the [`LlmProvider`] trait for Anthropic's Messages API,
//! enabling real Claude completions through the Synapse agent.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::message::{Message, Role};

/// Default max tokens for API responses.
const DEFAULT_MAX_TOKENS: u32 = 1024;

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
/// let provider = AnthropicProvider::new("sk-ant-...", "claude-3-5-sonnet-20241022");
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
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Anthropic API key
    /// * `model` - Model identifier (e.g., "claude-3-5-sonnet-20241022")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

/// Request body for Anthropic Messages API.
#[derive(Debug, Serialize)]
struct ApiRequest {
    /// Model identifier.
    model: String,
    /// Maximum tokens to generate.
    max_tokens: u32,
    /// Conversation messages (user/assistant only).
    messages: Vec<ApiMessage>,
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// A single message in the API request.
#[derive(Debug, Serialize)]
struct ApiMessage {
    /// Message role ("user" or "assistant").
    role: String,
    /// Message content.
    content: String,
}

/// Response body from Anthropic Messages API.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    /// Response content blocks.
    content: Vec<ContentBlock>,
}

/// Content block in API response.
#[derive(Debug, Deserialize)]
struct ContentBlock {
    /// Content type (e.g., "text").
    #[serde(rename = "type")]
    content_type: String,
    /// Text content (present for "text" type).
    text: Option<String>,
}

/// Error response from Anthropic API.
#[derive(Debug, Deserialize)]
struct ApiError {
    /// Error details.
    error: ErrorDetail,
}

/// Error detail from API error response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
struct ErrorDetail {
    /// Error type (e.g., "authentication_error").
    #[serde(rename = "type")]
    error_type: String,
    /// Human-readable error message.
    message: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        // Extract system messages to separate field
        let system_messages: Vec<&Message> =
            messages.iter().filter(|m| m.role == Role::System).collect();

        let system = if system_messages.is_empty() {
            None
        } else {
            Some(
                system_messages
                    .iter()
                    .map(|m| m.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            )
        };

        // Convert non-system messages to API format
        let api_messages: Vec<ApiMessage> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| ApiMessage {
                role: match m.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::System => unreachable!("System messages filtered above"),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = ApiRequest {
            model: self.model.clone(),
            max_tokens: DEFAULT_MAX_TOKENS,
            messages: api_messages,
            system,
        };

        let response = self
            .client
            .post(API_ENDPOINT)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();

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

        // Extract text content from response
        let content = api_response
            .content
            .iter()
            .filter(|block| block.content_type == "text")
            .filter_map(|block| block.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        Ok(Message::new(Role::Assistant, content))
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
                content: "Hello, Claude".to_string(),
            }],
            system: None,
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "claude-3-5-sonnet-20241022");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello, Claude");
        assert!(json.get("system").is_none());
    }

    #[test]
    fn test_api_request_serialization_with_system() {
        let request = ApiRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            max_tokens: 1024,
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            system: Some("You are a helpful assistant.".to_string()),
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

        // Extract system messages
        let system_messages: Vec<&Message> =
            messages.iter().filter(|m| m.role == Role::System).collect();

        assert_eq!(system_messages.len(), 1);
        assert_eq!(system_messages[0].content, "You are a helpful assistant.");

        // Non-system messages
        let non_system: Vec<&Message> =
            messages.iter().filter(|m| m.role != Role::System).collect();

        assert_eq!(non_system.len(), 1);
        assert_eq!(non_system[0].role, Role::User);
    }

    #[test]
    fn test_system_message_extraction_multiple() {
        let messages = [
            Message::new(Role::System, "System prompt 1"),
            Message::new(Role::System, "System prompt 2"),
            Message::new(Role::User, "Hello"),
        ];

        let system_messages: Vec<&Message> =
            messages.iter().filter(|m| m.role == Role::System).collect();

        let combined = system_messages
            .iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        assert_eq!(combined, "System prompt 1\n\nSystem prompt 2");
    }

    #[test]
    fn test_anthropic_provider_new() {
        let provider = AnthropicProvider::new("test-key", "test-model");

        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "test-model");
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
        // Verify AnthropicProvider implements the stream method
        // This is a compile-time check - if it compiles, the trait is implemented
        fn assert_stream_impl<T: LlmProvider>() {}
        assert_stream_impl::<AnthropicProvider>();
    }
}
