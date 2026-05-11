---
name: project-conventions
description: "Use when adding an LLM provider, config field, storage method, MCP tool, error variant, or test in the synapse workspace and concrete file placement plus copy-paste sources are needed beyond what CLAUDE.md summarizes"
allowed-tools: Read, Glob, Grep, Bash
---

# Project Conventions — synapse

`CLAUDE.md` has the architecture overview. This skill points to the canonical example for each common change so you copy-and-adapt rather than reinvent.

## Workspace shape

Three crates, hexagonal architecture. Dependencies flow inward only.

| Crate | Role | Error style |
|---|---|---|
| `synapse-core` | Library: agent, providers, storage, MCP, message, config | `thiserror` |
| `synapse-cli` | CLI binary: clap, ratatui REPL | `anyhow` + `.context()` |
| `synapse-telegram` | Telegram bot: teloxide, format pipeline | `anyhow` + `.context()` |

`synapse-core` **never** imports from `synapse-cli` or `synapse-telegram`.

## Adding an OpenAI-compatible LLM provider

Most new providers reuse `OpenAiCompatProvider` (`synapse-core/src/provider/openai_compat.rs:272`), which already implements message conversion, the request/response cycle, tool-call wiring, and SSE streaming.

1. Create `synapse-core/src/provider/<name>.rs` modeled on `deepseek.rs` (~93 lines) or `openai.rs` (~76 lines):
   - `pub struct <Name>Provider(pub(super) OpenAiCompatProvider);`
   - `impl <Name>Provider::new(api_key, model, max_tokens)` constructs the inner type with the provider's API endpoint constant.
   - `#[async_trait] impl LlmProvider for <Name>Provider` delegates `complete`, `complete_with_tools`, and `stream` to `self.0`.
2. Register the module: add `pub mod <name>;` in `synapse-core/src/provider.rs:6-12` next to `deepseek`/`openai`/`anthropic`.
3. Wire the factory: extend `create_provider()` in `synapse-core/src/provider/factory.rs:46` with a new match arm for `config.provider == "<name>"`.
4. Add an env var constant for the API key (e.g. `<NAME>_API_KEY_ENV`) at the top of `factory.rs:12-16`. Resolution order is **env var > config field**, handled by `get_api_key()`.
5. Tests in `factory.rs` already cover env-var precedence — add one that exercises your provider arm.

For a non-OpenAI-compatible API (e.g. Anthropic), `synapse-core/src/provider/anthropic.rs` (~580 lines) is the standalone reference — it implements the full trait directly without `OpenAiCompatProvider`.

## Adding a config field

`synapse-core/src/config.rs:39` defines `Config`. Patterns:

- Optional fields: `Option<T>` with `#[serde(default)]`.
- Defaults: free function `fn default_<field>() -> T` plus `#[serde(default = "default_<field>")]`. See `default_max_tokens()` for the canonical example.
- Nested config (e.g. `TelegramConfig`, `LoggingConfig`, `SessionConfig`): write a `manual` `impl Default` — don't derive `Default` if you need a non-zero default (deriving on `u32` gives `0`, which is almost never what you want).
- Tests live in `synapse-core/src/config/tests.rs` — add at minimum a "deserializes with defaults" test and one that overrides the field.

## Adding a `SessionStore` method

The port lives in `synapse-core/src/storage.rs:55`; the SQLite adapter lives in `synapse-core/src/storage/sqlite.rs`.

1. Add the async method signature to the `SessionStore` trait. It must be `async`, return `Result<_, StorageError>`, and only use types from `synapse-core` (no SQLx leakage in the trait).
2. Implement on `SqliteStore` in `storage/sqlite.rs`. Use the connection pool (`self.pool`); never open a fresh connection.
3. If the method needs a new column or table, add a migration file under `synapse-core/migrations/` — sqlx auto-applies migrations on startup.
4. Add a test to `synapse-core/src/storage/sqlite/tests.rs` using the `tempfile`-backed in-memory store fixture.

## Adding an MCP tool

The MCP integration lives in `synapse-core/src/mcp.rs` and `synapse-core/src/mcp/{tools,protocol}.rs`. Tools are loaded dynamically from MCP servers configured in the user's `mcp.toml` — synapse does not define tools itself, it consumes them.

If you need to add behavior around tool execution (logging, filtering, augmentation), the surface is `Agent::execute_tool()` at `synapse-core/src/agent.rs:331` and `Agent::get_tool_definitions()` at `:323`. The tool-call loop runs inside `Agent::complete()` (`:194`) and `Agent::stream()` (`:249`) up to `MAX_ITERATIONS = 10`.

## Adding an error variant

Pick the right enum:

| Layer | Enum | File |
|---|---|---|
| Agent orchestrator (tool-call loop, MCP errors) | `AgentError` | `synapse-core/src/agent.rs:20` |
| LLM providers (HTTP, auth, missing key) | `ProviderError` | `synapse-core/src/provider.rs:31` |
| Storage (database, migration, not-found) | `StorageError` | `synapse-core/src/storage.rs:19` |
| Config (IO, parse, missing) | `ConfigError` | `synapse-core/src/config.rs:13` |
| CLI / Telegram top-level | `anyhow::Error` with `.context("...")` | binary `main.rs` files |

Rules:
- `synapse-core` **never** uses `unwrap()` or `expect()` outside `#[cfg(test)]`. Propagate with `?`.
- `#[from]` for transparent wrapping; explicit variants for domain-specific cases.
- Keep variants flat — no nested error types in payloads.

## Module layout

Use the **Rust 2018+ module system**. Never create `mod.rs`.

```
src/
├── provider.rs        # declares: pub mod anthropic; pub mod deepseek; ...
└── provider/
    ├── anthropic.rs
    ├── deepseek.rs
    ├── openai_compat.rs
    └── openai_compat/
        ├── tests.rs
        └── types.rs
```

Parent `.rs` file declares `pub mod child;` for each submodule. Submodules can have their own children with the same pattern.

## Test placement

| Where | Pattern | Example |
|---|---|---|
| Small unit tests for one module | `#[cfg(test)] mod tests { … }` at bottom of file | `agent.rs:352` |
| Larger suite for one module | `#[path = "module/tests.rs"] mod tests;` | `config.rs` → `config/tests.rs` |
| Submodule fixtures/types | own file in same dir | `openai_compat/types.rs`, `openai_compat/tests.rs` |
| Integration tests | `#[path = "..."]` includes from the parent module | `format/chunk.rs`, `commands/keyboard.rs`, `startup/tests.rs` |

Naming: `test_<function>_<scenario>` (e.g. `test_agent_default_system_prompt_none`). Async tests use `#[tokio::test]`.

Tests that mutate environment variables (provider factory, config) must hold a `Mutex` — see `ENV_MUTEX` at `synapse-core/src/provider/factory.rs:115`.

## Streaming

Providers return `Pin<Box<dyn Stream<Item = Result<StreamEvent, ProviderError>> + Send>>`. `StreamEvent` (in `provider/streaming.rs`) has exactly two variants:
- `TextDelta(String)` — token fragment
- `Done` — stream complete

Use `async_stream::stream!` macro and `eventsource-stream` for SSE parsing (already a dependency). When tools are configured, the stream runs `complete()` iterations under the hood and only the final response is streamed — see `Agent::stream()` at `agent.rs:249`.

## Telegram bot specifics

Documented in `CLAUDE.md` under "Telegram Bot Architecture". Quick pointers:
- Multi-session per chat: `ChatSessionMap = Arc<RwLock<HashMap<i64, ChatSessions>>>` in `synapse-telegram/src/handlers.rs`.
- Two-branch dispatcher: messages and callback queries, in `synapse-telegram/src/main.rs`.
- Markdown → HTML pipeline: `synapse-telegram/src/format.rs` (`md_to_telegram_html`, `chunk_html`, `escape_html`); chunking helpers split out into `format/chunk.rs`.
- Slash commands in `commands.rs`; inline keyboards in `commands/keyboard.rs`. Slash commands never invoke `Agent` — they're pure session management.

## Pre-commit gate

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

All three must pass before any commit. CI runs the same set on push.

## Workspace versioning

Lockstep — all three crates share `version.workspace = true` from the root `[workspace.package]`. The `release` skill bumps the canonical version and rotates `CHANGELOG.md`.
