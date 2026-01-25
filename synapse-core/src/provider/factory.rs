//! Provider factory for dynamic provider creation.
//!
//! Creates the appropriate LLM provider based on configuration settings,
//! handling API key resolution from environment variables and config files.

use crate::config::Config;
use crate::provider::{AnthropicProvider, DeepSeekProvider, LlmProvider, ProviderError};

/// Create an LLM provider based on configuration.
///
/// Selects the appropriate provider based on `config.provider` and retrieves
/// the API key from environment variable or config file.
///
/// # Environment Variables
///
/// - `DEEPSEEK_API_KEY` for "deepseek" provider
/// - `ANTHROPIC_API_KEY` for "anthropic" provider
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
        "deepseek" | "anthropic" => {}
        unknown => return Err(ProviderError::UnknownProvider(unknown.to_string())),
    }

    let api_key = get_api_key(config)?;

    match config.provider.as_str() {
        "deepseek" => Ok(Box::new(DeepSeekProvider::new(api_key, &config.model))),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(api_key, &config.model))),
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

    fn make_config(provider: &str, api_key: Option<&str>) -> Config {
        Config {
            provider: provider.to_string(),
            model: "test-model".to_string(),
            api_key: api_key.map(|s| s.to_string()),
            session: None,
        }
    }

    #[test]
    fn test_create_provider_deepseek() {
        // Set env var for this test
        // SAFETY: This is a single-threaded test
        unsafe { env::set_var("DEEPSEEK_API_KEY", "test-deepseek-key") };

        let config = make_config("deepseek", None);
        let result = create_provider(&config);

        assert!(result.is_ok());

        // Clean up
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };
    }

    #[test]
    fn test_create_provider_anthropic() {
        // Set env var for this test
        // SAFETY: This is a single-threaded test
        unsafe { env::set_var("ANTHROPIC_API_KEY", "test-anthropic-key") };

        let config = make_config("anthropic", None);
        let result = create_provider(&config);

        assert!(result.is_ok());

        // Clean up
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("ANTHROPIC_API_KEY") };
    }

    #[test]
    fn test_create_provider_unknown() {
        let config = make_config("invalid", Some("key"));
        let result = create_provider(&config);

        assert!(matches!(result, Err(ProviderError::UnknownProvider(name)) if name == "invalid"));
    }

    #[test]
    fn test_get_api_key_from_env() {
        // Set env var
        // SAFETY: This is a single-threaded test
        unsafe { env::set_var("DEEPSEEK_API_KEY", "env-key-value") };

        let config = make_config("deepseek", None);
        let result = get_api_key(&config);

        assert_eq!(result.unwrap(), "env-key-value");

        // Clean up
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };
    }

    #[test]
    fn test_get_api_key_from_config() {
        // Ensure env var is not set
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };

        let config = make_config("deepseek", Some("config-key-value"));
        let result = get_api_key(&config);

        assert_eq!(result.unwrap(), "config-key-value");
    }

    #[test]
    fn test_env_var_takes_precedence() {
        // Set env var
        // SAFETY: This is a single-threaded test
        unsafe { env::set_var("DEEPSEEK_API_KEY", "env-key-value") };

        let config = make_config("deepseek", Some("config-key-value"));
        let result = get_api_key(&config);

        assert_eq!(result.unwrap(), "env-key-value");

        // Clean up
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };
    }

    #[test]
    fn test_get_api_key_missing() {
        // Ensure env var is not set
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("DEEPSEEK_API_KEY") };

        let config = make_config("deepseek", None);
        let result = get_api_key(&config);

        assert!(
            matches!(result, Err(ProviderError::MissingApiKey(msg)) if msg.contains("DEEPSEEK_API_KEY"))
        );
    }

    #[test]
    fn test_get_api_key_missing_anthropic() {
        // Ensure env var is not set
        // SAFETY: This is a single-threaded test
        unsafe { env::remove_var("ANTHROPIC_API_KEY") };

        let config = make_config("anthropic", None);
        let result = get_api_key(&config);

        assert!(
            matches!(result, Err(ProviderError::MissingApiKey(msg)) if msg.contains("ANTHROPIC_API_KEY"))
        );
    }
}
