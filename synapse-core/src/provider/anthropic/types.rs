//! Serde request/response structs for the Anthropic Messages API.
//!
//! These types are used exclusively by [`AnthropicProvider`] to serialize
//! API requests and deserialize API responses. They are intentionally kept
//! private to the `anthropic` module.
//!
//! [`AnthropicProvider`]: super::AnthropicProvider

use serde::{Deserialize, Serialize};

/// Request body for Anthropic Messages API.
#[derive(Debug, Serialize)]
pub(super) struct ApiRequest {
    /// Model identifier.
    pub(super) model: String,
    /// Maximum tokens to generate.
    pub(super) max_tokens: u32,
    /// Conversation messages (user/assistant only).
    pub(super) messages: Vec<ApiMessage>,
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) system: Option<String>,
    /// Optional tool definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tools: Option<Vec<AnthropicTool>>,
}

/// Tool definition in Anthropic API format.
#[derive(Debug, Serialize)]
pub(super) struct AnthropicTool {
    /// Tool name.
    pub(super) name: String,
    /// Tool description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) description: Option<String>,
    /// JSON Schema for the tool's input parameters.
    pub(super) input_schema: serde_json::Value,
}

/// Content that can be either a text string or an array of content blocks.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum ApiContent {
    /// Simple text content.
    Text(String),
    /// Array of content blocks (for tool results).
    Blocks(Vec<ContentBlock>),
}

/// A single message in the API request.
#[derive(Debug, Serialize)]
pub(super) struct ApiMessage {
    /// Message role ("user" or "assistant").
    pub(super) role: String,
    /// Message content (text or blocks).
    pub(super) content: ApiContent,
}

/// Response body from Anthropic Messages API.
#[derive(Debug, Deserialize)]
pub(super) struct ApiResponse {
    /// Response content blocks.
    pub(super) content: Vec<ContentBlock>,
}

/// Content block in API response.
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ContentBlock {
    /// Content type (e.g., "text", "tool_use", "tool_result").
    #[serde(rename = "type")]
    pub(super) content_type: String,
    /// Text content (present for "text" type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) text: Option<String>,
    /// Tool use ID (present for "tool_use" type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    /// Tool name (present for "tool_use" type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    /// Tool input (present for "tool_use" type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) input: Option<serde_json::Value>,
    /// Tool use ID reference (present for "tool_result" type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_use_id: Option<String>,
    /// Tool result content (present for "tool_result" type).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) content: Option<String>,
}

/// Error response from Anthropic API.
#[derive(Debug, Deserialize)]
pub(super) struct ApiError {
    /// Error details.
    pub(super) error: ErrorDetail,
}

/// Error detail from API error response.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for deserialization
pub(super) struct ErrorDetail {
    /// Error type (e.g., "authentication_error").
    #[serde(rename = "type")]
    pub(super) error_type: String,
    /// Human-readable error message.
    pub(super) message: String,
}
