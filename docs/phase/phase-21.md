# Phase 21: Code Refactoring II (SY-21)

**Goal:** Eliminate accumulated duplication, magic values, dead code, and oversized modules without changing external behaviour.

## Tasks

- [ ] 21.1 Unified `truncate` in synapse-core
  - Create `synapse-core/src/text.rs` with a single char-safe `truncate(s, max_chars)` function
  - Delete duplicate truncation logic from `synapse-cli/src/commands.rs`, `synapse-telegram/src/commands.rs`, and `synapse-core/src/storage/sqlite.rs`
  - Remove duplicate `test_truncate` unit test from `synapse-cli/src/main.rs`

- [ ] 21.2 Consolidate DeepSeek/OpenAI into `OpenAiCompatProvider`
  - Add a generic `OpenAiCompatProvider` struct to `openai_compat.rs` that holds base URL, API key, model, and max tokens
  - Implement all `LlmProvider` methods on `OpenAiCompatProvider` using the shared helpers
  - Rewrite `deepseek.rs` and `openai.rs` as newtypes delegating all trait methods to `OpenAiCompatProvider`

- [ ] 21.3 CLI session helper deduplication
  - Extract `load_or_create_session()` shared between `main.rs` and `repl.rs` into a common helper
  - Extract `init_storage()` consolidating storage initialisation + cleanup (duplicated twice in `main.rs`)

- [ ] 21.4 Telegram deduplication
  - Extract `check_auth(msg, allowed_users)` helper to replace inline auth checks in `handlers.rs` and `commands.rs`
  - Extract `tg_session_name(chat_id)` helper replacing ad-hoc `format!("tg:{chat_id}")` strings
  - Add `NO_SESSIONS_HINT` constant for the repeated "no sessions" reply string
  - Unify `TELEGRAM_MSG_LIMIT` so `format.rs` and `commands.rs` share a single constant
  - Merge `cmd_switch_keyboard` and `cmd_delete_keyboard` into a single `build_action_keyboard(action, sessions)` helper
  - Have `cmd_list` call `fetch_chat_sessions` instead of duplicating session-fetch logic

- [ ] 21.5 Dead code removal and constant extraction
  - Remove `stream_with_tools()` from the `LlmProvider` trait (never called by `Agent`)
  - Extract named constants: `PREVIEW_MAX_CHARS`, `HISTORY_MESSAGE_LIMIT`, `LIST_PREVIEW_MAX_CHARS`, `KEYBOARD_PREVIEW_MAX_CHARS`

- [ ] 21.6 Type safety improvements
  - Replace `rotation: String` in `LoggingConfig` with a `Rotation` enum (`Daily`, `Hourly`, `Never`) and derive `Deserialize`
  - Remove `Arc<Box<dyn SessionStore>>` double indirection in the Telegram crate; use `Arc<dyn SessionStore>` directly

- [ ] 21.7 Small polish
  - Extract the REPL layout split ratio into a named constant (e.g. `REPL_LAYOUT_SPLIT`)
  - Add `ChatSessions::new(session_id)` constructor to replace repeated struct literal construction
  - Replace `Ok(r) | Err(r) => r` ad-hoc patterns with `.unwrap_or_else(|e| e)` where applicable

- [ ] 21.8 Split large modules
  - Split `synapse-telegram/src/commands.rs` (706 lines) into `commands.rs` (slash command handlers) + `commands/keyboard.rs` (keyboard builders and callback logic)
  - Split `synapse-telegram/src/format.rs` (447 lines) into `format.rs` (`md_to_telegram_html`, `escape_html`) + `format/chunk.rs` (`chunk_html`, tag-balancing internals)
  - Split `synapse-core/src/provider/anthropic.rs` (407 lines) into `anthropic.rs` (provider impl) + `anthropic/types.rs` (serde request/response structs)

## Acceptance Criteria

**Test:** `cargo clippy -- -D warnings` passes with no new warnings; `cargo test` green; no public API surface regressions.

## Dependencies

- Phase 20 complete

## Implementation Notes

### Truncation (21.1)

Char-safe truncation already exists in `synapse-core/src/storage/sqlite.rs` (introduced in the bugfix commit `3a4e33a`). Move it to `synapse-core/src/text.rs` and re-export from `lib.rs` so all three crates can import it from a single location.

### OpenAiCompatProvider (21.2)

`openai_compat.rs` already contains all the shared serde types and request helpers (`build_api_messages`, `complete_request`, `stream_sse`). The remaining step is wrapping these in a concrete `OpenAiCompatProvider` struct that owns the configuration, then having `DeepSeekProvider` and `OpenAIProvider` become thin newtypes (or type aliases) over it.

### Module Splits (21.8)

Use the new Rust module system (no `mod.rs`). For example, splitting `commands.rs`:
```
synapse-telegram/src/
├── commands.rs          # slash command handlers only
└── commands/
    └── keyboard.rs      # build_action_keyboard, handle_callback, do_switch, do_delete
```
Declare the submodule in `commands.rs` with `mod keyboard;` and re-export public symbols as needed.
