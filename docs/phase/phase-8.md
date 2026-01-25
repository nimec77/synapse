# Phase 8: Session Storage

**Goal:** Persist conversations to SQLite.

## Tasks

- [ ] 8.1 Create `synapse-core/src/session.rs` with `Session` struct
- [ ] 8.2 Create `synapse-core/src/storage.rs` with `SessionStore` trait
- [ ] 8.3 Add `sqlx` (sqlite feature), create `storage/database.rs`
- [ ] 8.4 Implement schema migrations (sessions + messages tables)
- [ ] 8.5 Wire storage into CLI: save messages after each exchange
- [ ] 8.6 Implement session limits (max sessions, retention period)
- [ ] 8.7 Add automatic cleanup job for expired sessions

## Acceptance Criteria

**Test:** `synapse sessions list` shows previous conversations.

## Dependencies

- Phase 7 complete (Streaming Responses)

## Implementation Notes

### Session Storage Config

From `config.example.toml`:
- `database_url`: Database connection URL (optional, env var takes priority)
- `max_sessions`: Maximum number of sessions to keep (default: 100)
- `retention_days`: Delete sessions older than this (default: 90)
- `auto_cleanup`: Enable automatic cleanup (default: true)

### Database URL Resolution

Priority order:
1. `DATABASE_URL` environment variable (highest priority)
2. `session.database_url` in config.toml
3. Default: `sqlite:~/.config/synapse/sessions.db`

### Session IDs

- UUID v8 (RFC 9562) - modern replacement for v4
- Allows timestamp prefix for sortability
- Maintains universally unique properties

### Database Schema

Sessions and messages tables as defined in `docs/vision.md`:
- SQLite default at `~/.config/synapse/sessions.db`
- Configurable to PostgreSQL/MySQL via DATABASE_URL or config

### Cleanup Behavior

- Max sessions limit: oldest auto-deleted when exceeded
- Retention period: sessions older than `retention_days` purged
- Auto-cleanup runs on startup and periodically
