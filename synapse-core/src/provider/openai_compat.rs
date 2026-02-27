//! Shared types and logic for OpenAI-compatible Chat Completions APIs.
//!
//! Both [`DeepSeekProvider`](super::deepseek::DeepSeekProvider) and
//! [`OpenAiProvider`](super::openai::OpenAiProvider) implement the same
//! OpenAI-compatible wire format. This module centralises all shared serde
//! types and shared helper functions so that each provider module is reduced
//! to a thin wrapper that configures only its endpoint URL and model.

mod types;

#[cfg(test)]
mod tests;

use std::pin::Pin;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::{Stream, StreamExt};
use types::*;

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::{Message, Role, ToolCallData};

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
