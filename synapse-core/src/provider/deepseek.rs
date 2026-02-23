//! DeepSeek LLM provider.
//!
//! Implements the [`LlmProvider`] trait for DeepSeek's OpenAI-compatible
//! Chat Completions API by delegating all wire-format logic to the shared
//! [`openai_compat`](super::openai_compat) module.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::openai_compat::{self, DEFAULT_MAX_TOKENS};
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
/// let provider = DeepSeekProvider::new("sk-...", "deepseek-chat");
/// let messages = vec![Message::new(Role::User, "Hello, DeepSeek!")];
///
/// let response = provider.complete(&messages).await?;
/// println!("{}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct DeepSeekProvider {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// DeepSeek API key.
    api_key: String,
    /// Model identifier (e.g., "deepseek-chat").
    model: String,
}

impl DeepSeekProvider {
    /// Create a new DeepSeek provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - DeepSeek API key
    /// * `model` - Model identifier (e.g., "deepseek-chat")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    async fn complete(&self, messages: &[Message]) -> Result<Message, ProviderError> {
        let api_messages = openai_compat::build_api_messages(messages);
        let request = openai_compat::ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: DEFAULT_MAX_TOKENS,
            tools: None,
            tool_choice: None,
        };
        openai_compat::complete_request(&self.client, API_ENDPOINT, &self.api_key, &request).await
    }

    async fn complete_with_tools(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<Message, ProviderError> {
        let api_messages = openai_compat::build_api_messages(messages);
        let request = openai_compat::ApiRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: DEFAULT_MAX_TOKENS,
            tools: openai_compat::to_oai_tools(tools),
            tool_choice: if tools.is_empty() {
                None
            } else {
                Some("auto".to_string())
            },
        };
        openai_compat::complete_request(&self.client, API_ENDPOINT, &self.api_key, &request).await
    }

    fn stream(
        &self,
        messages: &[Message],
    ) -> Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send + '_>> {
        openai_compat::stream_sse(
            self.client.clone(),
            API_ENDPOINT,
            self.api_key.clone(),
            self.model.clone(),
            messages.to_vec(),
            DEFAULT_MAX_TOKENS,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepseek_provider_new() {
        let provider = DeepSeekProvider::new("test-key", "test-model");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "test-model");
    }
}
