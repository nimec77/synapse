//! OpenAI LLM provider.
//!
//! Implements the [`LlmProvider`] trait for OpenAI's Chat Completions API
//! by delegating all wire-format logic to the shared
//! [`openai_compat`](super::openai_compat) module.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::openai_compat::OpenAiCompatProvider;
use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::Message;

/// OpenAI Chat Completions API endpoint.
const API_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI LLM provider.
///
/// Sends messages to the OpenAI Chat Completions API and returns responses.
pub struct OpenAiProvider(pub(super) OpenAiCompatProvider);

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenAI API key
    /// * `model` - Model identifier (e.g., "gpt-4o")
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
impl LlmProvider for OpenAiProvider {
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
    fn test_openai_provider_new() {
        let provider = OpenAiProvider::new("test-key", "test-model", 4096);
        assert_eq!(provider.0.api_key, "test-key");
        assert_eq!(provider.0.model, "test-model");
        assert_eq!(provider.0.max_tokens, 4096);
    }
}
