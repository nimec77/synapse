# Completed Tickets

Historical index of shipped tickets. The authoritative changelog is `CHANGELOG.md` at the
repository root — this file is a short-form lookup table. Active ticket: `docs/.active_ticket`.

| Ticket | Description | Summary |
|--------|-------------|---------|
| SY-1 | Project Foundation | Workspace structure with 3 crates |
| SY-2 | CI/CD Pipeline | GitHub Actions with check + audit jobs |
| SY-3 | Echo CLI | CLI with clap, one-shot and stdin input modes |
| SY-4 | Configuration | TOML config loading with multi-location priority |
| SY-5 | Provider Abstraction | LlmProvider trait, Message/Role types, MockProvider |
| SY-6 | Anthropic Provider | AnthropicProvider with Claude API, async CLI with tokio |
| SY-7 | DeepSeek Provider | DeepSeekProvider with OpenAI-compatible API, provider factory pattern |
| SY-8 | Streaming Responses | Token-by-token streaming via SSE, DeepSeekProvider streaming, Ctrl+C handling |
| SY-9 | Session Storage | SQLite persistence, SessionStore trait, session commands, auto-cleanup |
| SY-10 | CLI REPL | Interactive TUI with ratatui/crossterm, multi-turn conversations, streaming, session resume |
| SY-11 | MCP Integration | Agent struct with tool call loop, MCP client via rmcp, tool discovery |
| SY-13 | Telegram Bot | teloxide bot, session-per-chat persistence, user allowlist auth, TelegramConfig |
| SY-14 | System Prompt | `system_prompt` in Config and Agent, `build_messages()` on-the-fly injection |
| SY-15 | File Logging | `LoggingConfig` in core, `tracing-appender` layered subscriber in Telegram bot |
| SY-16 | Code Refactoring | Dead code removal, `openai_compat.rs` shared base, magic-string constants, structured tracing, `Agent::from_config()`, `init_mcp_client()` in core, REPL file split, API surface tightened |
| SY-17 | Telegram Markdown Formatting | `format.rs` with `md_to_telegram_html` + `chunk_html`; HTML parse mode with plain-text fallback in handlers |
| SY-18 | Telegram Bot Commands | `max_tokens: u32` in `Config` (serde default 4096); `/help`, `/new`, `/history`, `/list`, `/switch N`, `/delete N` commands; `ChatSessions` multi-session struct; branched dispatcher; `max_sessions_per_chat` config cap |
| SY-19 | Telegram Command Fixes & Interactive Keyboards | Fix `/switch`/`/delete` fall-through (`String` instead of `usize`); `parse_session_arg`; `/start` command; defensive guard in `handle_message`; inline keyboards with `InlineKeyboardMarkup`; `handle_callback` with `do_switch`/`do_delete` shared logic; dispatcher extended to handle `CallbackQuery` |
| SY-20 | Improve /history Command | `/history` now shows last 10 user/assistant messages only (filtered from full history), truncated to 150 chars with `...`; `truncate_content` and `format_history` extracted as pure helpers for testability |
| SY-21 | Code Refactoring II | Unified `text::truncate` in core; `OpenAiCompatProvider` struct in `openai_compat.rs` (DeepSeek/OpenAI are thin wrappers); extracted submodules: `openai_compat/types.rs`, `openai_compat/tests.rs`, `anthropic/types.rs`, `config/tests.rs`, `storage/sqlite/tests.rs`, `commands/keyboard.rs`, `commands/tests.rs`, `format/chunk.rs`, `startup.rs`, `startup/tests.rs` |
| SY-22 | DeepSeek V4 Pro Reasoning Model | `Message.reasoning_content`, `StreamEvent::ReasoningDelta` + `#[non_exhaustive]`, wire-format extensions in `openai_compat` (`ThinkingConfig`, `ReasoningSettings`, `thinking`/`reasoning_effort` request fields), reasoning-aware DeepSeek factory with effort auto-escalation, agent loop preservation of reasoning on tool-call turns, SQLite migration + storage round-trip, REPL dim/italic rendering + one-shot stderr routing, Telegram `<blockquote>` rendering with `show_reasoning` flag (merged on `master`; release pending — see `[Unreleased]` in `CHANGELOG.md`) |
