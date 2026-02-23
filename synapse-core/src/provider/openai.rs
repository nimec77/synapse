//! OpenAI LLM provider.
//!
//! Implements the [`LlmProvider`] trait for OpenAI's Chat Completions API
//! by delegating all wire-format logic to the shared
//! [`openai_compat`](super::openai_compat) module.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::openai_compat::{self, DEFAULT_MAX_TOKENS};
use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::Message;

/// OpenAI Chat Completions API endpoint.
const API_ENDPOINT: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI LLM provider.
///
/// Sends messages to the OpenAI Chat Completions API and returns responses.
pub struct OpenAiProvider {
    /// HTTP client for API requests.
    client: reqwest::Client,
    /// OpenAI API key.
    api_key: String,
    /// Model identifier (e.g., "gpt-4o").
    model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
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
    fn test_openai_provider_new() {
        let provider = OpenAiProvider::new("test-key", "test-model");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "test-model");
    }
}
