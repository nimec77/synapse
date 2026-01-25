# Tasklist: SY-9 - Phase 8: Session Storage

Status: TASKLIST_READY

## Context

Phase 8 implements persistent conversation storage using SQLite. Based on code analysis, all core implementation tasks (8.1-8.7) are complete. The remaining work is documentation updates and verification.

## Tasks

- [ ] 8.1 Verify Session struct implementation
  - Acceptance: `Session`, `SessionSummary`, `StoredMessage` structs exist in `synapse-core/src/session.rs` with proper fields (id, name, provider, model, system_prompt, timestamps)
  - Acceptance: Unit tests pass for session creation and builder methods

- [ ] 8.2 Verify SessionStore trait implementation
  - Acceptance: `SessionStore` trait exists in `synapse-core/src/storage.rs` with CRUD methods (create_session, get_session, list_sessions, touch_session, delete_session, add_message, get_messages, cleanup)
  - Acceptance: `StorageError` enum has variants: Database, NotFound, Migration, InvalidData

- [ ] 8.3 Verify SQLite storage implementation
  - Acceptance: `SqliteStore` in `synapse-core/src/storage/sqlite.rs` implements `SessionStore` trait
  - Acceptance: `create_storage(None)` defaults to `~/.config/synapse/sessions.db`

- [ ] 8.4 Verify schema migrations
  - Acceptance: Migration file exists at `synapse-core/migrations/20250125_001_initial.sql`
  - Acceptance: Schema has `sessions` and `messages` tables with proper indexes and CASCADE delete

- [ ] 8.5 Verify CLI integration
  - Acceptance: `synapse --session <uuid> "message"` continues an existing session
  - Acceptance: `synapse sessions list` displays all sessions
  - Acceptance: `synapse sessions show <uuid>` displays session messages
  - Acceptance: `synapse sessions delete <uuid>` removes session

- [ ] 8.6 Verify session limits
  - Acceptance: `SessionConfig` has `max_sessions` (default 100) and `retention_days` (default 90)
  - Acceptance: `cleanup()` method deletes sessions exceeding max_sessions limit

- [ ] 8.7 Verify automatic cleanup
  - Acceptance: CLI runs cleanup on startup when `auto_cleanup: true`
  - Acceptance: Sessions older than `retention_days` are purged

- [ ] 8.8 Update docs/tasklist.md
  - Acceptance: Phase 8 tasks 8.1-8.7 marked as complete (`[x]`)
  - Acceptance: Progress table shows "Phase 8: Session Storage | ✅ Complete | 7/7"

- [ ] 8.9 Update CHANGELOG.md
  - Acceptance: SY-9 entry added under `[Unreleased]` section
  - Acceptance: Entry lists: Session/SessionSummary/StoredMessage types, SessionStore trait, SqliteStore, migrations, CLI commands, cleanup

- [ ] 8.10 Run integration tests
  - Acceptance: `cargo test` passes for all session/storage tests
  - Acceptance: `cargo clippy` passes without warnings

- [x] 8.11 Add `database_url` field to `SessionConfig`
  - Acceptance: `SessionConfig` has `database_url: Option<String>` field
  - Acceptance: Field is documented with priority order comment
  - Acceptance: TOML parsing test exists for `session.database_url`

- [x] 8.12 Implement DATABASE_URL environment variable priority in `create_storage()`
  - Acceptance: `create_storage()` checks `DATABASE_URL` env var first
  - Acceptance: Falls back to config parameter, then default path
  - Acceptance: Function signature uses `config_database_url` parameter name

- [x] 8.13 Update CLI to pass config database_url to storage
  - Acceptance: `main()` passes `session_config.database_url.as_deref()` to `create_storage()`
  - Acceptance: `handle_command()` loads config and passes database_url

- [x] 8.14 Update UUID version to v7 for sortable session IDs
  - Acceptance: `synapse-core/Cargo.toml` includes `v7` feature for uuid
  - Acceptance: `Session::new()` uses `Uuid::now_v7()` instead of `Uuid::new_v4()`
  - Acceptance: `StoredMessage::new()` uses `Uuid::now_v7()` instead of `Uuid::new_v4()`
