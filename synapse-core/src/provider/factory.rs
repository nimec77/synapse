//! Provider factory for dynamic provider creation.
//!
//! Creates the appropriate LLM provider based on configuration settings,
//! handling API key resolution from environment variables and config files.

use crate::config::Config;
use crate::provider::{
    AnthropicProvider, DeepSeekProvider, LlmProvider, OpenAiProvider, ProviderError,
};

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
/// let config = Config::load()?;
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

    let api_key = get_api_key(config)?;

    match config.provider.as_str() {
        "deepseek" => Ok(Box::new(DeepSeekProvider::new(api_key, &config.model))),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(api_key, &config.model))),
        "openai" => Ok(Box::new(OpenAiProvider::new(api_key, &config.model))),
        _ => unreachable!("Provider validated above"),
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
        "deepseek" => "DEEPSEEK_API_KEY",
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
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
            system_prompt: None,
            system_prompt_file: None,
            session: None,
            mcp: None,
            telegram: None,
            logging: None,
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
}
