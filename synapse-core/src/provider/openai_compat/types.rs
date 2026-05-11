//! Serde request/response structs for OpenAI-compatible Chat Completions APIs.
//!
//! These types are shared between [`DeepSeekProvider`] and [`OpenAiProvider`]
//! and are intentionally kept private to the `openai_compat` module.

use serde::{Deserialize, Serialize};

/// SSE "[DONE]" marker sent by OpenAI-compatible streaming APIs.
pub(in super::super) const SSE_DONE_MARKER: &str = "[DONE]";

/// Enables thinking/reasoning mode on supporting models (e.g. DeepSeek V4 Pro).
///
/// Serialises as `{"type": "enabled"}`. Only included in request bodies when the
/// provider's `reasoning` field is `Some(_)`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(in super::super) struct ThinkingConfig {
    #[serde(rename = "type")]
    pub(in super::super) kind: String,
}

/// A single message in the API request body.
#[derive(Debug, Serialize)]
pub(in super::super) struct ApiMessage {
    /// Message role ("system", "user", "assistant", or "tool").
    pub(in super::super) role: String,
    /// Message content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) content: Option<String>,
    /// Tool calls (present in assistant messages with tool use).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) tool_calls: Option<Vec<OaiToolCall>>,
    /// Tool call ID (present in tool result messages).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) tool_call_id: Option<String>,
    /// Chain-of-thought reasoning content.
    ///
    /// Included on outbound assistant messages only when the same message also
    /// carries `tool_calls` (DeepSeek thinking-mode requirement). Omitted on
    /// text-only turns to save tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) reasoning_content: Option<String>,
}

/// Request body for a non-streaming Chat Completions API call.
#[derive(Debug, Serialize)]
pub(in super::super) struct ApiRequest {
    /// Model identifier.
    pub(in super::super) model: String,
    /// Conversation messages.
    pub(in super::super) messages: Vec<ApiMessage>,
    /// Maximum tokens to generate.
    pub(in super::super) max_tokens: u32,
    /// Optional tool definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) tools: Option<Vec<OaiTool>>,
    /// Tool choice strategy: `"auto"`, `"required"`, or `"none"`.
    /// Omitted when `None` so the API default (`"auto"`) applies implicitly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) tool_choice: Option<String>,
    /// Enables thinking mode (DeepSeek V4 Pro and other reasoning models).
    ///
    /// Only included when the provider has reasoning enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) thinking: Option<ThinkingConfig>,
    /// Reasoning effort level ("low", "medium", "high", "max").
    ///
    /// Only included when the provider has reasoning enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) reasoning_effort: Option<String>,
}

/// Request body for a streaming Chat Completions API call.
#[derive(Debug, Serialize)]
pub(in super::super) struct StreamingApiRequest {
    /// Model identifier.
    pub(in super::super) model: String,
    /// Conversation messages.
    pub(in super::super) messages: Vec<ApiMessage>,
    /// Maximum tokens to generate.
    pub(in super::super) max_tokens: u32,
    /// Must be `true` for streaming calls.
    pub(in super::super) stream: bool,
    /// Optional tool definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) tools: Option<Vec<OaiTool>>,
    /// Enables thinking mode (DeepSeek V4 Pro and other reasoning models).
    ///
    /// Only included when the provider has reasoning enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) thinking: Option<ThinkingConfig>,
    /// Reasoning effort level ("low", "medium", "high", "max").
    ///
    /// Only included when the provider has reasoning enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) reasoning_effort: Option<String>,
}

/// Tool definition in OpenAI-compatible format.
#[derive(Debug, Serialize)]
pub(in super::super) struct OaiTool {
    #[serde(rename = "type")]
    pub(in super::super) tool_type: String,
    pub(in super::super) function: OaiFunction,
}

/// Function definition within a tool.
#[derive(Debug, Serialize)]
pub(in super::super) struct OaiFunction {
    pub(in super::super) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in super::super) description: Option<String>,
    pub(in super::super) parameters: serde_json::Value,
}

/// Tool call in an API response message (also serialised when echoing history).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(in super::super) struct OaiToolCall {
    pub(in super::super) id: String,
    #[serde(rename = "type")]
    pub(in super::super) call_type: String,
    pub(in super::super) function: OaiToolCallFunction,
}

/// Function name + arguments within a tool call.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(in super::super) struct OaiToolCallFunction {
    pub(in super::super) name: String,
    pub(in super::super) arguments: String,
}

/// Response body from a Chat Completions API call.
#[derive(Debug, Deserialize)]
pub(in super::super) struct ApiResponse {
    pub(in super::super) choices: Vec<Choice>,
}

/// A choice in the completion response.
#[derive(Debug, Deserialize)]
pub(in super::super) struct Choice {
    pub(in super::super) message: ChoiceMessage,
}

/// Message content in a completion choice.
#[derive(Debug, Deserialize)]
pub(in super::super) struct ChoiceMessage {
    #[serde(default)]
    pub(in super::super) content: Option<String>,
    pub(in super::super) tool_calls: Option<Vec<OaiToolCall>>,
    /// Chain-of-thought reasoning produced by reasoning-capable models.
    #[serde(default)]
    pub(in super::super) reasoning_content: Option<String>,
}

/// Error response body from the API.
#[derive(Debug, Deserialize)]
pub(in super::super) struct ApiError {
    pub(in super::super) error: ErrorDetail,
}

/// Detail inside an API error response.
#[derive(Debug, Deserialize)]
pub(in super::super) struct ErrorDetail {
    pub(in super::super) message: String,
}

/// An SSE streaming response chunk.
#[derive(Debug, Deserialize)]
pub(in super::super) struct StreamChunk {
    pub(in super::super) choices: Vec<StreamChoice>,
}

/// A choice in a streaming response chunk.
#[derive(Debug, Deserialize)]
pub(in super::super) struct StreamChoice {
    pub(in super::super) delta: StreamDelta,
    /// Deserialized from the API response for serde completeness but not
    /// read at runtime. Suppressed with `#[allow(dead_code)]` to avoid a
    /// compiler warning while keeping the field available for future use.
    #[serde(default)]
    #[allow(dead_code)]
    pub(in super::super) finish_reason: Option<String>,
}

/// Delta content in a streaming choice.
#[derive(Debug, Deserialize)]
pub(in super::super) struct StreamDelta {
    #[serde(default)]
    pub(in super::super) content: Option<String>,
    /// Incremental reasoning/thinking token (DeepSeek V4 Pro in thinking mode).
    #[serde(default)]
    pub(in super::super) reasoning_content: Option<String>,
    /// Tool call delta chunks (present when the model invokes tools).
    /// Deserialized for completeness; not yet consumed at runtime.
    #[serde(default)]
    #[allow(dead_code)]
    pub(in super::super) tool_calls: Option<Vec<OaiToolCallDelta>>,
}

/// Incremental tool-call chunk in a streaming delta.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(in super::super) struct OaiToolCallDelta {
    pub(in super::super) index: usize,
    #[serde(default)]
    pub(in super::super) id: Option<String>,
    #[serde(default)]
    pub(in super::super) function: Option<OaiToolCallFunctionDelta>,
}

/// Partial function data inside a streaming tool-call delta.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(in super::super) struct OaiToolCallFunctionDelta {
    #[serde(default)]
    pub(in super::super) name: Option<String>,
    #[serde(default)]
    pub(in super::super) arguments: Option<String>,
}
