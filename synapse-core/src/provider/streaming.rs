//! Streaming event types for LLM provider responses.
//!
//! This module defines the [`StreamEvent`] enum representing events
//! emitted during streaming LLM responses.

use crate::provider::ProviderError;

/// Events emitted during streaming LLM responses.
///
/// Each variant represents a different type of event that can occur
/// during a streaming response from an LLM provider.
///
/// # Examples
///
/// ```
/// use synapse_core::provider::StreamEvent;
///
/// // Text fragment from response
/// let delta = StreamEvent::TextDelta("Hello".to_string());
///
/// // Stream completed
/// let done = StreamEvent::Done;
/// ```
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A text fragment from the LLM response.
    ///
    /// These events are yielded as tokens arrive from the provider.
    /// The content is guaranteed to be non-empty.
    TextDelta(String),

    /// A tool call request from the LLM.
    ///
    /// This variant is reserved for future MCP integration (Phase 11).
    /// Currently not emitted by any provider implementation.
    ToolCall {
        /// Unique identifier for this tool call.
        id: String,
        /// Name of the tool to invoke.
        name: String,
        /// JSON input for the tool.
        input: serde_json::Value,
    },

    /// Result of a tool invocation.
    ///
    /// This variant is reserved for future MCP integration (Phase 11).
    /// Currently not emitted by any provider implementation.
    ToolResult {
        /// Tool call ID this result corresponds to.
        id: String,
        /// JSON output from the tool.
        output: serde_json::Value,
    },

    /// Stream completed successfully.
    ///
    /// This event signals the end of the stream. No more events
    /// will be yielded after this.
    Done,

    /// An error occurred during streaming.
    ///
    /// This event wraps a [`ProviderError`] for consistent error handling.
    /// The stream terminates after an error event.
    Error(ProviderError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_variants() {
        // TextDelta variant
        let text_delta = StreamEvent::TextDelta("Hello".to_string());
        assert!(matches!(text_delta, StreamEvent::TextDelta(s) if s == "Hello"));

        // ToolCall variant
        let tool_call = StreamEvent::ToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            input: serde_json::json!({"location": "London"}),
        };
        assert!(matches!(tool_call, StreamEvent::ToolCall { id, name, .. }
            if id == "call_123" && name == "get_weather"));

        // ToolResult variant
        let tool_result = StreamEvent::ToolResult {
            id: "call_123".to_string(),
            output: serde_json::json!({"temperature": 20}),
        };
        assert!(matches!(tool_result, StreamEvent::ToolResult { id, .. }
            if id == "call_123"));

        // Done variant
        let done = StreamEvent::Done;
        assert!(matches!(done, StreamEvent::Done));

        // Error variant
        let error = StreamEvent::Error(ProviderError::RequestFailed("timeout".to_string()));
        assert!(matches!(
            error,
            StreamEvent::Error(ProviderError::RequestFailed(_))
        ));
    }

    #[test]
    fn test_stream_event_debug() {
        let event = StreamEvent::TextDelta("test".to_string());
        let debug = format!("{:?}", event);
        assert!(debug.contains("TextDelta"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_stream_event_clone() {
        let original = StreamEvent::TextDelta("clone me".to_string());
        let cloned = original.clone();
        assert!(matches!(cloned, StreamEvent::TextDelta(s) if s == "clone me"));
    }
}
