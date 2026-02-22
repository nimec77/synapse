//! LLM provider abstraction layer.
//!
//! Defines the [`LlmProvider`] trait that all LLM provider implementations
//! must fulfill, and the [`ProviderError`] type for error handling.

mod anthropic;
mod deepseek;
mod factory;
mod mock;
mod openai;
mod openai_compat;
mod streaming;

pub use anthropic::AnthropicProvider;
pub use deepseek::DeepSeekProvider;
pub use factory::create_provider;
pub use mock::MockProvider;
pub use openai::OpenAiProvider;
pub use streaming::StreamEvent;

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::mcp::ToolDefinition;
use crate::message::Message;

/// Error type for provider operations.
#[derive(Debug, Clone, thiserror::Error)]
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

    /// API key not configured.
    #[error("missing API key: {0}")]
    MissingApiKey(String),

    /// Unknown provider name in configuration.
    #[error("unknown provider: {0}")]
    UnknownProvider(String),
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

    /// Stream response tokens from the LLM.
    ///
    /// Returns a stream of [`StreamEvent`] items. The stream ends with
    /// [`StreamEvent::Done`] on success or yields an error on failure.
    ///
    /// # Arguments
    ///
    /// * `messages` - Conversation history to send to the model
    ///
    /// # Returns
    ///
    /// A pinned, boxed stream of stream events. The `Send` bound enables
    /// use in async contexts across thread boundaries.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use futures::StreamExt;
    /// use synapse_core::provider::{LlmProvider, StreamEvent};
    ///
    /// async fn stream_example(provider: &dyn LlmProvider, messages: &[Message]) {
    ///     let mut stream = provider.stream(messages);
    ///     while let Some(event) = stream.next().await {
    ///         match event {
    ///             Ok(StreamEvent::TextDelta(text)) => print!("{}", text),
    ///             Ok(StreamEvent::Done) => break,
    ///             Err(e) => eprintln!("Error: {}", e),
    ///         }
    ///     }
    /// }
    /// ```
    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>>;

    /// Send messages with tool definitions and get a response.
    ///
    /// Default implementation delegates to `complete()`, ignoring tools.
    /// Providers that support tool calling should override this.
    async fn complete_with_tools(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        self.complete(messages).await
    }

    /// Stream response with tool definitions.
    ///
    /// Default implementation delegates to `stream()`, ignoring tools.
    /// Providers that support tool calling should override this.
    fn stream_with_tools(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        self.stream(messages)
    }
}
