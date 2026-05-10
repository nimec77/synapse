//! Streaming event types for LLM provider responses.
//!
//! This module defines the [`StreamEvent`] enum representing events
//! emitted during streaming LLM responses.

/// Events emitted during streaming LLM responses.
///
/// Each variant represents a different type of event that can occur
/// during a streaming response from an LLM provider.
///
/// This enum is `#[non_exhaustive]` so that future variants (e.g. for
/// Anthropic or OpenAI reasoning support) can be added without a breaking
/// change. Match arms should include a wildcard or named catch-all arm.
///
/// # Variants
///
/// - `TextDelta(String)` — incremental fragment of the user-visible answer.
/// - `ReasoningDelta(String)` — incremental fragment of chain-of-thought
///   reasoning (DeepSeek V4 Pro and other reasoning models). Renderers should
///   display this above / distinct from `TextDelta`.
/// - `Done` — stream complete; no more events will follow.
///
/// # Examples
///
/// ```
/// use synapse_core::provider::StreamEvent;
///
/// // Text fragment from response
/// let delta = StreamEvent::TextDelta("Hello".to_string());
///
/// // Reasoning fragment
/// let reasoning = StreamEvent::ReasoningDelta("Let me think...".to_string());
///
/// // Stream completed
/// let done = StreamEvent::Done;
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum StreamEvent {
    /// A text fragment from the LLM response (user-visible answer).
    ///
    /// These events are yielded as tokens arrive from the provider.
    /// The content is guaranteed to be non-empty.
    TextDelta(String),

    /// An incremental fragment of chain-of-thought reasoning.
    ///
    /// Emitted by reasoning-capable models (e.g. DeepSeek V4 Pro).
    /// Renderers should display this above or distinct from `TextDelta`
    /// (e.g. dim/italic in the REPL, hidden or `<blockquote>`-wrapped in
    /// Telegram).
    ReasoningDelta(String),

    /// Stream completed successfully.
    ///
    /// This event signals the end of the stream. No more events
    /// will be yielded after this.
    Done,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_variants() {
        // TextDelta variant
        let text_delta = StreamEvent::TextDelta("Hello".to_string());
        assert!(matches!(text_delta, StreamEvent::TextDelta(s) if s == "Hello"));

        // Done variant
        let done = StreamEvent::Done;
        assert!(matches!(done, StreamEvent::Done));
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

    #[test]
    fn test_stream_event_reasoning_delta() {
        // ReasoningDelta variant exists, implements Debug and Clone.
        let event = StreamEvent::ReasoningDelta("step 1: consider the problem".to_string());
        assert!(
            matches!(event, StreamEvent::ReasoningDelta(ref s) if s == "step 1: consider the problem")
        );

        // Clone
        let cloned = event.clone();
        assert!(
            matches!(cloned, StreamEvent::ReasoningDelta(ref s) if s == "step 1: consider the problem")
        );

        // Debug
        let debug = format!("{:?}", cloned);
        assert!(debug.contains("ReasoningDelta"));
        assert!(debug.contains("step 1"));
    }
}
