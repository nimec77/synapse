//! Mock LLM provider for testing.
//!
//! Provides [`MockProvider`], a configurable mock implementation
//! of [`LlmProvider`] for unit and integration testing.

use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::Stream;

use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::{Message, Role, ToolCallData};

/// A mock LLM provider for testing.
///
/// Returns configurable responses. If no responses are configured,
/// returns a default response.
///
/// # Examples
///
/// ```
/// use synapse_core::provider::{LlmProvider, MockProvider};
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() {
/// let provider = MockProvider::new()
///     .with_response("Hello from mock!");
/// let messages = vec![Message::new(Role::User, "Hi")];
///
/// let response = provider.complete(&messages).await.unwrap();
/// assert_eq!(response.content, "Hello from mock!");
/// # }
/// ```
#[derive(Debug, Default)]
pub struct MockProvider {
    responses: Mutex<Vec<Message>>,
    stream_tokens: Mutex<Vec<String>>,
}

impl MockProvider {
    /// Create a new mock provider with no predefined responses.
    ///
    /// When no responses are configured, `complete()` returns
    /// a default "Mock response" message.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a response to be returned on the next call to `complete`.
    ///
    /// Responses are returned in LIFO order (last added = first returned).
    /// This allows for intuitive test setup where the most recently
    /// added response is returned first.
    ///
    /// # Arguments
    ///
    /// * `content` - The content of the response message
    ///
    /// # Examples
    ///
    /// ```
    /// use synapse_core::provider::{LlmProvider, MockProvider};
    /// use synapse_core::message::{Message, Role};
    ///
    /// # async fn example() {
    /// let provider = MockProvider::new()
    ///     .with_response("First")
    ///     .with_response("Second");
    ///
    /// let messages = vec![Message::new(Role::User, "Test")];
    ///
    /// // "Second" is returned first (LIFO)
    /// let r1 = provider.complete(&messages).await.unwrap();
    /// assert_eq!(r1.content, "Second");
    ///
    /// let r2 = provider.complete(&messages).await.unwrap();
    /// assert_eq!(r2.content, "First");
    /// # }
    /// ```
    #[must_use]
    pub fn with_response(self, content: impl Into<String>) -> Self {
        // Lock acquisition failure indicates a bug in test code
        // (mutex poisoned from a panic in another test thread).
        // We use a match to handle this gracefully in test contexts.
        match self.responses.lock() {
            Ok(mut responses) => {
                responses.push(Message::new(Role::Assistant, content));
            }
            Err(poisoned) => {
                // In test context, recover from poisoned mutex
                let mut responses = poisoned.into_inner();
                responses.push(Message::new(Role::Assistant, content));
            }
        }
        self
    }

    /// Add a tool call response to be returned on the next call to `complete`.
    ///
    /// The response will have `Role::Assistant` with the specified tool calls.
    /// This enables testing the agent loop without real providers.
    ///
    /// # Arguments
    ///
    /// * `tool_calls` - Tool calls the mock "assistant" wants to invoke
    ///
    /// # Examples
    ///
    /// ```
    /// use synapse_core::provider::MockProvider;
    /// use synapse_core::message::ToolCallData;
    ///
    /// let provider = MockProvider::new()
    ///     .with_tool_call_response(vec![ToolCallData {
    ///         id: "call_1".to_string(),
    ///         name: "get_weather".to_string(),
    ///         input: serde_json::json!({"location": "London"}),
    ///     }])
    ///     .with_response("The weather is sunny.");
    /// ```
    #[must_use]
    pub fn with_tool_call_response(self, tool_calls: Vec<ToolCallData>) -> Self {
        let mut msg = Message::new(Role::Assistant, "");
        msg.tool_calls = Some(tool_calls);

        match self.responses.lock() {
            Ok(mut responses) => {
                responses.push(msg);
            }
            Err(poisoned) => {
                let mut responses = poisoned.into_inner();
                responses.push(msg);
            }
        }
        self
    }

    /// Configure tokens to yield when streaming.
    ///
    /// When streaming is called, each token is yielded as a `TextDelta`
    /// event, followed by a `Done` event.
    ///
    /// If no tokens are configured, streaming will fall back to calling
    /// `complete()` and yielding the full response as a single `TextDelta`.
    ///
    /// # Arguments
    ///
    /// * `tokens` - Tokens to yield during streaming
    ///
    /// # Examples
    ///
    /// ```
    /// use synapse_core::provider::MockProvider;
    ///
    /// let provider = MockProvider::new()
    ///     .with_stream_tokens(vec!["Hello", " ", "world", "!"]);
    /// ```
    #[must_use]
    pub fn with_stream_tokens(self, tokens: Vec<&str>) -> Self {
        match self.stream_tokens.lock() {
            Ok(mut stream_tokens) => {
                *stream_tokens = tokens.into_iter().map(|s| s.to_string()).collect();
            }
            Err(poisoned) => {
                let mut stream_tokens = poisoned.into_inner();
                *stream_tokens = tokens.into_iter().map(|s| s.to_string()).collect();
            }
        }
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _messages: &[Message]) -> Result<Message, ProviderError> {
        let mut responses = match self.responses.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(response) = responses.pop() {
            Ok(response)
        } else {
            Ok(Message::new(Role::Assistant, "Mock response"))
        }
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        // MockProvider uses the same response queue for both complete and complete_with_tools.
        self.complete(messages).await
    }

    fn stream(
        &self,
        _messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        // Get configured tokens
        let tokens: Vec<String> = match self.stream_tokens.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };

        // Get fallback response if tokens are empty
        let fallback_content: Option<String> = if tokens.is_empty() {
            let mut responses = match self.responses.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };

            let response = if let Some(response) = responses.pop() {
                response
            } else {
                Message::new(Role::Assistant, "Mock response")
            };
            Some(response.content)
        } else {
            None
        };

        Box::pin(async_stream::stream! {
            if let Some(content) = fallback_content {
                // Fallback: yield the complete response as single delta
                yield Ok(StreamEvent::TextDelta(content));
                yield Ok(StreamEvent::Done);
            } else {
                // Yield each token as a TextDelta
                for token in tokens {
                    yield Ok(StreamEvent::TextDelta(token));
                }
                yield Ok(StreamEvent::Done);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider_default_response() {
        let provider = MockProvider::new();
        let messages = vec![Message::new(Role::User, "Hello")];

        let response = provider.complete(&messages).await.unwrap();

        assert_eq!(response.role, Role::Assistant);
        assert_eq!(response.content, "Mock response");
    }

    #[tokio::test]
    async fn test_mock_provider_configured_response() {
        let provider = MockProvider::new().with_response("Custom response");
        let messages = vec![Message::new(Role::User, "Hello")];

        let response = provider.complete(&messages).await.unwrap();

        assert_eq!(response.role, Role::Assistant);
        assert_eq!(response.content, "Custom response");
    }

    #[tokio::test]
    async fn test_mock_provider_multiple_responses() {
        let provider = MockProvider::new()
            .with_response("First")
            .with_response("Second");
        let messages = vec![Message::new(Role::User, "Hello")];

        // LIFO order: Second returned first
        let r1 = provider.complete(&messages).await.unwrap();
        assert_eq!(r1.content, "Second");

        let r2 = provider.complete(&messages).await.unwrap();
        assert_eq!(r2.content, "First");

        // Falls back to default
        let r3 = provider.complete(&messages).await.unwrap();
        assert_eq!(r3.content, "Mock response");
    }

    #[tokio::test]
    async fn test_mock_provider_with_string() {
        let provider = MockProvider::new().with_response(String::from("String response"));
        let messages = vec![Message::new(Role::User, "Test")];

        let response = provider.complete(&messages).await.unwrap();

        assert_eq!(response.content, "String response");
    }

    #[tokio::test]
    async fn test_llmprovider_is_object_safe() {
        // Verify that LlmProvider can be used as a trait object
        let provider: Box<dyn LlmProvider> = Box::new(MockProvider::new());
        let messages = vec![Message::new(Role::User, "Test")];

        let response = provider.complete(&messages).await.unwrap();

        assert_eq!(response.role, Role::Assistant);
    }

    #[tokio::test]
    async fn test_mock_stream_tokens() {
        use futures::StreamExt;

        let provider = MockProvider::new().with_stream_tokens(vec!["Hello", " ", "world", "!"]);
        let messages = vec![Message::new(Role::User, "Test")];

        let mut stream = provider.stream(&messages);
        let mut tokens = Vec::new();

        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::TextDelta(text)) => tokens.push(text),
                Ok(StreamEvent::Done) => break,
                _ => {}
            }
        }

        assert_eq!(tokens, vec!["Hello", " ", "world", "!"]);
    }

    #[tokio::test]
    async fn test_mock_stream_fallback() {
        use futures::StreamExt;

        // No stream tokens configured, should fall back to complete()
        let provider = MockProvider::new().with_response("Fallback response");
        let messages = vec![Message::new(Role::User, "Test")];

        let mut stream = provider.stream(&messages);
        let mut tokens = Vec::new();
        let mut done_received = false;

        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::TextDelta(text)) => tokens.push(text),
                Ok(StreamEvent::Done) => {
                    done_received = true;
                    break;
                }
                _ => {}
            }
        }

        assert_eq!(tokens, vec!["Fallback response"]);
        assert!(done_received, "Stream should end with Done event");
    }

    #[tokio::test]
    async fn test_mock_stream_ends_with_done() {
        use futures::StreamExt;

        let provider = MockProvider::new().with_stream_tokens(vec!["test"]);
        let messages = vec![Message::new(Role::User, "Test")];

        let mut stream = provider.stream(&messages);
        let mut last_event = None;

        while let Some(event) = stream.next().await {
            last_event = Some(event);
        }

        assert!(
            matches!(last_event, Some(Ok(StreamEvent::Done))),
            "Stream should end with Done event"
        );
    }

    #[tokio::test]
    async fn test_mock_provider_handles_tool_role() {
        // AC1: MockProvider handles Role::Tool messages without panicking
        let provider = MockProvider::new().with_response("After tool result");
        let messages = vec![
            Message::new(Role::User, "Hello"),
            Message::tool_result("call_1", "Tool output"),
        ];

        let response = provider.complete(&messages).await.unwrap();
        assert_eq!(response.content, "After tool result");
    }

    #[tokio::test]
    async fn test_mock_with_tool_call_response() {
        // AC2: with_tool_call_response configures the mock to return a message with tool calls
        // on the first call and a text response on subsequent calls
        let provider = MockProvider::new()
            .with_response("Final text response")
            .with_tool_call_response(vec![ToolCallData {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
                input: serde_json::json!({"location": "London"}),
            }]);

        let messages = vec![Message::new(Role::User, "What's the weather?")];

        // First call: returns tool call (LIFO order)
        let r1 = provider.complete(&messages).await.unwrap();
        assert!(r1.tool_calls.is_some());
        let tool_calls = r1.tool_calls.unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "get_weather");
        assert_eq!(tool_calls[0].id, "call_1");

        // Second call: returns text response
        let r2 = provider.complete(&messages).await.unwrap();
        assert_eq!(r2.content, "Final text response");
        assert!(r2.tool_calls.is_none());
    }

    #[tokio::test]
    async fn test_mock_complete_with_tools_delegates() {
        // complete_with_tools should use the same response queue
        let provider = MockProvider::new().with_response("Tool-aware response");
        let messages = vec![Message::new(Role::User, "Hello")];
        let tools = vec![crate::mcp::ToolDefinition {
            name: "test".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
        }];

        let response = provider
            .complete_with_tools(&messages, &tools)
            .await
            .unwrap();
        assert_eq!(response.content, "Tool-aware response");
    }
}
