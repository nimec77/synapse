//! Shared types and logic for OpenAI-compatible Chat Completions APIs.
//!
//! Both [`DeepSeekProvider`](super::deepseek::DeepSeekProvider) and
//! [`OpenAiProvider`](super::openai::OpenAiProvider) implement the same
//! OpenAI-compatible wire format. This module centralises all shared serde
//! types and shared helper functions so that each provider module is reduced
//! to a thin wrapper that configures only its endpoint URL and model.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::{Message, Role, ToolCallData};

/// SSE "[DONE]" marker sent by OpenAI-compatible streaming APIs.
pub(super) const SSE_DONE_MARKER: &str = "[DONE]";

// ---------------------------------------------------------------------------
// Serde types (shared across DeepSeek and OpenAI providers)
// ---------------------------------------------------------------------------

/// A single message in the API request body.
#[derive(Debug, Serialize)]
pub(super) struct ApiMessage {
    /// Message role ("system", "user", "assistant", or "tool").
    pub(super) role: String,
    /// Message content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) content: Option<String>,
    /// Tool calls (present in assistant messages with tool use).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_calls: Option<Vec<OaiToolCall>>,
    /// Tool call ID (present in tool result messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_call_id: Option<String>,
}

/// Request body for a non-streaming Chat Completions API call.
#[derive(Debug, Serialize)]
pub(super) struct ApiRequest {
    /// Model identifier.
    pub(super) model: String,
    /// Conversation messages.
    pub(super) messages: Vec<ApiMessage>,
    /// Maximum tokens to generate.
    pub(super) max_tokens: u32,
    /// Optional tool definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tools: Option<Vec<OaiTool>>,
    /// Tool choice strategy: `"auto"`, `"required"`, or `"none"`.
    /// Omitted when `None` so the API default (`"auto"`) applies implicitly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<String>,
}

/// Request body for a streaming Chat Completions API call.
#[derive(Debug, Serialize)]
pub(super) struct StreamingApiRequest {
    /// Model identifier.
    pub(super) model: String,
    /// Conversation messages.
    pub(super) messages: Vec<ApiMessage>,
    /// Maximum tokens to generate.
    pub(super) max_tokens: u32,
    /// Must be `true` for streaming calls.
    pub(super) stream: bool,
    /// Optional tool definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tools: Option<Vec<OaiTool>>,
}

/// Tool definition in OpenAI-compatible format.
#[derive(Debug, Serialize)]
pub(super) struct OaiTool {
    #[serde(rename = "type")]
    pub(super) tool_type: String,
    pub(super) function: OaiFunction,
}

/// Function definition within a tool.
#[derive(Debug, Serialize)]
pub(super) struct OaiFunction {
    pub(super) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) description: Option<String>,
    pub(super) parameters: serde_json::Value,
}

/// Tool call in an API response message (also serialised when echoing history).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct OaiToolCall {
    pub(super) id: String,
    #[serde(rename = "type")]
    pub(super) call_type: String,
    pub(super) function: OaiToolCallFunction,
}

/// Function name + arguments within a tool call.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(super) struct OaiToolCallFunction {
    pub(super) name: String,
    pub(super) arguments: String,
}

/// Response body from a Chat Completions API call.
#[derive(Debug, Deserialize)]
pub(super) struct ApiResponse {
    pub(super) choices: Vec<Choice>,
}

/// A choice in the completion response.
#[derive(Debug, Deserialize)]
pub(super) struct Choice {
    pub(super) message: ChoiceMessage,
}

/// Message content in a completion choice.
#[derive(Debug, Deserialize)]
pub(super) struct ChoiceMessage {
    #[serde(default)]
    pub(super) content: Option<String>,
    pub(super) tool_calls: Option<Vec<OaiToolCall>>,
}

/// Error response body from the API.
#[derive(Debug, Deserialize)]
pub(super) struct ApiError {
    pub(super) error: ErrorDetail,
}

/// Detail inside an API error response.
#[derive(Debug, Deserialize)]
pub(super) struct ErrorDetail {
    pub(super) message: String,
}

/// An SSE streaming response chunk.
#[derive(Debug, Deserialize)]
pub(super) struct StreamChunk {
    pub(super) choices: Vec<StreamChoice>,
}

/// A choice in a streaming response chunk.
#[derive(Debug, Deserialize)]
pub(super) struct StreamChoice {
    pub(super) delta: StreamDelta,
    /// Deserialized from the API response for serde completeness but not
    /// read at runtime. Suppressed with `#[allow(dead_code)]` to avoid a
    /// compiler warning while keeping the field available for future use.
    #[serde(default)]
    #[allow(dead_code)]
    pub(super) finish_reason: Option<String>,
}

/// Delta content in a streaming choice.
#[derive(Debug, Deserialize)]
pub(super) struct StreamDelta {
    pub(super) content: Option<String>,
}

// ---------------------------------------------------------------------------
// Shared helper functions
// ---------------------------------------------------------------------------

/// Convert a slice of [`Message`]s to the OpenAI-compatible wire format.
pub(super) fn build_api_messages(messages: &[Message]) -> Vec<ApiMessage> {
    messages
        .iter()
        .map(|m| {
            let role = m.role.as_str().to_string();

            let tool_calls = m
                .tool_calls
                .as_ref()
                .filter(|tc| !tc.is_empty())
                .map(|tcs| {
                    tcs.iter()
                        .map(|tc| OaiToolCall {
                            id: tc.id.clone(),
                            call_type: "function".to_string(),
                            function: OaiToolCallFunction {
                                name: tc.name.clone(),
                                arguments: tc.input.to_string(),
                            },
                        })
                        .collect()
                });

            ApiMessage {
                role,
                content: Some(m.content.clone()),
                tool_calls,
                tool_call_id: m.tool_call_id.clone(),
            }
        })
        .collect()
}

/// Send a non-streaming completion request and parse the response into a [`Message`].
pub(super) async fn complete_request(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    request: &ApiRequest,
) -> Result<Message, ProviderError> {
    tracing::debug!(endpoint, "openai_compat: POST complete request");
    let response = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(request)
        .send()
        .await
        .map_err(|e| ProviderError::RequestFailed(e.to_string()))?;

    let status = response.status();
    tracing::debug!(
        status = status.as_u16(),
        "openai_compat: complete response status"
    );

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

    let choice = api_response
        .choices
        .first()
        .ok_or(ProviderError::ProviderError {
            message: "no choices in response".to_string(),
        })?;

    let content = choice.message.content.clone().unwrap_or_default();
    let mut msg = Message::new(Role::Assistant, content);

    if let Some(ref tool_calls) = choice.message.tool_calls {
        let parsed: Vec<ToolCallData> = tool_calls
            .iter()
            .map(|tc| {
                let input: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
                ToolCallData {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    input,
                }
            })
            .collect();
        if !parsed.is_empty() {
            msg.tool_calls = Some(parsed);
        }
    }

    Ok(msg)
}

/// Convert a slice of [`ToolDefinition`]s to the OpenAI-compatible tool format.
///
/// Returns `None` when the input slice is empty so that `tools` can be omitted
/// from the serialized request body.
pub(super) fn to_oai_tools(tools: &[ToolDefinition]) -> Option<Vec<OaiTool>> {
    if tools.is_empty() {
        None
    } else {
        Some(
            tools
                .iter()
                .map(|t| OaiTool {
                    tool_type: "function".to_string(),
                    function: OaiFunction {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.input_schema.clone(),
                    },
                })
                .collect(),
        )
    }
}

/// Stream SSE tokens from an OpenAI-compatible endpoint.
///
/// Returns a pinned, owned stream so callers do not need to hold a reference
/// to the provider. Yields [`StreamEvent::TextDelta`] for each non-empty token
/// and [`StreamEvent::Done`] when the stream ends.
pub(super) fn stream_sse(
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>> {
    Box::pin(async_stream::stream! {
        let api_messages = build_api_messages(&messages);

        let request = StreamingApiRequest {
            model,
            messages: api_messages,
            max_tokens,
            stream: true,
            tools: None,
        };

        let response = client
            .post(&endpoint)
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

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            yield Err(ProviderError::AuthenticationError(
                "Invalid API key".to_string(),
            ));
            return;
        }

        if !status.is_success() {
            yield Err(ProviderError::RequestFailed(format!("HTTP {}", status)));
            return;
        }

        tracing::debug!(endpoint, "openai_compat: SSE stream started");
        let mut sse_stream = response.bytes_stream().eventsource();

        while let Some(event) = sse_stream.next().await {
            match event {
                Ok(event) => {
                    if event.data == SSE_DONE_MARKER {
                        tracing::debug!(endpoint, "openai_compat: SSE stream ended");
                        yield Ok(StreamEvent::Done);
                        return;
                    }

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

        // Stream ended without [DONE] – still signal completion.
        tracing::debug!(endpoint, "openai_compat: SSE stream ended");
        yield Ok(StreamEvent::Done);
    })
}

// ---------------------------------------------------------------------------
// Generic OpenAI-compatible provider struct
// ---------------------------------------------------------------------------

/// Generic provider for OpenAI-compatible Chat Completions APIs.
///
/// Holds the base URL, API key, model, and max tokens. Implements all
/// `LlmProvider` methods using the shared helpers in this module.
pub(super) struct OpenAiCompatProvider {
    pub(super) client: reqwest::Client,
    pub(super) base_url: String,
    pub(super) api_key: String,
    pub(super) model: String,
    pub(super) max_tokens: u32,
}

impl OpenAiCompatProvider {
    /// Create a new [`OpenAiCompatProvider`].
    pub(super) fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            max_tokens,
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        let api_messages = build_api_messages(messages);
        let request = ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: self.max_tokens,
            tools: None,
            tool_choice: None,
        };
        complete_request(&self.client, &self.base_url, &self.api_key, &request).await
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        let api_messages = build_api_messages(messages);
        let request = ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: self.max_tokens,
            tools: to_oai_tools(tools),
            tool_choice: if tools.is_empty() {
                None
            } else {
                Some("auto".to_string())
            },
        };
        complete_request(&self.client, &self.base_url, &self.api_key, &request).await
    }

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        stream_sse(
            self.client.clone(),
            self.base_url.clone(),
            self.api_key.clone(),
            self.model.clone(),
            messages.to_vec(),
            self.max_tokens,
        )
    }
}

// ---------------------------------------------------------------------------
// Tests for shared serde types and functions (migrated from deepseek/openai)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::ToolCallData;

    // -- ApiRequest serialisation --

    #[test]
    fn test_api_request_serialization() {
        let request = ApiRequest {
            model: "test-model".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: None,
            tool_choice: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello");
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_api_request_with_system_message() {
        let request = ApiRequest {
            model: "test-model".to_string(),
            messages: vec![
                ApiMessage {
                    role: "system".to_string(),
                    content: Some("You are a helpful assistant.".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
                ApiMessage {
                    role: "user".to_string(),
                    content: Some("Hello".to_string()),
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            max_tokens: 1024,
            tools: None,
            tool_choice: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(
            json["messages"][0]["content"],
            "You are a helpful assistant."
        );
        assert_eq!(json["messages"][1]["role"], "user");
        assert_eq!(json["messages"][1]["content"], "Hello");
    }

    #[test]
    fn test_streaming_request_serialization() {
        let request = StreamingApiRequest {
            model: "test-model".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            stream: true,
            tools: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["stream"], true);
        assert_eq!(json["max_tokens"], 1024);
    }

    // -- ApiResponse deserialisation --

    #[test]
    fn test_api_response_parsing() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello! How can I help you today?"
                    },
                    "finish_reason": "stop"
                }
            ]
        }"#;

        let response: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.content,
            Some("Hello! How can I help you today?".to_string())
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

    // -- SSE chunk deserialisation --

    #[test]
    fn test_parse_sse_text_delta() {
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "delta": {"content": "Hello"},
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
        // The [DONE] marker is checked as a string before JSON parsing.
        assert_eq!(SSE_DONE_MARKER, "[DONE]");

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
                    "delta": {"content": ""},
                    "finish_reason": null
                }
            ]
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let content = chunk.choices[0].delta.content.as_deref().unwrap_or("");
        assert!(content.is_empty());
    }

    #[test]
    fn test_parse_sse_with_role() {
        // First SSE event often has role but no content.
        let json = r#"{
            "id": "chatcmpl-123",
            "choices": [
                {
                    "index": 0,
                    "delta": {"role": "assistant"},
                    "finish_reason": null
                }
            ]
        }"#;

        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.choices[0].delta.content.is_none());
    }

    // -- Tool-related serialisation --

    #[test]
    fn test_complete_with_tools_serialization() {
        let tools = vec![OaiTool {
            tool_type: "function".to_string(),
            function: OaiFunction {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {"location": {"type": "string"}}
                }),
            },
        }];

        let request = ApiRequest {
            model: "test-model".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("What's the weather?".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: Some(tools),
            tool_choice: Some("auto".to_string()),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("tools").is_some());
        assert_eq!(json["tools"][0]["type"], "function");
        assert_eq!(json["tools"][0]["function"]["name"], "get_weather");
        assert_eq!(json["tool_choice"], "auto");
    }

    #[test]
    fn test_complete_with_tools_no_tools() {
        let request = ApiRequest {
            model: "test-model".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: None,
            tool_choice: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_api_request_tool_choice_absent_without_tools() {
        let request = ApiRequest {
            model: "test-model".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: None,
            tool_choice: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(
            json.get("tool_choice").is_none(),
            "tool_choice must be absent when no tools"
        );
    }

    #[test]
    fn test_tool_call_response_parsing() {
        let json = r#"{
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"London\"}"
                        }
                    }]
                }
            }]
        }"#;

        let response: ApiResponse = serde_json::from_str(json).unwrap();
        let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].function.name, "get_weather");
    }

    // -- build_api_messages helper --

    #[test]
    fn test_tool_role_message_serialization() {
        let messages = vec![Message::tool_result("call_1", "Sunny, 20C")];
        let api_messages = build_api_messages(&messages);

        assert_eq!(api_messages[0].role, "tool");
        assert_eq!(api_messages[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(api_messages[0].content, Some("Sunny, 20C".to_string()));
    }

    #[test]
    fn test_assistant_tool_call_message_serialization() {
        let mut assistant_msg = Message::new(Role::Assistant, "");
        assistant_msg.tool_calls = Some(vec![ToolCallData {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: serde_json::json!({"location": "London"}),
        }]);

        let messages = vec![
            Message::new(Role::User, "What's the weather?"),
            assistant_msg,
        ];

        let api_messages = build_api_messages(&messages);
        assert_eq!(api_messages.len(), 2);

        assert_eq!(api_messages[1].role, "assistant");
        let tool_calls = api_messages[1].tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].call_type, "function");
        assert_eq!(tool_calls[0].function.name, "get_weather");

        let args: serde_json::Value =
            serde_json::from_str(&tool_calls[0].function.arguments).unwrap();
        assert_eq!(args["location"], "London");
    }

    // -- to_oai_tools helper --

    #[test]
    fn test_to_oai_tools_empty() {
        let result = to_oai_tools(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_to_oai_tools_conversion() {
        let tools = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({"type": "object"}),
        }];
        let result = to_oai_tools(&tools).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_type, "function");
        assert_eq!(result[0].function.name, "test_tool");
    }
}
