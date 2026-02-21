# SY-13 Summary: Phase 12 - Telegram Bot

**Status:** COMPLETE
**Date:** 2026-02-21
**QA Verdict:** RELEASE WITH RESERVATIONS

---

## Overview

SY-13 brings the `synapse-telegram` crate from an empty placeholder (created in SY-1) to a fully functional Telegram bot. This is the second interface for Synapse — alongside the CLI — and validates the hexagonal architecture by proving that a second frontend can share the same `synapse-core` Agent, SessionStore, and MCP subsystems without any modification to core code.

The implementation adds user authorization via an allowlist, session-per-chat persistence keyed by Telegram chat ID, a typing indicator before LLM responses, response chunking for Telegram's 4096-character message limit, and graceful shutdown. A single `TelegramConfig` struct addition to `synapse-core/src/config.rs` is the only change to core; all Telegram-specific logic lives in `synapse-telegram`.

---

## What Was Built

### New Components

1. **Message Handler** (`synapse-telegram/src/handlers.rs`)
   - `handle_message()`: teloxide endpoint with dependency injection via `dptree` for `Arc<Config>`, `Arc<Agent>`, `Arc<Box<dyn SessionStore>>`, `ChatSessionMap`
   - `is_authorized(user_id, allowed_users)`: checks user ID against allowlist; returns `false` for empty list (secure by default)
   - `resolve_session(chat_id, config, storage, chat_map)`: read-lock fast path, write-lock double-check for race-safe session creation; sessions named `"tg:<chat_id>"`
   - `chunk_message(text)`: splits at paragraph (`\n\n`) > newline (`\n`) > space > hard boundary; ensures no chunk exceeds 4096 characters
   - `ChatSessionMap` type alias: `Arc<RwLock<HashMap<i64, Uuid>>>`

2. **Bot Entry Point** (rewrite of `synapse-telegram/src/main.rs`)
   - `resolve_bot_token(config)`: env var `TELEGRAM_BOT_TOKEN` > `telegram.token` in config; token never passed to any tracing macro
   - `rebuild_chat_map(storage)`: reconstructs the in-memory chat ID → session UUID map from `list_sessions()` on startup by filtering sessions whose name starts with `"tg:"`
   - `init_mcp_client(config_path)`: loads MCP config and returns `Some(McpClient)` if tools are available, `None` otherwise
   - Full initialization sequence: tracing → config → token → Bot → storage + cleanup → provider → MCP → Agent → chat map → Dispatcher → graceful shutdown
   - Graceful shutdown via `Arc::try_unwrap(agent)` after Dispatcher stops; logs warning on failure (acceptable; process exit drops connections)

### Modified Components

1. **`TelegramConfig` in `synapse-core/src/config.rs`**
   - New `TelegramConfig` struct: `token: Option<String>`, `allowed_users: Vec<u64>`
   - `Default` impl: `token: None`, `allowed_users: vec![]` (secure by default)
   - `pub telegram: Option<TelegramConfig>` field added to `Config` with `#[serde(default)]`
   - `Config::default()` updated to set `telegram: None`
   - Fully backward-compatible: existing config files without `[telegram]` section continue to work

2. **`synapse-core/src/lib.rs`**
   - `TelegramConfig` added to the `pub use config::{...}` re-export line

3. **`synapse-telegram/Cargo.toml`**
   - Added: `synapse-core` (path), `teloxide` 0.13 (macros), `tokio` (rt-multi-thread, macros), `anyhow`, `futures`, `uuid` (v4, v7), `tracing`, `tracing-subscriber` (env-filter), `async-trait` (dev), `chrono` (dev)

4. **`config.example.toml`**
   - Added commented-out `[telegram]` section with `token` and `allowed_users` examples, warning about committing tokens, and instructions for finding a Telegram user ID via `@userinfobot`

---

## Key Decisions

### 1. `agent.complete()` Instead of `stream_owned()`

**Decision:** The Telegram handler uses `agent.complete()` (non-streaming) for LLM interaction.

**Rationale:** Telegram messages are delivered atomically — there is no mechanism to update a message in-place as tokens arrive (unlike the CLI REPL). Streaming adds complexity without any user-facing benefit. `complete()` handles the tool call loop internally and returns the final text directly, making the handler simpler and more appropriate for the Telegram delivery model.

### 2. Session-Per-Chat with Name Convention `"tg:<chat_id>"`

**Decision:** Persist one session per Telegram chat ID, using the session name `"tg:<chat_id>"` as the mapping key.

**Rationale:** Reuses the existing `SessionStore` trait and `create_session()`/`list_sessions()` API without any new core abstractions. On startup, `rebuild_chat_map()` can reconstruct the in-memory routing map by filtering sessions whose name matches the `"tg:"` prefix, enabling multi-turn conversation continuity across bot restarts. The naming convention is human-readable and non-colliding with CLI sessions.

### 3. Double-Check Pattern for Concurrent Session Creation

**Decision:** Read-lock fast path to look up an existing session; if not found, create the session and then acquire the write lock with a re-check before inserting.

**Rationale:** The fast path (read lock) is taken on every message once a session exists, keeping concurrency high. The slow path (write lock + double-check) prevents duplicate insertions when two messages from the same chat arrive simultaneously before any session is created. The known limitation is that both tasks may call `create_session()`, creating one orphan row; the write-lock re-check ensures consistent routing to a single UUID. The orphan is eventually removed by the auto-cleanup job.

### 4. Empty `allowed_users` Rejects All Users (Secure by Default)

**Decision:** An empty `allowed_users` list (including when no `[telegram]` section is configured at all) silently drops all incoming messages.

**Rationale:** Prevents accidental open access if an operator deploys the bot without setting an allowlist. The `Default` impl for `TelegramConfig` produces `allowed_users: vec![]`, and the handler's `unwrap_or(&[])` fallback when `config.telegram` is `None` reinforces this. Silent drop (no reply) avoids revealing the bot's existence to unauthorized users.

### 5. `teloxide` 0.13 with `dptree` Dependency Injection

**Decision:** Use `teloxide` 0.13's `Dispatcher::builder()` with `dptree::deps![]` for handler dependency injection rather than closures or global state.

**Rationale:** `dptree` injects typed dependencies directly into the handler function signature, keeping `handle_message` easily testable (all inputs are explicit parameters). Shared state is wrapped in `Arc` (or `Arc<RwLock>` for mutable state) per Rust concurrency idiom. `enable_ctrlc_handler()` provides zero-boilerplate Ctrl+C support.

### 6. Hexagonal Architecture Validation

**Decision:** No new methods or traits were added to `synapse-core` for Telegram's benefit. The entire implementation uses the existing `Agent::complete()`, `SessionStore` CRUD, `Config::load()`, `create_provider()`, `create_storage()`, `McpClient::new()`, and `load_mcp_config()` APIs.

**Rationale:** This directly validates the hexagonal architecture goal from the project vision: multiple frontends sharing the same core logic. The `synapse-telegram` crate is a pure adapter, converting Telegram API interactions to and from the core's port interfaces. Zero business logic duplication between CLI and Telegram.

---

## Data Flow

### Authorized User Sends a Message (New Session)

```
1. Telegram API delivers Update with Message from user 123456789
2. teloxide Dispatcher routes to handle_message() with injected deps
3. is_authorized(123456789, allowed_users) -> true
4. msg.text() -> "What is Rust?"
5. resolve_session(chat_id): read-lock miss -> create Session("tg:<chat_id>") -> write-lock insert
6. storage.get_messages(session_id) -> [] (empty, new session)
7. Append user Message to messages vec; store StoredMessage in DB
8. bot.send_chat_action(Typing)
9. agent.complete(&mut messages) -> Message(Role::Assistant, "Rust is...")
10. storage.add_message(assistant StoredMessage)
11. chunk_message("Rust is...") -> ["Rust is..."] (single chunk)
12. bot.send_message(chat_id, "Rust is...")
13. return Ok(())
```

### Unauthorized User Sends a Message

```
1. Telegram API delivers Update from user 999999999
2. is_authorized(999999999, [123456789]) -> false
3. return Ok(()) -- silent drop, no reply, no error, no acknowledgment
```

### Bot Startup Token Resolution

```
TELEGRAM_BOT_TOKEN set:  env var -> Bot::new(env_token) -- config file token ignored
Config file only:        env var absent/empty -> config.telegram.token -> Bot::new(config_token)
Neither source:          resolve_bot_token() returns Err -> main() exits with error message
```

---

## Testing

### New Tests (17 total)

| Category | Count | Location |
|----------|-------|----------|
| TelegramConfig parsing | 4 | `synapse-core/src/config.rs` |
| User authorization | 3 | `synapse-telegram/src/handlers.rs` |
| Bot token resolution | 4 | `synapse-telegram/src/main.rs` |
| Chat map reconstruction | 3 | `synapse-telegram/src/main.rs` |
| Message chunking | 3 | `synapse-telegram/src/handlers.rs` |
| **Total new** | **17** | -- |

**Full regression suite: 167 tests total (17 new + 150 pre-existing), 0 failures**

### Quality Gate Results

| Check | Result |
|-------|--------|
| `cargo fmt --check` | PASS |
| `cargo clippy -- -D warnings` | PASS |
| `cargo test` (all crates) | PASS -- 167 tests, 0 failures |

### Manual Tests Required (Not Feasible in CI)

The end-to-end pipeline (authorized user sends message → LLM response delivered) requires a live bot token and LLM API key. The following must be verified manually before production deployment:

- Authorized user receives LLM response
- Unauthorized user receives no reply
- Multi-turn conversation persists across bot restart
- Bot token does not appear in logs at any level (including `RUST_LOG=debug`)
- Bot exits with clear error when no token is provided

---

## Files Changed

| File | Change Type | Description |
|------|-------------|-------------|
| `synapse-core/src/config.rs` | Modified | `TelegramConfig` struct, `telegram: Option<TelegramConfig>` on `Config`, `Default` impl, 4 new tests |
| `synapse-core/src/lib.rs` | Modified | `TelegramConfig` added to `pub use config::{...}` |
| `synapse-telegram/Cargo.toml` | Modified | All production and dev dependencies added |
| `synapse-telegram/src/main.rs` | Rewritten | Bot entry point: tracing init, config loading, token resolution, storage, provider, MCP, Agent, chat map, Dispatcher, graceful shutdown |
| `synapse-telegram/src/handlers.rs` | Created | Message handler, `resolve_session()`, `chunk_message()`, `is_authorized()`, handler tests |
| `config.example.toml` | Modified | `[telegram]` section added (commented out) |

### Files NOT Modified (Confirming PRD Constraint 1)

`synapse-core/src/agent.rs`, `provider.rs`, `storage.rs`, `mcp.rs`, `message.rs`, `session.rs`, `synapse-cli/` — all unchanged. The hexagonal architecture constraint is upheld.

---

## Module Structure

```
synapse-telegram/src/
  main.rs         # Bot entry point, resolve_bot_token(), rebuild_chat_map(), tests
  handlers.rs     # handle_message(), resolve_session(), chunk_message(), is_authorized(), tests

synapse-core/src/
  config.rs       # + TelegramConfig struct, Default impl, telegram: Option<TelegramConfig> on Config
  lib.rs          # + TelegramConfig re-export
```

---

## Known Limitations

1. **No end-to-end integration test.** `handle_message()` and the full message pipeline are not covered by automated tests. The teloxide Dispatcher requires a live Telegram connection. Manual smoke testing is mandatory before production deployment.

2. **Graceful shutdown `Arc::try_unwrap()` always fails.** Because the Dispatcher's active handler tasks hold `Arc<Agent>` clones, `Arc::try_unwrap(agent)` will return `Err` on every shutdown, triggering the `warn!` log. MCP connections are dropped via process exit (acceptable for v1 per plan).

3. **Concurrent session creation race.** Two simultaneous first messages from the same chat ID can create two database sessions; the write-lock double-check ensures only one UUID enters the map, but the second session becomes an orphan. Auto-cleanup will remove it. Impact: negligible.

4. **Non-text messages are silently ignored.** Photos, stickers, voice messages, and other non-text content receive no reply. No user feedback is sent. Acceptable for v1.

5. **Telegram rate limits not enforced.** No explicit rate limiting or backoff. For a personal/small-group bot this is acceptable; `teloxide::Throttle` adaptor is available for future use if needed.

---

## Future Work

- End-to-end integration tests using a test Telegram bot token and `MockProvider`
- Streaming delivery for Telegram (edit message as tokens arrive, requires polling or webhook mode)
- Inline keyboard buttons for common commands (`/new`, `/history`, `/help`)
- Support for non-text messages (image captioning, voice transcription)
- Webhook mode for production deployments (avoids long-polling latency)
- Per-chat model override command (`/model <provider>/<model>`)
- `teloxide::Throttle` adaptor for rate limiting in shared deployments
