//! DeepSeek LLM provider.
//!
//! Implements the [`LlmProvider`] trait for DeepSeek's OpenAI-compatible
//! Chat Completions API.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::message::{Message, Role};

/// Default max tokens for API responses.
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// DeepSeek Chat Completions API endpoint.
const API_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

/// DeepSeek LLM provider.
///
/// Sends messages to the DeepSeek Chat Completions API (OpenAI-compatible)
/// and returns responses.
///
/// # Examples
///
/// ```no_run
/// use synapse_core::provider::{DeepSeekProvider, LlmProvider};
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = DeepSeekProvider::new("sk-...", "deepseek-chat");
/// let messages = vec![Message::new(Role::User, "Hello, DeepSeek!")];
///
/// let response = provider.complete(&messages).await?;
/// println!("{}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct DeepSeekProvider {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// DeepSeek API key.
    api_key: String,
    /// Model identifier (e.g., "deepseek-chat").
    model: String,
}

impl DeepSeekProvider {
    /// Create a new DeepSeek provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - DeepSeek API key
    /// * `model` - Model identifier (e.g., "deepseek-chat")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

/// Request body for DeepSeek Chat Completions API (OpenAI-compatible).
#[derive(Debug, Serialize)]
struct ApiRequest {
    /// Model identifier.
    model: String,
    /// Conversation messages (including system messages).
    messages: Vec<ApiMessage>,
    /// Maximum tokens to generate.
    max_tokens: u32,
}

/// Request body for streaming Chat Completions API.
#[derive(Debug, Serialize)]
struct StreamingApiRequest {
    /// Model identifier.
    model: String,
    /// Conversation messages (including system messages).
    messages: Vec<ApiMessage>,
    /// Maximum tokens to generate.
    max_tokens: u32,
    /// Enable streaming mode.
    stream: bool,
}

/// A single message in the API request.
#[derive(Debug, Serialize)]
struct ApiMessage {
    /// Message role ("system", "user", or "assistant").
    role: String,
    /// Message content.
    content: String,
}

/// Response body from DeepSeek Chat Completions API.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    /// Response choices.
    choices: Vec<Choice>,
}

/// A choice in the API response.
#[derive(Debug, Deserialize)]
struct Choice {
    /// The message content.
    message: ChoiceMessage,
}

/// Message content in a choice.
#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    /// The generated content.
    content: String,
}

/// Error response from DeepSeek API.
#[derive(Debug, Deserialize)]
struct ApiError {
    /// Error details.
    error: ErrorDetail,
}

/// Error detail from API error response.
#[derive(Debug, Deserialize)]
struct ErrorDetail {
    /// Human-readable error message.
    message: String,
}

/// SSE streaming response chunk.
#[derive(Debug, Deserialize)]
struct StreamChunk {
    /// Response choices.
    choices: Vec<StreamChoice>,
}

/// A choice in the streaming response.
#[derive(Debug, Deserialize)]
struct StreamChoice {
    /// Delta content for this chunk.
    delta: StreamDelta,
    /// Reason the response finished (if complete).
    ///
    /// Currently unused but deserialized for potential future use.
    #[serde(default)]
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

/// Delta content in a streaming choice.
#[derive(Debug, Deserialize)]
struct StreamDelta {
    /// The content fragment (may be None on first/last chunks).
    content: Option<String>,
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        // Convert all messages to API format (system messages go in messages array)
        let api_messages: Vec<ApiMessage> = messages
            .iter()
            .map(|m| ApiMessage {
                role: match m.role {
                    Role::System => "system".to_string(),
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: DEFAULT_MAX_TOKENS,
        };

        let response = self
            .client
            .post(API_ENDPOINT)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            let error_body: ApiError = response.json().await.unwrap_or_else(|_| ApiError {
                error: ErrorDetail {
                    message: "Invalid API key".to_string(),
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

        // Extract content from first choice
        let content = api_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(Message::new(Role::Assistant, content))
    }

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        // Clone data needed for the async stream
        let messages = messages.to_vec();
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            // Convert all messages to API format
            let api_messages: Vec<ApiMessage> = messages
                .iter()
                .map(|m| ApiMessage {
                    role: match m.role {
                        Role::System => "system".to_string(),
                        Role::User => "user".to_string(),
                        Role::Assistant => "assistant".to_string(),
                    },
                    content: m.content.clone(),
                })
                .collect();

            let request = StreamingApiRequest {
                model,
                messages: api_messages,
                max_tokens: DEFAULT_MAX_TOKENS,
                stream: true,
            };

            // Send request
            let response = client
                .post(API_ENDPOINT)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    yield Err(ProviderError::RequestFailed(e.to_string()));
                    return;
                }
            };

            // Check for HTTP errors
            let status = response.status();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                yield Err(ProviderError::AuthenticationError(
                    "Invalid API key".to_string()
                ));
                return;
            }

            if !status.is_success() {
                yield Err(ProviderError::RequestFailed(
                    format!("HTTP {}", status)
                ));
                return;
            }

            // Parse SSE stream
            let mut stream = response.bytes_stream().eventsource();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        // Handle [DONE] marker
                        if event.data == "[DONE]" {
                            yield Ok(StreamEvent::Done);
                            return;
                        }

                        // Parse JSON delta
                        match serde_json::from_str::<StreamChunk>(&event.data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.first()
                                    && let Some(content) = &choice.delta.content
                                    && !content.is_empty()
                                {
                                    yield Ok(StreamEvent::TextDelta(content.clone()));
                                }
                            }
                            Err(e) => {
                                yield Err(ProviderError::ProviderError {
                                    message: format!("Failed to parse SSE: {}", e),
                                });
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ProviderError::RequestFailed(e.to_string()));
                        return;
                    }
                }
            }

            // Stream ended without [DONE] - still signal done
            yield Ok(StreamEvent::Done);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_provider_new() {
        let provider = DeepSeekProvider::new("test-key", "test-model");

        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "test-model");
    }

    #[test]
    fn test_api_request_serialization() {
        let request = ApiRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello, DeepSeek".to_string(),
            }],
            max_tokens: 1024,
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "deepseek-chat");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello, DeepSeek");
    }

    #[test]
    fn test_api_request_with_system_message() {
        let request = ApiRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![
                ApiMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                },
                ApiMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
            ],
            max_tokens: 1024,
        };

        let json = serde_json::to_value(&request).unwrap();

        // System message should be in messages array (not separate field)
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(
            json["messages"][0]["content"],
            "You are a helpful assistant."
        );
        assert_eq!(json["messages"][1]["role"], "user");
        assert_eq!(json["messages"][1]["content"], "Hello");
    }

    #[test]
    fn test_api_response_parsing() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "deepseek-chat",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 15,
                "total_tokens": 25
            }
        }"#;

        let response: ApiResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            "Hello! How can I help you today?"
        );
    }

    #[test]
    fn test_api_error_parsing() {
        let json = r#"{
            "error": {
                "message": "Incorrect API key provided",
                "type": "invalid_request_error",
                "code": "invalid_api_key"
            }
        }"#;

        let error: ApiError = serde_json::from_str(json).unwrap();

        assert_eq!(error.error.message, "Incorrect API key provided");
    }

    #[test]
    fn test_streaming_request_serialization() {
        let request = StreamingApiRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 1024,
            stream: true,
        };

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["model"], "deepseek-chat");
        assert_eq!(json["stream"], true);
        assert_eq!(json["max_tokens"], 1024);
    }

    #[test]
    fn test_parse_sse_text_delta() {
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "content": "Hello"
                    },
                    "finish_reason": null
                }
            ]
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
        assert!(chunk.choices[0].finish_reason.is_none());
    }

    #[test]
    fn test_parse_sse_done() {
        // The [DONE] marker is checked as a string before parsing
        let done_marker = "[DONE]";
        assert_eq!(done_marker, "[DONE]");

        // Also test parsing a final chunk with finish_reason
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }
            ]
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        assert!(chunk.choices[0].delta.content.is_none());
        assert_eq!(chunk.choices[0].finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_parse_sse_empty_content() {
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "content": ""
                    },
                    "finish_reason": null
                }
            ]
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        // Empty content should be filtered out by the streaming logic
        let content = chunk.choices[0].delta.content.as_deref().unwrap_or("");
        assert!(content.is_empty());
    }

    #[test]
    fn test_parse_sse_with_role() {
        // First event often has role but no content
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "role": "assistant"
                    },
                    "finish_reason": null
                }
            ]
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();

        // No content in first event
        assert!(chunk.choices[0].delta.content.is_none());
    }
}
