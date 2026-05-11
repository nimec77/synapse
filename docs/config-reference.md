# Config Reference

Authoritative source: `synapse-core/src/config.rs`. This file is a flat field-by-field reference;
load-bearing invariants (priority order, the inline-wins rule, secure defaults) live in `CLAUDE.md`.

## Load priority

1. `--config <path>` CLI flag (error if missing)
2. `./config.toml`
3. `~/.config/synapse/config.toml`
4. Error — no silent defaults

## Top-level `Config`

| Field | Type | Default | Notes |
|---|---|---|---|
| `provider` | `String` | — | `"deepseek"` / `"anthropic"` / `"openai"` |
| `api_key` | `Option<String>` | `None` | Env var wins; see provider section below |
| `model` | `String` | — | Model identifier per provider |
| `max_tokens` | `u32` | `4096` | Passed to every provider call |
| `system_prompt` | `Option<String>` | `None` | Injected on-the-fly; never stored in DB |
| `system_prompt_file` | `Option<String>` | `None` | External file path; **inline `system_prompt` wins if both set** |
| `reasoning_effort` | `Option<String>` | `None` | `"low"` / `"medium"` / `"high"` / `"max"`; only applied for DeepSeek + `REASONING_MODELS`; defaults to `"high"` internally |
| `session` | `Option<SessionConfig>` | `None` | See SQLite section in CLAUDE.md |
| `mcp` | `Option<McpConfig>` | `None` | MCP server list |
| `telegram` | `Option<TelegramConfig>` | `None` | Telegram bot only |
| `logging` | `Option<LoggingConfig>` | `None` | Omit → stdout only |
| `cli` | `Option<CliConfig>` | `None` | CLI display preferences |

## `TelegramConfig`

| Field | Type | Default | Notes |
|---|---|---|---|
| `token` | `Option<String>` | `None` | **`TELEGRAM_BOT_TOKEN` env wins** |
| `allowed_users` | `Vec<u64>` | `[]` | **Empty rejects all (secure by default)** |
| `max_sessions_per_chat` | `u32` | `10` | Manual `impl Default` (not derived) to keep serde and `Default::default()` in sync |
| `show_reasoning` | `bool` | `false` | Privacy-preserving; `true` → `<blockquote>` above the answer |

## `CliConfig`

| Field | Type | Default | Notes |
|---|---|---|---|
| `show_reasoning` | `bool` | `true` | CLI is developer-facing |

## `LoggingConfig`

| Field | Type | Default | Notes |
|---|---|---|---|
| `directory` | `String` | `"logs"` | Rolling-file directory |
| `max_files` | `usize` | `7` | Oldest deleted when exceeded |
| `rotation` | `Rotation` (enum) | `Daily` | `"daily"` / `"hourly"` / `"never"` |

## API key env vars (override config file)

- `DEEPSEEK_API_KEY` — for `provider = "deepseek"`
- `ANTHROPIC_API_KEY` — for `provider = "anthropic"`
- `OPENAI_API_KEY` — for `provider = "openai"`

## Database URL priority

1. `DATABASE_URL` env var
2. `session.database_url` in config
3. Default: `sqlite:~/.config/synapse/sessions.db`
