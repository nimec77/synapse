# Phase 12: Telegram Bot

**Goal:** Second interface using shared core.

## Tasks

- [ ] 12.1 Add `teloxide` to `synapse-telegram`
- [ ] 12.2 Create bot initialization with token from config
- [ ] 12.3 Implement message handler using `synapse-core` agent
- [ ] 12.4 Add session-per-chat persistence
- [ ] 12.5 Add user authorization via `allowed_users` allowlist

## Acceptance Criteria

- **Test:** Send message to bot, receive LLM response.
- **Auth test:** Messages from unlisted user IDs are silently dropped; only allowlisted users receive responses.

## Dependencies

- Phase 11 complete (MCP Integration)

## Implementation Notes

### `TelegramConfig` struct

```toml
[telegram]
token = "..."           # bot token (overridden by TELEGRAM_BOT_TOKEN env var)
allowed_users = [123456789, 987654321]  # Telegram user IDs (u64)
```

```rust
pub struct TelegramConfig {
    pub token: Option<String>,
    pub allowed_users: Vec<u64>,
}
```

### Bot token resolution priority

1. `TELEGRAM_BOT_TOKEN` environment variable (highest priority)
2. `telegram.token` in `config.toml`

### User authorization

- On every incoming message, check `msg.from().map(|u| u.id.0)` against `allowed_users`.
- **Secure-by-default:** an empty `allowed_users` list rejects all users.
- **Silent drop:** unauthorized messages are ignored without any reply (do not reveal the bot exists).

```rust
async fn handle_message(bot: Bot, msg: Message, cfg: Arc<Config>) -> ResponseResult<()> {
    let user_id = msg.from().map(|u| u.id.0).unwrap_or(0);
    if !cfg.telegram.allowed_users.contains(&user_id) {
        return Ok(()); // silent drop
    }
    // ... forward to agent
}
```
