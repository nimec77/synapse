# Phase 18: Telegram Bot Commands (SY-18)

**Goal:** Add slash command support to the Telegram bot — `/help`, `/new`, `/history`, `/list`, `/switch N`, `/delete N` — with multi-session management and a per-chat session cap.

## Tasks

- [x] 18.1 Add `max_sessions_per_chat: u32` to `TelegramConfig` in `synapse-core/src/config.rs` (serde default `10`)
- [x] 18.2 Move `chrono` from dev-dependencies to dependencies in `synapse-telegram/Cargo.toml`
- [x] 18.3 Create `synapse-telegram/src/commands.rs` — `Command` enum (BotCommands derive) + 6 handlers: `/help`, `/new`, `/history`, `/list`, `/switch N`, `/delete N`
- [x] 18.4 Update `synapse-telegram/src/main.rs` — branched dispatcher, `Me` injection, `set_my_commands`, `rebuild_chat_map` multi-session fix
- [x] 18.5 Add per-chat session cap enforcement in `/new` command (auto-delete oldest when over `max_sessions_per_chat`)
- [x] 18.6 Add unit tests for command logic, `rebuild_chat_map` multi-session, config deserialization

## Acceptance Criteria

**Test:** Send `/help`, `/new`, `/list`, `/switch 1`, `/delete 1` to the bot and verify correct responses; verify session cap evicts oldest session when exceeded.

## Dependencies

- Phase 17 complete

## Implementation Notes

- `Command` enum derives `teloxide::utils::command::BotCommands`; each variant maps to a handler function
- `/switch N` and `/delete N` take a 1-based index into the session list for the current chat
- `rebuild_chat_map` must support multiple sessions per chat (currently assumes one session per chat ID)
- `max_sessions_per_chat` default of `10` enforced in `/new`; oldest session (by `last_active`) is deleted when the cap is reached
- `chrono` is needed at runtime (not just in tests) for session timestamp formatting in `/history` and `/list` output
