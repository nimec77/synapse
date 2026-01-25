# Research: SY-9 - Phase 8: Session Storage

## Resolved Questions

The PRD states "None - Implementation is complete and ready for final verification."

Based on code analysis, the implementation appears substantially complete. The remaining work identified in the PRD:
1. Update `docs/tasklist.md` to mark tasks as complete
2. Final integration testing
3. Update CHANGELOG.md

---

## Related Modules and Services

### synapse-core Structure

| File | Purpose | Status |
|------|---------|--------|
| `synapse-core/src/session.rs` | `Session`, `SessionSummary`, `StoredMessage` structs | Implemented |
| `synapse-core/src/storage.rs` | `SessionStore` trait, `StorageError`, `CleanupResult` | Implemented |
| `synapse-core/src/storage/sqlite.rs` | `SqliteStore` implementation, `create_storage()` factory | Implemented |
| `synapse-core/src/config.rs` | `SessionConfig` with cleanup settings | Implemented |
| `synapse-core/src/lib.rs` | Public exports for session/storage types | Implemented |
| `synapse-core/migrations/20250125_001_initial.sql` | Database schema (sessions + messages tables) | Implemented |

### synapse-cli Structure

| File | Purpose | Status |
|------|---------|--------|
| `synapse-cli/src/main.rs` | CLI with session management subcommands | Implemented |

---

## Current Endpoints and Contracts

### Session Types (synapse-core/src/session.rs)

```rust
/// A conversation session containing metadata.
pub struct Session {
    pub id: Uuid,
    pub name: Option<String>,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Summary information for listing sessions.
pub struct SessionSummary {
    pub id: Uuid,
    pub name: Option<String>,
    pub provider: String,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
    pub preview: Option<String>,  // First user message, truncated to 50 chars
}

/// A message stored in the database with full metadata.
pub struct StoredMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}
```

### SessionStore Trait (synapse-core/src/storage.rs)

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: &Session) -> Result<(), StorageError>;
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError>;
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError>;
    async fn touch_session(&self, id: Uuid) -> Result<(), StorageError>;
    async fn delete_session(&self, id: Uuid) -> Result<bool, StorageError>;
    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError>;
    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError>;
    async fn cleanup(&self, config: &SessionConfig) -> Result<CleanupResult, StorageError>;
}
```

### StorageError Enum

```rust
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(String),

    #[error("session not found: {0}")]
    NotFound(Uuid),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("invalid data: {0}")]
    InvalidData(String),
}
```

### CleanupResult

```rust
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CleanupResult {
    pub sessions_deleted: u32,
    pub by_max_limit: u32,
    pub by_retention: u32,
}
```

### SessionConfig (synapse-core/src/config.rs)

```rust
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_max_sessions")]  // 100
    pub max_sessions: u32,

    #[serde(default = "default_retention_days")]  // 90
    pub retention_days: u32,

    #[serde(default = "default_auto_cleanup")]  // true
    pub auto_cleanup: bool,
}
```

### CLI Commands

```bash
# Continue existing session
synapse --session <uuid> "message"
synapse -s <uuid> "message"

# Session management
synapse sessions list
synapse sessions show <uuid>
synapse sessions delete <uuid>
```

---

## Database Schema

Located in `synapse-core/migrations/20250125_001_initial.sql`:

```sql
-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    system_prompt TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
```

Key features:
- CASCADE delete for messages when session is deleted
- Indexes on session_id, timestamp, and updated_at for performance
- Foreign key enforcement via `PRAGMA foreign_keys = ON`

---

## Patterns Used

### Hexagonal Architecture (Ports and Adapters)

The implementation follows the pattern defined in `docs/vision.md`:
- **Port**: `SessionStore` trait in `storage.rs`
- **Adapter**: `SqliteStore` in `storage/sqlite.rs`
- **Factory**: `create_storage()` function for creating storage backends

### Builder Pattern for Session

```rust
let session = Session::new("deepseek", "deepseek-chat")
    .with_name("My Chat")
    .with_system_prompt("You are helpful.");
```

### Factory Pattern for Storage

```rust
pub async fn create_storage(
    database_url: Option<&str>,
) -> Result<Box<dyn SessionStore>, StorageError>
```

Default URL: `sqlite:~/.config/synapse/sessions.db`

### WAL Mode for SQLite

Enabled in `SqliteStore::new()` for better concurrent access:
```rust
let options = SqliteConnectOptions::new()
    .filename(url)
    .create_if_missing(true)
    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
```

### Connection Pooling

```rust
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(options)
    .await
```

### Auto-Cleanup on Startup

```rust
// In CLI main.rs
let session_config = config.session.clone().unwrap_or_default();
if session_config.auto_cleanup {
    let _ = storage.cleanup(&session_config).await;
}
```

---

## Dependencies

### synapse-core/Cargo.toml

```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
dirs = "6.0.0"
thiserror = "2"
```

### synapse-cli/Cargo.toml

```toml
[dependencies]
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
```

---

## Implementation Analysis

### Task 8.1: Session Struct - COMPLETE

`synapse-core/src/session.rs` contains:
- `Session` struct with all required fields (id, name, provider, model, system_prompt, timestamps)
- `SessionSummary` struct for list views
- `StoredMessage` struct for message persistence
- Builder methods (`with_name`, `with_system_prompt`)
- Unit tests (10 tests)

### Task 8.2: SessionStore Trait - COMPLETE

`synapse-core/src/storage.rs` contains:
- `SessionStore` trait with all CRUD operations
- `StorageError` enum with appropriate variants
- `CleanupResult` struct
- Unit tests for error display and CleanupResult

### Task 8.3: SQLite Storage - COMPLETE

`synapse-core/src/storage/sqlite.rs` contains:
- `SqliteStore` struct with connection pooling
- All `SessionStore` trait methods implemented
- `create_storage()` factory function
- WAL mode enabled
- Auto-migration on startup
- Role parsing/serialization helpers
- Comprehensive tests (20 tests)

### Task 8.4: Schema Migrations - COMPLETE

`synapse-core/migrations/20250125_001_initial.sql` contains:
- Sessions and messages tables
- Foreign key with CASCADE delete
- Performance indexes
- sqlx migration format

### Task 8.5: CLI Integration - COMPLETE

`synapse-cli/src/main.rs` contains:
- `--session` / `-s` flag for continuing sessions
- `synapse sessions list` subcommand
- `synapse sessions show <id>` subcommand
- `synapse sessions delete <id>` subcommand
- Message persistence after each exchange
- Session creation for new conversations
- Conversation history loading for existing sessions

### Task 8.6: Session Limits - COMPLETE

`SqliteStore::cleanup()` implements:
- Delete sessions exceeding `max_sessions` (oldest first)
- Delete sessions older than `retention_days`
- Returns `CleanupResult` with counts

### Task 8.7: Auto-Cleanup - COMPLETE

CLI `main.rs` runs cleanup on startup when `auto_cleanup` is enabled.

---

## Limitations and Risks

### Limitations

1. **SQLite Only**: No PostgreSQL/MySQL support yet (documented as future enhancement)
2. **No Encryption**: Messages stored in plaintext
3. **Single-User**: No authentication or multi-tenant support
4. **Full History Load**: All messages loaded into memory for context (may be slow for very long sessions)
5. **No DATABASE_URL from Config**: `create_storage()` only accepts URL parameter, doesn't read from config

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| R-1: Database corruption | Low | High | WAL mode enabled, sqlx transactions |
| R-2: Large session history | Medium | Medium | Current implementation loads all; future pagination |
| R-3: Migration conflicts | Low | Medium | sqlx versioned migrations |
| R-4: Storage path access | Low | Low | Configurable via DATABASE_URL env var |

---

## New Technical Questions

Questions discovered during research:

1. **DATABASE_URL Priority**: The PRD mentions database URL resolution priority (env > config > default), but `create_storage()` only takes an optional URL parameter and falls back to default. The CLI always calls `create_storage(None)`. Should config file `session.database_url` be implemented?
   - **Current Status**: Not implemented, uses hardcoded default path
   - **Recommendation**: Document as future enhancement or implement if needed

2. **Session ID Display**: After creating a new session, the CLI doesn't display the session ID. Users need to run `synapse sessions list` to find it.
   - **Recommendation**: Consider printing session ID on first exchange for UX

3. **UUID Version**: The PRD and phase docs mention UUID v8 (RFC 9562), but implementation uses UUID v4 (`Uuid::new_v4()`).
   - **Current Status**: Using v4
   - **Impact**: Low - both are valid, v4 is widely supported

4. **Test Coverage**: The sqlite tests use temporary databases but some tests may leave artifacts.
   - **Current Status**: Tests use `temp_dir()` with random UUIDs
   - **Impact**: Low - cleanup is automatic via OS temp handling

---

## Remaining Work

Based on PRD and code analysis:

1. **Update `docs/tasklist.md`**: Mark all Phase 8 tasks as complete (checkboxes)
2. **Update CHANGELOG.md**: Add SY-9 session storage entry
3. **Integration Testing**: Verify end-to-end flow manually
4. **Optional Enhancements**:
   - Print session ID after first exchange
   - Implement `session.database_url` config option
   - Consider UUID v8 if sortability is desired

---

## Files Modified/Created

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/Cargo.toml` | Modified | Added sqlx, uuid, chrono dependencies |
| `synapse-core/src/session.rs` | Created | Session, SessionSummary, StoredMessage types |
| `synapse-core/src/storage.rs` | Created | SessionStore trait, StorageError, CleanupResult |
| `synapse-core/src/storage/sqlite.rs` | Created | SqliteStore implementation |
| `synapse-core/src/config.rs` | Modified | Added SessionConfig |
| `synapse-core/src/lib.rs` | Modified | Export session and storage types |
| `synapse-core/migrations/20250125_001_initial.sql` | Created | Database schema |
| `synapse-cli/Cargo.toml` | Modified | Added uuid, chrono dependencies |
| `synapse-cli/src/main.rs` | Modified | Session management CLI commands |
| `config.example.toml` | Modified | Added [session] configuration section |

---

## Test Coverage

### Unit Tests (synapse-core)

**session.rs** (10 tests):
- `test_session_new`
- `test_session_with_name`
- `test_session_with_system_prompt`
- `test_session_field_access`
- `test_session_summary_construction`
- `test_stored_message_new`
- `test_stored_message_field_access`
- `test_session_clone`
- `test_stored_message_clone`

**storage.rs** (4 tests):
- `test_storage_error_display`
- `test_cleanup_result_default`
- `test_cleanup_result_construction`
- `test_cleanup_result_clone`

**storage/sqlite.rs** (20 tests):
- `test_role_to_string`
- `test_parse_role`
- `test_create_and_get_session`
- `test_get_session_not_found`
- `test_list_sessions`
- `test_touch_session`
- `test_touch_session_not_found`
- `test_delete_session`
- `test_delete_session_not_found`
- `test_add_and_get_messages`
- `test_messages_ordered_by_timestamp`
- `test_list_sessions_with_message_count_and_preview`
- `test_preview_truncated_to_50_chars`
- `test_cascade_delete`
- `test_cleanup_by_max_sessions`
- `test_cleanup_no_action_when_under_limit`
- `test_cleanup_result_counts`

**config.rs** (includes SessionConfig tests):
- `test_session_config_defaults`
- `test_config_without_session_section`
- `test_config_with_session_section`
- `test_session_config_partial_defaults`

### CLI Tests (synapse-cli)

**main.rs** (5 tests):
- `test_args_parse`
- `test_args_with_session`
- `test_args_session_short_flag`
- `test_truncate`

---

## References

- `docs/prd/SY-9.prd.md` - PRD document
- `docs/phase/phase-8.md` - Phase task breakdown
- `docs/vision.md` - Architecture patterns and database schema
- `config.example.toml` - Session configuration options
- [sqlx documentation](https://docs.rs/sqlx/)
- [uuid crate documentation](https://docs.rs/uuid/)
