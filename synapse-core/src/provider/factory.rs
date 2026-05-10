//! Provider factory for dynamic provider creation.
//!
//! Creates the appropriate LLM provider based on configuration settings,
//! handling API key resolution from environment variables and config files.

use crate::config::Config;
use crate::provider::{
    AnthropicProvider, DeepSeekProvider, LlmProvider, OpenAiProvider, ProviderError,
};

use super::deepseek::is_reasoning_model;

/// Environment variable name for the DeepSeek API key.
const DEEPSEEK_API_KEY_ENV: &str = "DEEPSEEK_API_KEY";
/// Environment variable name for the Anthropic API key.
const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";
/// Environment variable name for the OpenAI API key.
const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";

/// Create an LLM provider based on configuration.
///
/// Selects the appropriate provider based on `config.provider` and retrieves
/// the API key from environment variable or config file.
///
/// # Environment Variables
///
/// - `DEEPSEEK_API_KEY` for "deepseek" provider
/// - `ANTHROPIC_API_KEY` for "anthropic" provider
/// - `OPENAI_API_KEY` for "openai" provider
///
/// # Errors
///
/// - [`ProviderError::MissingApiKey`] if no API key is found
/// - [`ProviderError::UnknownProvider`] if provider name is not recognized
///
/// # Examples
///
/// ```no_run
/// use synapse_core::config::Config;
/// use synapse_core::provider::create_provider;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::load(None)?;
/// let provider = create_provider(&config)?;
/// # Ok(())
/// # }
/// ```
pub fn create_provider(config: &Config) -> Result<Box<dyn LlmProvider>, ProviderError> {
    // Validate provider name first
    match config.provider.as_str() {
        "deepseek" | "anthropic" | "openai" => {}
        unknown => return Err(ProviderError::UnknownProvider(unknown.to_string())),
    }

    // Warn when reasoning_effort is set for a non-DeepSeek provider (forward-compat).
    if config.reasoning_effort.is_some() && config.provider != "deepseek" {
        tracing::warn!(
            provider = %config.provider,
            "config.reasoning_effort is set but provider does not support reasoning mode; ignoring",
        );
    }

    let api_key = get_api_key(config)?;

    tracing::info!(provider = %config.provider, model = %config.model, "factory: creating provider");

    match config.provider.as_str() {
        "deepseek" => {
            let (effort, auto_escalated) = resolve_reasoning_effort(config);

            if is_reasoning_model(&config.model) {
                tracing::info!(
                    model = %config.model,
                    effort = %effort,
                    auto_escalated,
                    "factory: enabling DeepSeek thinking mode",
                );
            }

            Ok(Box::new(DeepSeekProvider::new(
                api_key,
                &config.model,
                config.max_tokens,
                Some(effort),
            )))
        }
        "anthropic" => Ok(Box::new(AnthropicProvider::new(
            api_key,
            &config.model,
            config.max_tokens,
        ))),
        "openai" => Ok(Box::new(OpenAiProvider::new(
            api_key,
            &config.model,
            config.max_tokens,
        ))),
        _ => unreachable!("Provider validated above"),
    }
}

/// Resolve the effective reasoning effort string and whether it was auto-escalated.
///
/// Priority:
/// 1. Explicit `config.reasoning_effort` — always wins; no escalation.
/// 2. MCP configured (`config.mcp.is_some()`) AND no explicit effort → `"max"` (auto-escalated).
/// 3. Otherwise → `"high"` (the safe default per DeepSeek docs).
fn resolve_reasoning_effort(config: &Config) -> (String, bool) {
    if let Some(ref explicit) = config.reasoning_effort {
        (explicit.clone(), false)
    } else if config.mcp.is_some() {
        ("max".to_string(), true)
    } else {
        ("high".to_string(), false)
    }
}

/// Retrieve API key from environment variable or config file.
///
/// Priority: environment variable > config.api_key
///
/// # Panics
///
/// Panics if called with an unknown provider (caller should validate first).
fn get_api_key(config: &Config) -> Result<String, ProviderError> {
    let env_var = match config.provider.as_str() {
        "deepseek" => DEEPSEEK_API_KEY_ENV,
        "anthropic" => ANTHROPIC_API_KEY_ENV,
        "openai" => OPENAI_API_KEY_ENV,
        _ => unreachable!("Provider should be validated before calling get_api_key"),
    };

    // Check environment variable first
    if let Ok(key) = std::env::var(env_var)
        && !key.is_empty()
    {
        return Ok(key);
    }

    // Fall back to config file
    config.api_key.clone().ok_or_else(|| {
        ProviderError::MissingApiKey(format!(
            "Set {} environment variable or add api_key to config.toml",
            env_var
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Serialize all tests that mutate environment variables to prevent race conditions.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn make_config(provider: &str, api_key: Option<&str>) -> Config {
        Config {
            provider: provider.to_string(),
            model: "test-model".to_string(),
            api_key: api_key.map(|s| s.to_string()),
            max_tokens: 4096,
            system_prompt: None,
            system_prompt_file: None,
            session: None,
            mcp: None,
            telegram: None,
            logging: None,
            reasoning_effort: None,
            cli: None,
        }
    }

    fn make_deepseek_config(model: &str, effort: Option<&str>, with_mcp: bool) -> Config {
        use crate::config::McpSettings;
        Config {
            provider: "deepseek".to_string(),
            model: model.to_string(),
            api_key: Some("test-key".to_string()),
            max_tokens: 4096,
            system_prompt: None,
            system_prompt_file: None,
            session: None,
            mcp: if with_mcp {
                Some(McpSettings {
                    config_path: Some("/fake/mcp.json".to_string()),
                })
            } else {
                None
            },
            telegram: None,
            logging: None,
            reasoning_effort: effort.map(|s| s.to_string()),
            cli: None,
        }
    }

    #[test]
    fn test_create_provider_deepseek() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::set_var("DEEPSEEK_API_KEY", "test-deepseek-key") };

        let config = make_config("deepseek", None);
        let result = create_provider(&config);

        unsafe { env::remove_var("DEEPSEEK_API_KEY") };
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_provider_anthropic() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::set_var("ANTHROPIC_API_KEY", "test-anthropic-key") };

        let config = make_config("anthropic", None);
        let result = create_provider(&config);

        unsafe { env::remove_var("ANTHROPIC_API_KEY") };
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_provider_unknown() {
        let config = make_config("invalid", Some("key"));
        let result = create_provider(&config);

        assert!(matches!(result, Err(ProviderError::UnknownProvider(name)) if name == "invalid"));
    }

    #[test]
    fn test_get_api_key_from_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::set_var("DEEPSEEK_API_KEY", "env-key-value") };

        let config = make_config("deepseek", None);
        let result = get_api_key(&config);

        unsafe { env::remove_var("DEEPSEEK_API_KEY") };
        assert_eq!(result.unwrap(), "env-key-value");
    }

    #[test]
    fn test_get_api_key_from_config() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };

        let config = make_config("deepseek", Some("config-key-value"));
        let result = get_api_key(&config);

        assert_eq!(result.unwrap(), "config-key-value");
    }

    #[test]
    fn test_env_var_takes_precedence() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::set_var("DEEPSEEK_API_KEY", "env-key-value") };

        let config = make_config("deepseek", Some("config-key-value"));
        let result = get_api_key(&config);

        unsafe { env::remove_var("DEEPSEEK_API_KEY") };
        assert_eq!(result.unwrap(), "env-key-value");
    }

    #[test]
    fn test_get_api_key_missing() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };

        let config = make_config("deepseek", None);
        let result = get_api_key(&config);

        assert!(
            matches!(result, Err(ProviderError::MissingApiKey(msg)) if msg.contains("DEEPSEEK_API_KEY"))
        );
    }

    #[test]
    fn test_get_api_key_missing_anthropic() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::remove_var("ANTHROPIC_API_KEY") };

        let config = make_config("anthropic", None);
        let result = get_api_key(&config);

        assert!(
            matches!(result, Err(ProviderError::MissingApiKey(msg)) if msg.contains("ANTHROPIC_API_KEY"))
        );
    }

    #[test]
    fn test_create_provider_openai() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::set_var("OPENAI_API_KEY", "test-openai-key") };

        let config = make_config("openai", None);
        let result = create_provider(&config);

        unsafe { env::remove_var("OPENAI_API_KEY") };
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_api_key_missing_openai() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // SAFETY: Serialized via ENV_MUTEX; no concurrent env-var mutation.
        unsafe { env::remove_var("OPENAI_API_KEY") };

        let config = make_config("openai", None);
        let result = get_api_key(&config);

        assert!(
            matches!(result, Err(ProviderError::MissingApiKey(msg)) if msg.contains("OPENAI_API_KEY"))
        );
    }

    // =========================================================================
    // SY-22: Reasoning effort resolution tests
    // =========================================================================

    /// AC: `resolve_reasoning_effort` with explicit `reasoning_effort = "low"` returns
    /// `("low", false)` regardless of MCP.
    #[test]
    fn test_factory_explicit_effort_overrides_mcp_escalation() {
        let config = make_deepseek_config("deepseek-v4-pro", Some("low"), true);
        let (effort, auto_escalated) = resolve_reasoning_effort(&config);
        assert_eq!(effort, "low");
        assert!(!auto_escalated, "explicit always wins, no auto-escalation");
    }

    /// AC: `resolve_reasoning_effort` with no explicit effort AND MCP configured returns
    /// `("max", true)`.
    #[test]
    fn test_factory_auto_escalates_to_max_with_mcp() {
        let config = make_deepseek_config("deepseek-v4-pro", None, true);
        let (effort, auto_escalated) = resolve_reasoning_effort(&config);
        assert_eq!(effort, "max");
        assert!(auto_escalated);
    }

    /// AC: `resolve_reasoning_effort` with no explicit effort AND no MCP returns
    /// `("high", false)`.
    #[test]
    fn test_factory_no_mcp_no_explicit_uses_high() {
        let config = make_deepseek_config("deepseek-v4-pro", None, false);
        let (effort, auto_escalated) = resolve_reasoning_effort(&config);
        assert_eq!(effort, "high");
        assert!(!auto_escalated);
    }

    /// AC: `create_provider` passes explicit reasoning effort to DeepSeek for a
    /// reasoning model.
    #[test]
    fn test_factory_passes_reasoning_effort() {
        let config = make_deepseek_config("deepseek-v4-pro", Some("low"), false);
        // DeepSeekProvider is Box<dyn LlmProvider> so we can't introspect it here.
        // The unit tests in deepseek.rs cover the inner provider's reasoning field.
        // Here we just assert create_provider succeeds (api_key is in config).
        let result = create_provider(&config);
        assert!(result.is_ok());
    }
}
