# Implementation Plan: SY-9 - Phase 8: Session Storage

Status: PLAN_APPROVED

## Overview

This plan documents the implementation of persistent conversation storage using SQLite for the Synapse CLI. The feature enables users to maintain context across sessions, manage conversation history, and continue previous conversations by session ID.

Based on code analysis, all implementation tasks (8.1-8.7) are complete. This plan documents the existing implementation and identifies remaining documentation/testing work.

---

## Components

### 1. Session Data Types (`synapse-core/src/session.rs`)

Contains the core session-related data structures.

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

**Features:**
- Builder pattern for Session (`with_name`, `with_system_prompt`)
- UUID v4 for unique identification
- DateTime<Utc> for timezone-aware timestamps

### 2. SessionStore Trait (`synapse-core/src/storage.rs`)

Port definition following hexagonal architecture.

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

**Supporting types:**

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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CleanupResult {
    pub sessions_deleted: u32,
    pub by_max_limit: u32,
    pub by_retention: u32,
}
```

### 3. SqliteStore Implementation (`synapse-core/src/storage/sqlite.rs`)

Adapter implementing the SessionStore trait for SQLite.

```rust
pub struct SqliteStore {
    pool: Pool<Sqlite>,
}

impl SqliteStore {
    pub async fn new(url: &str) -> Result<Self, StorageError>;
}

pub async fn create_storage(
    database_url: Option<&str>,
) -> Result<Box<dyn SessionStore>, StorageError>;
```

**Features:**
- WAL mode for better concurrent access
- Connection pooling (max 5 connections)
- Auto-migration on startup
- Factory function for creating storage instances
- Default path: `~/.config/synapse/sessions.db`

### 4. SessionConfig (`synapse-core/src/config.rs`)

Configuration options for session storage.

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

### 5. Database Schema (`synapse-core/migrations/20250125_001_initial.sql`)

SQLite schema with CASCADE delete and performance indexes.

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

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
```

### 6. CLI Integration (`synapse-cli/src/main.rs`)

CLI commands for session management.

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

## API Contract

### Session Operations

| Operation | Method | Description |
|-----------|--------|-------------|
| Create session | `create_session(&Session)` | Creates new session record |
| Get session | `get_session(Uuid)` | Returns session or None |
| List sessions | `list_sessions()` | Returns all sessions with summary info |
| Update timestamp | `touch_session(Uuid)` | Updates `updated_at` field |
| Delete session | `delete_session(Uuid)` | Removes session and messages (cascade) |
| Add message | `add_message(&StoredMessage)` | Appends message to session |
| Get messages | `get_messages(Uuid)` | Returns all messages ordered by timestamp |
| Cleanup | `cleanup(&SessionConfig)` | Removes old/excess sessions |

### Error Handling

All operations return `Result<T, StorageError>`:
- `StorageError::Database` for SQLite errors
- `StorageError::NotFound` for missing sessions
- `StorageError::Migration` for schema migration failures
- `StorageError::InvalidData` for parsing errors

---

## Data Flows

### Flow 1: New Conversation with Persistence

```
1. User: synapse "What is Rust?"
         |
2. CLI: create_storage(None) -> Box<dyn SessionStore>
         |
3. CLI: SessionConfig::auto_cleanup check
         |
4. CLI: storage.cleanup(&config) (if enabled)
         |
5. CLI: Session::new(provider, model) -> session
         |
6. CLI: storage.create_session(&session)
         |
7. CLI: StoredMessage::new(user_message) -> msg
         |
8. CLI: storage.add_message(&msg)
         |
9. CLI: provider.stream(&messages)
         |
10. CLI: [tokens stream to terminal]
         |
11. CLI: StoredMessage::new(assistant_response) -> response_msg
         |
12. CLI: storage.add_message(&response_msg)
         |
13. User sees response, session saved for continuation
```

### Flow 2: Continue Existing Session

```
1. User: synapse --session abc-123 "Tell me more"
         |
2. CLI: create_storage(None) -> Box<dyn SessionStore>
         |
3. CLI: storage.get_session(abc-123)
         |
4. CLI: storage.get_messages(abc-123) -> history
         |
5. CLI: Convert history to Message[] for provider
         |
6. CLI: Add new user message
         |
7. CLI: provider.stream(&full_history)
         |
8. CLI: storage.add_message(&user_msg)
         |
9. CLI: [tokens stream to terminal]
         |
10. CLI: storage.add_message(&assistant_msg)
         |
11. CLI: storage.touch_session(abc-123)
         |
12. User sees response with full context maintained
```

### Flow 3: Session Cleanup

```
1. CLI startup with auto_cleanup: true
         |
2. CLI: storage.cleanup(&session_config)
         |
3. SqliteStore: Calculate retention cutoff (now - retention_days)
         |
4. SqliteStore: DELETE sessions WHERE updated_at < cutoff
         |
5. SqliteStore: Count remaining sessions
         |
6. SqliteStore: If count > max_sessions, delete oldest
         |
7. SqliteStore: Return CleanupResult { sessions_deleted, by_max_limit, by_retention }
         |
8. CLI: Continue with normal operation
```

### Flow 4: Session Management Commands

```
# List sessions
1. User: synapse sessions list
2. CLI: storage.list_sessions()
3. CLI: Display table with ID, provider, model, count, preview
4. Sessions ordered by updated_at DESC

# Show session
1. User: synapse sessions show abc-123
2. CLI: storage.get_session(abc-123)
3. CLI: storage.get_messages(abc-123)
4. CLI: Display session metadata + all messages

# Delete session
1. User: synapse sessions delete abc-123
2. CLI: storage.delete_session(abc-123)
3. CLI: Display confirmation (cascade deletes messages)
```

---

## Non-Functional Requirements

### Performance

| Operation | Target | Implementation |
|-----------|--------|----------------|
| Session load | < 50ms | SQLite with indexes on session_id |
| Message insert | < 10ms | Single row INSERT |
| List sessions | < 100ms | Index on updated_at, LIMIT if needed |
| Cleanup | < 500ms | Bulk DELETE with transactions |
| Startup overhead | < 100ms | Migration runs only on first use |

### Reliability

| Requirement | Implementation |
|-------------|----------------|
| Data integrity | WAL mode, FOREIGN KEY constraints |
| Cascade delete | ON DELETE CASCADE for messages |
| Migration safety | sqlx versioned migrations |
| Error recovery | Transaction rollback on failure |

### Security

| Requirement | Implementation |
|-------------|----------------|
| Local storage | Database at `~/.config/synapse/sessions.db` |
| No encryption | Messages stored in plaintext (user responsibility) |
| Single-user | No authentication (local CLI tool) |
| Input validation | UUID parsing validates session IDs |

### Scalability

| Requirement | Implementation |
|-------------|----------------|
| Session limits | max_sessions config (default: 100) |
| Retention | retention_days config (default: 90) |
| Auto-cleanup | Runs on startup when enabled |
| Connection pooling | max 5 connections via sqlx |

---

## File Structure

| File | Purpose | Status |
|------|---------|--------|
| `synapse-core/src/session.rs` | Session, SessionSummary, StoredMessage types | Complete |
| `synapse-core/src/storage.rs` | SessionStore trait, StorageError, CleanupResult | Complete |
| `synapse-core/src/storage/sqlite.rs` | SqliteStore implementation, create_storage() | Complete |
| `synapse-core/src/config.rs` | SessionConfig with cleanup settings | Complete |
| `synapse-core/src/lib.rs` | Public exports for session/storage types | Complete |
| `synapse-core/migrations/20250125_001_initial.sql` | Database schema | Complete |
| `synapse-cli/src/main.rs` | CLI with session management commands | Complete |
| `config.example.toml` | [session] configuration section | Complete |

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

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **R-1: Database corruption** | Low | High | WAL mode enabled, sqlx handles transactions |
| **R-2: Large session history** | Medium | Medium | Future: pagination or summarization |
| **R-3: Migration conflicts** | Low | Medium | sqlx versioned migrations |
| **R-4: Storage path access** | Low | Low | Configurable via DATABASE_URL env var |
| **R-5: UUID collision** | Very Low | High | UUID v4 has negligible collision probability |

---

## Design Decisions

### 1. SQLite as Default Storage

**Decision:** Use SQLite for local storage with option to configure other databases.

**Rationale:** Simple deployment (single file), no external dependencies, sufficient for personal use workloads. The trait-based design allows future PostgreSQL/MySQL support.

**Trade-off:** Single-user only, no remote access. Acceptable for CLI tool.

### 2. UUID v4 for Session IDs

**Decision:** Use UUID v4 instead of v8 (RFC 9562).

**Rationale:** v4 is widely supported, simpler to implement. v8's sortability benefit is minimal for this use case since we sort by updated_at anyway.

### 3. WAL Mode for SQLite

**Decision:** Enable WAL journal mode.

**Rationale:** Better concurrent read/write performance, safer against corruption during crashes.

### 4. Auto-Cleanup on Startup

**Decision:** Run cleanup at CLI startup when enabled.

**Rationale:** Simple implementation, ensures limits are enforced. Alternative (background job) adds complexity without significant benefit for CLI tool.

### 5. Messages Stored with Session

**Decision:** Store messages in separate table with foreign key to sessions.

**Rationale:** Follows normalized database design. CASCADE delete ensures no orphan messages. Allows efficient retrieval of messages for a session.

### 6. Plaintext Storage

**Decision:** Store messages in plaintext without encryption.

**Rationale:** Encryption at rest adds complexity and key management burden. Users can encrypt their home directory if needed. Clear documentation of this limitation.

---

## Test Coverage

### Unit Tests (synapse-core)

**session.rs** (10 tests):
- Session creation and builder methods
- SessionSummary construction
- StoredMessage creation
- Clone implementations

**storage.rs** (4 tests):
- StorageError display
- CleanupResult default and construction
- Clone implementations

**storage/sqlite.rs** (20 tests):
- Role serialization/parsing
- CRUD operations for sessions
- CRUD operations for messages
- CASCADE delete verification
- Cleanup by max_sessions
- Cleanup by retention_days
- CleanupResult counts

**config.rs** (4 tests for SessionConfig):
- Default values
- Partial deserialization
- Full deserialization

### CLI Tests (synapse-cli)

**main.rs** (4 tests):
- Argument parsing
- Session flag handling
- Truncate function

---

## Remaining Work

Based on PRD and implementation analysis:

1. **Update `docs/tasklist.md`**: Mark all Phase 8 tasks as complete
2. **Update `CHANGELOG.md`**: Add SY-9 session storage entry
3. **Integration Testing**: Verify end-to-end flow manually
4. **Optional Enhancements** (future tickets):
   - Print session ID after first exchange for UX
   - Implement `session.database_url` config option
   - Consider pagination for very long sessions

---

## Implementation Status

| Task | Description | Status |
|------|-------------|--------|
| 8.1 | Session struct | Complete |
| 8.2 | SessionStore trait | Complete |
| 8.3 | SQLite storage | Complete |
| 8.4 | Schema migrations | Complete |
| 8.5 | CLI integration | Complete |
| 8.6 | Session limits | Complete |
| 8.7 | Auto-cleanup | Complete |

All core implementation tasks are complete. Remaining work is documentation updates.

---

## References

- `docs/prd/SY-9.prd.md` - Requirements
- `docs/research/SY-9.md` - Technical research
- `docs/vision.md` - Architecture patterns, database schema
- `docs/conventions.md` - Code standards
- `config.example.toml` - Session configuration options
- [sqlx documentation](https://docs.rs/sqlx/)
- [uuid crate documentation](https://docs.rs/uuid/)
