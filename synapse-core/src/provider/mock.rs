//! Mock LLM provider for testing.
//!
//! Provides [`MockProvider`], a configurable mock implementation
//! of [`LlmProvider`] for unit and integration testing.

use std::sync::Mutex;

use async_trait::async_trait;

use super::{LlmProvider, ProviderError};
use crate::message::{Message, Role};

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
}
