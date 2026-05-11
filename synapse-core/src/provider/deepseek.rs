//! DeepSeek LLM provider.
//!
//! Implements the [`LlmProvider`] trait for DeepSeek's OpenAI-compatible
//! Chat Completions API by delegating all wire-format logic to the shared
//! [`openai_compat`](super::openai_compat) module.

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::openai_compat::{OpenAiCompatProvider, ReasoningSettings};
use super::{LlmProvider, ProviderError, StreamEvent};
use crate::mcp::ToolDefinition;
use crate::message::Message;

/// DeepSeek Chat Completions API endpoint.
const API_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";

/// DeepSeek model identifiers known to support thinking (chain-of-thought) mode.
///
/// Kept in sync with `docs/research/SY-22-deepseek-v4-pro.md`.
/// Add new reasoning-capable model identifiers here as DeepSeek releases them.
pub(super) const REASONING_MODELS: &[&str] = &["deepseek-v4-pro"];

/// Default reasoning effort when a reasoning model is selected but the user
/// has not set `reasoning_effort` explicitly in config.
pub(super) const DEFAULT_REASONING_EFFORT: &str = "high";

/// Return `true` if `model` is a DeepSeek reasoning-capable model.
pub(super) fn is_reasoning_model(model: &str) -> bool {
    REASONING_MODELS.contains(&model)
}

/// DeepSeek LLM provider.
///
/// Sends messages to the DeepSeek Chat Completions API (OpenAI-compatible)
/// and returns responses.
///
/// # Reasoning models
///
/// When `model` is in [`REASONING_MODELS`] (e.g. `"deepseek-v4-pro"`), thinking mode
/// is enabled automatically. The `reasoning_effort` parameter controls the effort level
/// (`"low"`, `"medium"`, `"high"`, `"max"`); defaults to `"high"` when `None`.
///
/// # Examples
///
/// ```no_run
/// use synapse_core::provider::{DeepSeekProvider, LlmProvider};
/// use synapse_core::message::{Message, Role};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = DeepSeekProvider::new("sk-...", "deepseek-chat", 4096, None);
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
    /// When `model` is in [`REASONING_MODELS`] (e.g. `"deepseek-v4-pro"`), thinking mode
    /// is configured automatically with the given effort level (defaulting to
    /// [`DEFAULT_REASONING_EFFORT`] = `"high"` if `reasoning_effort` is `None`).
    ///
    /// For non-reasoning models (e.g. `"deepseek-chat"`), `reasoning_effort` is ignored
    /// and the outbound wire format is byte-identical to pre-SY-22.
    ///
    /// # Arguments
    ///
    /// * `api_key` - DeepSeek API key
    /// * `model` - Model identifier (e.g., `"deepseek-chat"`, `"deepseek-v4-pro"`)
    /// * `max_tokens` - Maximum tokens to generate in API responses
    /// * `reasoning_effort` - Effort level for reasoning models; `None` uses `"high"`
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        max_tokens: u32,
        reasoning_effort: Option<String>,
    ) -> Self {
        let model_str = model.into();
        let mut inner = OpenAiCompatProvider::new(API_ENDPOINT, api_key, &model_str, max_tokens);

        if is_reasoning_model(&model_str) {
            let effort = reasoning_effort.unwrap_or_else(|| DEFAULT_REASONING_EFFORT.to_string());
            inner = inner.with_reasoning(ReasoningSettings { effort });
        }

        Self(inner)
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
        let provider = DeepSeekProvider::new("test-key", "test-model", 4096, None);
        assert_eq!(provider.0.api_key, "test-key");
        assert_eq!(provider.0.model, "test-model");
        assert_eq!(provider.0.max_tokens, 4096);
    }

    #[test]
    fn test_is_reasoning_model_known_and_unknown() {
        assert!(is_reasoning_model("deepseek-v4-pro"));
        assert!(!is_reasoning_model("deepseek-chat"));
        assert!(!is_reasoning_model("gpt-4o"));
        assert!(!is_reasoning_model(""));
        assert!(!is_reasoning_model("deepseek-v4-pro-extra")); // exact match only
    }

    #[test]
    fn test_deepseek_provider_v4_pro_enables_reasoning() {
        let provider = DeepSeekProvider::new("test-key", "deepseek-v4-pro", 4096, None);
        // Reasoning must be enabled with the default effort
        assert!(provider.0.reasoning.is_some());
        assert_eq!(
            provider.0.reasoning.as_ref().unwrap().effort,
            DEFAULT_REASONING_EFFORT
        );
    }

    #[test]
    fn test_deepseek_provider_chat_does_not_enable_reasoning() {
        let provider = DeepSeekProvider::new("test-key", "deepseek-chat", 4096, None);
        // Non-reasoning model: reasoning must be absent
        assert!(provider.0.reasoning.is_none());
    }

    #[test]
    fn test_deepseek_provider_v4_pro_with_explicit_effort() {
        let provider =
            DeepSeekProvider::new("test-key", "deepseek-v4-pro", 4096, Some("max".to_string()));
        assert!(provider.0.reasoning.is_some());
        assert_eq!(provider.0.reasoning.as_ref().unwrap().effort, "max");
    }
}
