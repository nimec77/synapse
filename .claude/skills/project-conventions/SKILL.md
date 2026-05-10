---
name: project-conventions
description: "Use when adding MCP tools, newtypes, error variants, or tests in this codebase and concrete placement rules plus copy-paste sources are needed beyond what CLAUDE.md summarizes"
---

# Project Conventions — telegram-connector

CLAUDE.md has the architecture overview. This skill points to the canonical example for each common change so you can copy-and-adapt rather than reinvent.

## Adding a new MCP tool

All 8 tools live in one `#[tool_router] impl McpServer<T, R>` block in `src/mcp/server.rs`. They cannot be split out (rmcp macro constraint).

- Pick the closest existing tool in `src/mcp/server.rs` as a template; copy its shape.
- Tool params live in `src/mcp/tools/`. They derive `serde::Deserialize` and `schemars::JsonSchema` (schemars **v1** — `#[derive(JsonSchema)]` from `schemars::JsonSchema`, NOT v0.8).
- Return type is always `Result<String, String>` with a JSON-serialized payload. This is an rmcp constraint.
- Add a test file under `src/mcp/tests/{topic}.rs` using `MockTelegramClientTrait` and `MockRateLimiterTrait` (mockall-generated).
- Before reading `server.rs`, run `ast-index outline src/mcp/server.rs` — it's large.

## Adding a newtype

Domain newtypes (`ChannelId`, `MessageId`, `UserId`, `Username`, `ChannelName`) live in `src/telegram/types/`. Use the closest existing one as a template — `ChannelId(i64)` for integer IDs, `Username(String)` for normalized strings.

For string-backed types, normalize in the constructor (e.g. strip a leading `@`). Derive `serde` and `schemars::JsonSchema` if the type appears in MCP tool params or responses.

## Adding an error variant

| Layer | Crate | File |
|---|---|---|
| Library / domain errors | `thiserror` | `src/error.rs` |
| Application top level (CLI, main loop) | `anyhow` with `.context("…")` | `src/main.rs`, top-level handlers |

Never `unwrap()` in production. `expect()` only in tests or genuinely unreachable cases.

## Test placement

| Where | Pattern |
|---|---|
| Small unit tests for one module | inline `#[cfg(test)] mod tests { … }` at bottom of file |
| Larger suite for one module | `#[path = "module/tests.rs"] mod tests;` (e.g. `config.rs` → `config/tests.rs`) |
| MCP tool tests | new file in `src/mcp/tests/{topic}.rs` |
| Telegram client tests (mock-based) | `src/telegram/tests/client_tests.rs` |
| Test fixtures | reuse `src/test_helpers.rs` (`create_test_message`, `create_test_channel`, …) |

Config tests mutate env vars and **must** run serial: `cargo test config -- --test-threads=1`.

## Sensitive fields

Wrap secrets in `secrecy::SecretString`. Access via `.expose_secret()`. Never log them, never include them in `Debug` output of structs you log.

## Pre-commit gate

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

All three must pass. The user creates commits — you don't (CLAUDE.md, "Critical Rules").

## Extending the trait-based DI

`McpServer<T: TelegramClientTrait, R: RateLimiterTrait>` is the production seam. To add a new client capability:

1. Add the method signature to `TelegramClientTrait` in `src/telegram/trait_def.rs`.
2. Implement it on `TelegramClient` in `src/telegram/client.rs`.
3. `mockall` regenerates `MockTelegramClientTrait` automatically — use it in new test files.
