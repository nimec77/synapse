//! Streaming event types for LLM provider responses.
//!
//! This module defines the [`StreamEvent`] enum representing events
//! emitted during streaming LLM responses.

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
}
