//! LLM provider abstraction layer.
//!
//! Defines the [`LlmProvider`] trait that all LLM provider implementations
//! must fulfill, and the [`ProviderError`] type for error handling.

mod anthropic;
mod mock;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;

use async_trait::async_trait;

use crate::message::Message;

/// Error type for provider operations.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// The provider returned an error response.
    #[error("provider error: {message}")]
    ProviderError {
        /// The error message from the provider.
        message: String,
    },

    /// Request failed due to network or connection issues.
    #[error("request failed: {0}")]
    RequestFailed(String),

    /// Authentication failed (e.g., invalid API key).
    #[error("authentication failed: {0}")]
    AuthenticationError(String),
}

/// Trait for LLM providers.
///
/// Implementations must be thread-safe (`Send + Sync`) for use
/// in async contexts.
///
/// # Examples
///
/// ```
/// use synapse_core::provider::{LlmProvider, MockProvider};
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() {
/// let provider = MockProvider::new();
/// let messages = vec![Message::new(Role::User, "Hello")];
///
/// let response = provider.complete(&messages).await.unwrap();
/// assert_eq!(response.role, Role::Assistant);
/// # }
/// ```
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send messages to the LLM and get a response.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history to send to the model
    ///
    /// # Returns
    ///
    /// The assistant's response message, or an error if the request failed.
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError>;
}
