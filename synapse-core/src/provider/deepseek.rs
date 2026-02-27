//! DeepSeek LLM provider.
//!
//! Implements the [`LlmProvider`] trait for DeepSeek's OpenAI-compatible
//! Chat Completions API by delegating all wire-format logic to the shared
//! [`openai_compat`](super::openai_compat) module.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::openai_compat::OpenAiCompatProvider;
use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::Message;

/// DeepSeek Chat Completions API endpoint.
const API_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

/// DeepSeek LLM provider.
///
/// Sends messages to the DeepSeek Chat Completions API (OpenAI-compatible)
/// and returns responses.
///
/// # Examples
///
/// ```no_run
/// use synapse_core::provider::{DeepSeekProvider, LlmProvider};
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = DeepSeekProvider::new("sk-...", "deepseek-chat", 4096);
/// let messages = vec![Message::new(Role::User, "Hello, DeepSeek!")];
///
/// let response = provider.complete(&messages).await?;
/// println!("{}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct DeepSeekProvider(pub(super) OpenAiCompatProvider);

impl DeepSeekProvider {
    /// Create a new DeepSeek provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - DeepSeek API key
    /// * `model` - Model identifier (e.g., "deepseek-chat")
    /// * `max_tokens` - Maximum tokens to generate in API responses
    pub fn new(api_key: impl Into<String>, model: impl Into<String>, max_tokens: u32) -> Self {
        Self(OpenAiCompatProvider::new(
            API_ENDPOINT,
            api_key,
            model,
            max_tokens,
        ))
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        self.0.complete(messages).await
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        self.0.complete_with_tools(messages, tools).await
    }

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        self.0.stream(messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_provider_new() {
        let provider = DeepSeekProvider::new("test-key", "test-model", 4096);
        assert_eq!(provider.0.api_key, "test-key");
        assert_eq!(provider.0.model, "test-model");
        assert_eq!(provider.0.max_tokens, 4096);
    }
}
