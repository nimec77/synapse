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
- `max_sessions`: Maximum number of sessions to keep (default: 100)
- `retention_days`: Delete sessions older than this (default: 90)
- `auto_cleanup`: Enable automatic cleanup (default: true)

### Database Schema

Sessions and messages tables as defined in `docs/vision.md`:
- SQLite default at `~/.config/synapse/sessions.db`
- Configurable to PostgreSQL/MySQL via `DATABASE_URL`

### Cleanup Behavior

- Max sessions limit: oldest auto-deleted when exceeded
- Retention period: sessions older than `retention_days` purged
- Auto-cleanup runs on startup and periodically
