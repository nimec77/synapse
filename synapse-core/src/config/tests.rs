use super::*;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.provider, "deepseek");
    assert_eq!(config.api_key, None);
    assert_eq!(config.model, "deepseek-chat");
}

#[test]
fn test_parse_minimal_toml() {
    let toml = r#"provider = "anthropic""#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.provider, "anthropic");
    assert_eq!(config.api_key, None);
    assert_eq!(config.model, "deepseek-chat"); // default
}

#[test]
fn test_parse_full_toml() {
    let toml = r#"
provider = "openai"
api_key = "sk-test-key"
model = "gpt-4"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.provider, "openai");
    assert_eq!(config.api_key, Some("sk-test-key".to_string()));
    assert_eq!(config.model, "gpt-4");
}

#[test]
fn test_parse_partial_toml() {
    let toml = r#"model = "claude-3-opus""#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.provider, "deepseek"); // default
    assert_eq!(config.api_key, None);
    assert_eq!(config.model, "claude-3-opus");
}

#[test]
fn test_parse_empty_toml() {
    let toml = "";
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.provider, "deepseek");
    assert_eq!(config.api_key, None);
    assert_eq!(config.model, "deepseek-chat");
}

#[test]
fn test_load_from_path() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("synapse_test_config.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, r#"provider = "test-provider""#).unwrap();
    drop(file);

    let config = Config::load_from(&path).unwrap();
    assert_eq!(config.provider, "test-provider");

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_parse_invalid_toml() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("synapse_invalid_config.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, r#"invalid = ["#).unwrap();
    drop(file);

    let result = Config::load_from(&path);
    assert!(matches!(result, Err(ConfigError::ParseError { .. })));

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_load_from_nonexistent_file() {
    let result = Config::load_from("/nonexistent/path/config.toml");
    assert!(matches!(result, Err(ConfigError::IoError { .. })));
}

#[test]
fn test_session_config_defaults() {
    let config = SessionConfig::default();
    assert_eq!(config.database_url, None);
    assert_eq!(config.max_sessions, 100);
    assert_eq!(config.retention_days, 90);
    assert!(config.auto_cleanup);
}

#[test]
fn test_config_without_session_section() {
    let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.session.is_none());
}

#[test]
fn test_config_with_session_section() {
    let toml = r#"
provider = "deepseek"
model = "deepseek-chat"

[session]
max_sessions = 50
retention_days = 30
auto_cleanup = false
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.session.is_some());
    let session = config.session.unwrap();
    assert_eq!(session.max_sessions, 50);
    assert_eq!(session.retention_days, 30);
    assert!(!session.auto_cleanup);
}

#[test]
fn test_session_config_partial_defaults() {
    let toml = r#"
[session]
max_sessions = 200
"#;
    let config: Config = toml::from_str(toml).unwrap();
    let session = config.session.unwrap();
    assert_eq!(session.max_sessions, 200);
    assert_eq!(session.retention_days, 90); // default
    assert!(session.auto_cleanup); // default
}

#[test]
fn test_session_config_with_database_url() {
    let toml = r#"
[session]
database_url = "sqlite:/custom/path/sessions.db"
max_sessions = 50
"#;
    let config: Config = toml::from_str(toml).unwrap();
    let session = config.session.unwrap();
    assert_eq!(
        session.database_url,
        Some("sqlite:/custom/path/sessions.db".to_string())
    );
    assert_eq!(session.max_sessions, 50);
}

#[test]
fn test_config_without_mcp_section() {
    let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.mcp.is_none());
}

#[test]
fn test_config_with_mcp_section() {
    let toml = r#"
provider = "deepseek"

[mcp]
config_path = "/custom/path/mcp_servers.json"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.mcp.is_some());
    let mcp = config.mcp.unwrap();
    assert_eq!(
        mcp.config_path,
        Some("/custom/path/mcp_servers.json".to_string())
    );
}

#[test]
fn test_config_with_mcp_section_no_path() {
    let toml = r#"
[mcp]
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.mcp.is_some());
    assert!(config.mcp.unwrap().config_path.is_none());
}

#[test]
fn test_config_without_telegram_section() {
    let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.telegram.is_none());
}

#[test]
fn test_config_with_telegram_section() {
    let toml = r#"
provider = "deepseek"

[telegram]
token = "123456:ABC-DEF"
allowed_users = [123456789, 987654321]
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.telegram.is_some());
    let tg = config.telegram.unwrap();
    assert_eq!(tg.token, Some("123456:ABC-DEF".to_string()));
    assert_eq!(tg.allowed_users, vec![123456789u64, 987654321u64]);
}

#[test]
fn test_config_telegram_partial_defaults() {
    let toml = r#"
[telegram]
token = "bot-token-only"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    let tg = config.telegram.unwrap();
    assert_eq!(tg.token, Some("bot-token-only".to_string()));
    assert!(tg.allowed_users.is_empty());
}

#[test]
fn test_telegram_config_default() {
    let tg = TelegramConfig::default();
    assert!(tg.token.is_none());
    assert!(tg.allowed_users.is_empty());
    assert_eq!(tg.max_sessions_per_chat, 10);
}

#[test]
fn test_config_telegram_max_sessions_per_chat() {
    // Explicit value should be honoured.
    let toml = r#"
[telegram]
max_sessions_per_chat = 5
"#;
    let config: Config = toml::from_str(toml).unwrap();
    let tg = config.telegram.unwrap();
    assert_eq!(tg.max_sessions_per_chat, 5);

    // Omitting the field should yield the default of 10.
    let toml_default = r#"
[telegram]
"#;
    let config_default: Config = toml::from_str(toml_default).unwrap();
    let tg_default = config_default.telegram.unwrap();
    assert_eq!(tg_default.max_sessions_per_chat, 10);
}

#[test]
fn test_config_with_system_prompt() {
    let toml = r#"system_prompt = "You are helpful.""#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.system_prompt, Some("You are helpful.".to_string()));
}

#[test]
fn test_config_without_system_prompt() {
    let toml = r#"provider = "deepseek""#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(config.system_prompt, None);
}

#[test]
fn test_config_default_system_prompt() {
    let config = Config::default();
    assert_eq!(config.system_prompt, None);
}

#[test]
fn test_parse_system_prompt_file_field() {
    let toml = r#"system_prompt_file = "prompts/system.md""#;
    let config: Config = toml::from_str(toml).unwrap();
    assert_eq!(
        config.system_prompt_file,
        Some("prompts/system.md".to_string())
    );
    assert_eq!(config.system_prompt, None);
}

#[test]
fn test_resolve_system_prompt_from_file() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("synapse_test_system_prompt.md");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "You are a helpful assistant.").unwrap();
    drop(file);

    let mut config = Config {
        system_prompt_file: Some(path.to_str().unwrap().to_string()),
        ..Config::default()
    };
    config.resolve_system_prompt().unwrap();

    assert_eq!(
        config.system_prompt,
        Some("You are a helpful assistant.".to_string())
    );
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_resolve_system_prompt_inline_takes_priority() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("synapse_test_system_prompt_priority.md");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "From file.").unwrap();
    drop(file);

    let mut config = Config {
        system_prompt: Some("Inline wins.".to_string()),
        system_prompt_file: Some(path.to_str().unwrap().to_string()),
        ..Config::default()
    };
    config.resolve_system_prompt().unwrap();

    assert_eq!(config.system_prompt, Some("Inline wins.".to_string()));
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_resolve_system_prompt_file_not_found() {
    let mut config = Config {
        system_prompt_file: Some("/nonexistent/path/prompt.md".to_string()),
        ..Config::default()
    };
    let result = config.resolve_system_prompt();
    assert!(matches!(result, Err(ConfigError::IoError { .. })));
}

#[test]
fn test_resolve_system_prompt_neither_set() {
    let mut config = Config::default();
    config.resolve_system_prompt().unwrap();
    assert_eq!(config.system_prompt, None);
}

#[test]
fn test_resolve_system_prompt_empty_file() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("synapse_test_empty_prompt.md");
    let mut file = std::fs::File::create(&path).unwrap();
    write!(file, "   \n\t\n  ").unwrap();
    drop(file);

    let mut config = Config {
        system_prompt_file: Some(path.to_str().unwrap().to_string()),
        ..Config::default()
    };
    config.resolve_system_prompt().unwrap();

    assert_eq!(config.system_prompt, None);
    std::fs::remove_file(&path).ok();
}

#[test]
fn test_config_default_max_tokens() {
    // Empty TOML should yield the default of 4096.
    let config: Config = toml::from_str("").unwrap();
    assert_eq!(config.max_tokens, 4096);

    // Explicit value should be honoured.
    let config: Config = toml::from_str("max_tokens = 8192").unwrap();
    assert_eq!(config.max_tokens, 8192);
}

#[test]
fn test_logging_config_defaults() {
    let lc = LoggingConfig::default();
    assert_eq!(lc.directory, "logs");
    assert_eq!(lc.max_files, 7);
    assert_eq!(lc.rotation, Rotation::Daily);
}

#[test]
fn test_config_without_logging_section() {
    let toml = r#"
provider = "deepseek"
model = "deepseek-chat"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.logging.is_none());
}

#[test]
fn test_config_with_logging_section() {
    let toml = r#"
provider = "deepseek"

[logging]
directory = "/var/log/synapse"
max_files = 30
rotation = "hourly"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.logging.is_some());
    let lc = config.logging.unwrap();
    assert_eq!(lc.directory, "/var/log/synapse");
    assert_eq!(lc.max_files, 30);
    assert_eq!(lc.rotation, Rotation::Hourly);
}

#[test]
fn test_config_with_logging_section_partial_defaults() {
    let toml = r#"
[logging]
directory = "/tmp/logs"
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.logging.is_some());
    let lc = config.logging.unwrap();
    assert_eq!(lc.directory, "/tmp/logs");
    assert_eq!(lc.max_files, 7); // default
    assert_eq!(lc.rotation, Rotation::Daily); // default
}

#[test]
fn test_config_with_empty_logging_section() {
    let toml = r#"
[logging]
"#;
    let config: Config = toml::from_str(toml).unwrap();
    assert!(config.logging.is_some());
    let lc = config.logging.unwrap();
    assert_eq!(lc.directory, "logs"); // default
    assert_eq!(lc.max_files, 7); // default
    assert_eq!(lc.rotation, Rotation::Daily); // default
}

// Rotation enum deserialization tests.

#[test]
fn test_rotation_deserialize_daily() {
    let lc: LoggingConfig = toml::from_str("rotation = \"daily\"").unwrap();
    assert_eq!(lc.rotation, Rotation::Daily);
}

#[test]
fn test_rotation_deserialize_hourly() {
    let lc: LoggingConfig = toml::from_str("rotation = \"hourly\"").unwrap();
    assert_eq!(lc.rotation, Rotation::Hourly);
}

#[test]
fn test_rotation_deserialize_never() {
    let lc: LoggingConfig = toml::from_str("rotation = \"never\"").unwrap();
    assert_eq!(lc.rotation, Rotation::Never);
}

#[test]
fn test_resolve_system_prompt_trims_whitespace() {
    use std::io::Write;
    let dir = std::env::temp_dir();
    let path = dir.join("synapse_test_whitespace_prompt.md");
    let mut file = std::fs::File::create(&path).unwrap();
    write!(file, "\n  You are a coding assistant.  \n\n").unwrap();
    drop(file);

    let mut config = Config {
        system_prompt_file: Some(path.to_str().unwrap().to_string()),
        ..Config::default()
    };
    config.resolve_system_prompt().unwrap();

    assert_eq!(
        config.system_prompt,
        Some("You are a coding assistant.".to_string())
    );
    std::fs::remove_file(&path).ok();
}
