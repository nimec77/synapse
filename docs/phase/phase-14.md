# Phase 14: File Logging

**Goal:** Production-ready file-based logging with rotation for the Telegram bot.

## Tasks

- [x] 14.1 Add `LoggingConfig` struct to `synapse-core/src/config.rs` with defaults
- [x] 14.2 Add `tracing-appender` dependency and `registry` feature to `synapse-telegram`
- [x] 14.3 Rewrite tracing init with layered subscriber (stdout + file appender)
- [x] 14.4 Update `config.example.toml` with `[logging]` section documentation
- [x] 14.5 Update `docs/idea.md`, `docs/vision.md`, and `docs/tasklist.md`

## Acceptance Criteria

**Test:** Add `[logging]` to config, start the bot, verify log files appear in the
configured directory with correct rotation and file count limits.

## Dependencies

- Phase 13 complete (System Prompt)

## Implementation Notes

The Telegram bot runs as a long-lived process on a VPS where stdout logs are lost on restart. File-based logging with rotation solves this.

Key design points:
- `LoggingConfig` is defined in `synapse-core/src/config.rs` and deserialized from the `[logging]` TOML section.
- Fields with defaults: `directory: String` ("logs"), `max_files: usize` (7), `rotation: String` ("daily").
- `synapse-telegram` uses `tracing-appender::rolling::daily()` (or hourly/never based on config) to create a file writer.
- A `tracing_subscriber::registry()` layered subscriber combines a stdout `fmt` layer and the file appender layer.
- The non-blocking writer handle must be kept alive for the process lifetime (`let _guard = ...`).
- File naming pattern: `synapse-telegram.YYYY-MM-DD.log` (tracing-appender appends the date suffix automatically).
- Only `synapse-telegram` gets file logging; `synapse-cli` continues with stdout only.
