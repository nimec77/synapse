//! OpenAI LLM provider.
//!
//! Implements the [`LlmProvider`] trait for OpenAI's Chat Completions API.

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::{Message, Role, ToolCallData};

/// Default max tokens for API responses.
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// OpenAI Chat Completions API endpoint.
const API_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI LLM provider.
///
/// Sends messages to the OpenAI Chat Completions API and returns responses.
pub struct OpenAiProvider {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// OpenAI API key.
    api_key: String,
    /// Model identifier (e.g., "gpt-4o").
    model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Build API messages from conversation messages.
    fn build_api_messages(messages: &[Message]) -> Vec<ApiMessage> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system".to_string(),
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::Tool => "tool".to_string(),
                };

                ApiMessage {
                    role,
                    content: Some(m.content.clone()),
                    tool_calls: None,
                    tool_call_id: m.tool_call_id.clone(),
                }
            })
            .collect()
    }
}

/// Request body for OpenAI Chat Completions API.
#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
}

/// Request body for streaming Chat Completions API.
#[derive(Debug, Serialize)]
struct StreamingApiRequest {
    model: String,
    messages: Vec<ApiMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
}

/// Tool definition in OpenAI API format.
#[derive(Debug, Serialize)]
struct OpenAiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunction,
}

/// Function definition within a tool.
#[derive(Debug, Serialize)]
struct OpenAiFunction {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: serde_json::Value,
}

/// A single message in the API request.
#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

/// Tool call in API response.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiToolCallFunction,
}

/// Function call data.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAiToolCallFunction {
    name: String,
    arguments: String,
}

/// Response body from OpenAI Chat Completions API.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Vec<Choice>,
}

/// A choice in the API response.
#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

/// Message content in a choice.
#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

/// Error response from OpenAI API.
#[derive(Debug, Deserialize)]
struct ApiError {
    error: ErrorDetail,
}

/// Error detail from API error response.
#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
}

/// SSE streaming response chunk.
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

/// A choice in the streaming response.
#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    #[serde(default)]
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

/// Delta content in a streaming choice.
#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

/// Convert ToolDefinitions to OpenAI format.
fn to_openai_tools(tools: &[ToolDefinition]) -> Option<Vec<OpenAiTool>> {
    if tools.is_empty() {
        None
    } else {
        Some(
            tools
                .iter()
                .map(|t| OpenAiTool {
                    tool_type: "function".to_string(),
                    function: OpenAiFunction {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.input_schema.clone(),
                    },
                })
                .collect(),
        )
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        let api_messages = Self::build_api_messages(messages);

        let request = ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: DEFAULT_MAX_TOKENS,
            tools: None,
        };

        self.complete_request(&request).await
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        let api_messages = Self::build_api_messages(messages);

        let request = ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: DEFAULT_MAX_TOKENS,
            tools: to_openai_tools(tools),
        };

        self.complete_request(&request).await
    }

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        let messages = messages.to_vec();
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            let api_messages = OpenAiProvider::build_api_messages(&messages);

            let request = StreamingApiRequest {
                model,
                messages: api_messages,
                max_tokens: DEFAULT_MAX_TOKENS,
                stream: true,
                tools: None,
            };

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

            let mut stream = response.bytes_stream().eventsource();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(event) => {
                        if event.data == "[DONE]" {
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

            yield Ok(StreamEvent::Done);
        })
    }
}

impl OpenAiProvider {
    /// Send a complete request and parse the response.
    async fn complete_request(&self, request: &ApiRequest) -> Result<Message, ProviderError> {
        let response = self
            .client
            .post(API_ENDPOINT)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
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

        let choice = api_response
            .choices
            .first()
            .ok_or(ProviderError::ProviderError {
                message: "no choices in response".to_string(),
            })?;

        let content = choice.message.content.clone().unwrap_or_default();
        let mut msg = Message::new(Role::Assistant, content);

        // Parse tool calls if present
        if let Some(ref tool_calls) = choice.message.tool_calls {
            let parsed: Vec<ToolCallData> = tool_calls
                .iter()
                .map(|tc| {
                    let input: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::json!({}));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_new() {
        let provider = OpenAiProvider::new("test-key", "test-model");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "test-model");
    }

    #[test]
    fn test_api_request_serialization() {
        let request = ApiRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("Hello, OpenAI".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["max_tokens"], 1024);
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "Hello, OpenAI");
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_api_request_with_system_message() {
        let request = ApiRequest {
            model: "gpt-4o".to_string(),
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
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["messages"][0]["role"], "system");
        assert_eq!(
            json["messages"][0]["content"],
            "You are a helpful assistant."
        );
    }

    #[test]
    fn test_api_response_parsing() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4o",
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
            Some("Hello! How can I help you today?".to_string())
        );
    }

    #[test]
    fn test_api_error_parsing() {
        let json = r#"{
            "error": {
                "message": "Incorrect API key provided",
                "type": "invalid_request_error",
                "param": null,
                "code": "invalid_api_key"
            }
        }"#;

        let error: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error.message, "Incorrect API key provided");
    }

    #[test]
    fn test_streaming_request_serialization() {
        let request = StreamingApiRequest {
            model: "gpt-4o".to_string(),
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
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn test_parse_sse_text_delta() {
        let json = r#"{"id":"chatcmpl-123","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_parse_sse_done() {
        let done_marker = "[DONE]";
        assert_eq!(done_marker, "[DONE]");
    }

    #[test]
    fn test_complete_with_tools_serialization() {
        let tools = vec![OpenAiTool {
            tool_type: "function".to_string(),
            function: OpenAiFunction {
                name: "get_weather".to_string(),
                description: Some("Get weather".to_string()),
                parameters: serde_json::json!({"type": "object", "properties": {"location": {"type": "string"}}}),
            },
        }];

        let request = ApiRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("What's the weather?".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: Some(tools),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("tools").is_some());
        assert_eq!(json["tools"][0]["type"], "function");
        assert_eq!(json["tools"][0]["function"]["name"], "get_weather");
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

    #[test]
    fn test_tool_role_message_serialization() {
        let messages = vec![Message::tool_result("call_1", "Sunny, 20C")];
        let api_messages = OpenAiProvider::build_api_messages(&messages);

        assert_eq!(api_messages[0].role, "tool");
        assert_eq!(api_messages[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(api_messages[0].content, Some("Sunny, 20C".to_string()));
    }

    #[test]
    fn test_complete_with_tools_no_tools() {
        let request = ApiRequest {
            model: "gpt-4o".to_string(),
            messages: vec![ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            }],
            max_tokens: 1024,
            tools: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("tools").is_none());
    }
}
