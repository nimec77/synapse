# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **SY-10: CLI REPL** - Interactive terminal UI for multi-turn conversations:
  - `synapse --repl` / `synapse -r` enters interactive REPL mode with a `ratatui` + `crossterm` TUI
  - Three-area vertical layout: scrollable conversation history, input area with cursor, status bar
  - Streaming responses render token-by-token in the conversation history
  - Full input editing: character insert, backspace, cursor movement (left/right/home/end)
  - History scrolling: up/down (line), page up/page down (page)
  - Session persistence: all messages stored to SQLite during REPL conversation
  - Session resume: `synapse --repl --session <uuid>` loads and continues a previous conversation
  - Session ID printed to stderr on exit for future resumption
  - `/quit` command and Ctrl+C for clean exit
  - `TerminalGuard` with `Drop` implementation ensures terminal state restoration on all exit paths
  - 29 new tests (25 REPL logic + 4 CLI flag parsing)
  - Dependencies: `ratatui` 0.30.0, `crossterm` 0.29.0 (with `event-stream` feature)

- **SY-9: Session Storage** - Persistent conversation storage using SQLite:
  - `Session`, `SessionSummary`, `StoredMessage` types in `synapse-core/src/session.rs`
  - `SessionStore` trait defining storage abstraction with CRUD operations
  - `SqliteStore` implementation with connection pooling and WAL mode
  - `StorageError` enum with `Database`, `NotFound`, `Migration`, `InvalidData` variants
  - `CleanupResult` struct tracking deleted sessions by limit and retention
  - Database migrations in `synapse-core/migrations/20250125_001_initial.sql`
  - `sessions` and `messages` tables with indexes and CASCADE delete
  - UUID v7 for time-sortable session and message identifiers
  - `SessionConfig` with `database_url`, `max_sessions` (default 100), `retention_days` (default 90), `auto_cleanup` (default true)
  - Database URL resolution priority: `DATABASE_URL` env var > config.toml > default path
  - CLI session commands: `synapse sessions list`, `synapse sessions show <uuid>`, `synapse sessions delete <uuid>`
  - Continue session: `synapse --session <uuid> "message"` or `synapse -s <uuid> "message"`
  - Auto-cleanup on startup when `auto_cleanup: true`
  - `create_storage()` factory function for storage initialization
  - 30+ unit tests for session, storage, and SQLite operations
  - Dependencies: sqlx (sqlite, runtime-tokio), uuid (v7, serde), chrono (serde), async-trait, dirs

- **SY-8: Streaming Responses** - Token-by-token output for real-time response display:
  - `StreamEvent` enum with `TextDelta`, `ToolCall`, `ToolResult`, `Done`, `Error` variants
  - `stream()` method added to `LlmProvider` trait with object-safe return type
  - DeepSeek SSE streaming via `eventsource-stream` and `async_stream` crates
  - CLI prints tokens progressively with `print!()` and `stdout.flush()`
  - Graceful Ctrl+C interruption via `tokio::select!` with `tokio::signal::ctrl_c()`
  - `[Interrupted]` message on Ctrl+C, clean exit
  - AnthropicProvider fallback streaming (wraps `complete()` for non-progressive output)
  - MockProvider `with_stream_tokens()` for testing streaming behavior
  - 12 unit tests for StreamEvent, SSE parsing, and provider streaming
  - Dependencies: `eventsource-stream = "0.2"`, `async-stream = "0.3"`, `futures = "0.3"`, reqwest `stream` feature, tokio `signal` and `io-std` features

- **SY-7: DeepSeek Provider** - Default LLM provider with provider factory:
  - `DeepSeekProvider` implementing `LlmProvider` trait for DeepSeek's OpenAI-compatible Chat Completions API
  - Provider factory pattern with `create_provider(config)` for dynamic provider selection
  - API key resolution with environment variable priority (`DEEPSEEK_API_KEY` > config file)
  - `MissingApiKey` and `UnknownProvider` error variants added to `ProviderError`
  - System messages included in messages array (OpenAI format, not separate field)
  - Authorization via Bearer token header
  - Default provider changed from hardcoded Anthropic to configuration-based selection
  - CLI now uses factory to create provider based on `config.provider` setting
  - 13 unit tests for DeepSeekProvider (5) and provider factory (8)
  - Support for provider switching: `provider = "deepseek"` or `provider = "anthropic"` in config

- **SY-6: Anthropic Provider** - Real Claude API integration:
  - `AnthropicProvider` implementing `LlmProvider` trait for Anthropic Messages API
  - HTTP client via `reqwest` with JSON serialization for API requests
  - System message extraction to separate `system` field in API request
  - `AuthenticationError` variant added to `ProviderError` for 401 responses
  - API version pinned to `2023-06-01` for stability
  - CLI now sends messages to Claude and displays real responses
  - API key validation with clear error message if missing
  - Support for both one-shot (`synapse "msg"`) and piped input modes
  - 8 unit tests for request/response serialization and message handling
  - Dependencies: reqwest (json), serde_json in synapse-core; tokio (rt-multi-thread), anyhow in synapse-cli

- **SY-5: Provider Abstraction** - LLM provider abstraction layer:
  - `Role` enum with `System`, `User`, `Assistant` variants for conversation roles
  - `Message` struct with role and content fields for conversation messages
  - `LlmProvider` trait with async `complete()` method as the provider contract
  - `MockProvider` for testing with configurable LIFO responses
  - `ProviderError` enum with `ProviderError` and `RequestFailed` variants
  - Object-safe, thread-safe trait design (`Send + Sync` bounds)
  - Dependencies: tokio (rt, macros), async-trait in synapse-core

- **SY-4: Configuration System** - TOML-based configuration loading:
  - `Config` struct with `provider`, `api_key`, and `model` fields
  - Priority-based config loading: `SYNAPSE_CONFIG` env var > `./config.toml` > `~/.config/synapse/config.toml` > defaults
  - Default values: provider = "deepseek", model = "deepseek-chat"
  - `ConfigError` with `IoError` and `ParseError` variants
  - CLI displays configured provider on startup
  - `config.example.toml` with documented options
  - Dependencies: toml, serde, dirs, thiserror in synapse-core
  - synapse-cli now depends on synapse-core

- **SY-3: Echo CLI** - CLI argument parsing with clap:
  - One-shot mode: `synapse "message"` prints `Echo: message`
  - Stdin mode: `echo "message" | synapse` reads from pipe
  - TTY detection shows help when no input provided
  - `--help` and `--version` flags

- **SY-2: CI/CD Pipeline** - GitHub Actions workflow for automated quality checks:
  - Format check (`cargo fmt --check`)
  - Linting with warnings as errors (`cargo clippy -- -D warnings`)
  - Test execution (`cargo test`)
  - Security audit via `rustsec/audit-check`
  - Dependency caching with `Swatinem/rust-cache`
  - Triggers on push to `master`/`feature/*` and PRs to `master`
  - `rust-toolchain.toml` for consistent nightly toolchain

- **SY-1: Project Foundation** - Established Rust workspace with three crates:
  - `synapse-core`: Core library for agent logic, providers, storage, and MCP
  - `synapse-cli`: CLI binary (executable: `synapse`)
  - `synapse-telegram`: Telegram bot binary
  - Configured for Rust Edition 2024 with resolver version 3
