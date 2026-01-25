# Research: SY-9 - Phase 8: Session Storage

## Resolved Questions

User-provided answers to implementation questions:

1. **SQLite Mode**: Compile-time checked queries (requires sqlx-cli for migrations)
   - This means using `sqlx::query!` and `sqlx::query_as!` macros
   - Requires running `sqlx database create` and `sqlx migrate run` during development
   - DATABASE_URL environment variable needed for compile-time checks

2. **Session ID Format**: UUID v8 (modern replacement for v4, sortable, universally unique)
   - Add `uuid` crate with `v8` and `serde` features
   - Session IDs generated with `Uuid::new_v8()`

3. **Default Session Behavior**: New session (always create new, require explicit --session to continue)
   - Each CLI invocation creates a fresh session
   - User must pass `--session <id>` flag to continue an existing session
   - Simplifies default behavior, explicit is better than implicit

---

## Related Modules and Services

### synapse-core Structure

| File | Purpose | Relevance to SY-9 |
|------|---------|-------------------|
| `synapse-core/src/lib.rs` | Module exports | Must export `Session`, `SessionStore`, and storage types |
| `synapse-core/src/config.rs` | Configuration types | Must add `SessionConfig` struct for session settings |
| `synapse-core/src/message.rs` | `Role` and `Message` types | Session stores messages, may need serde derives |
| `synapse-core/src/provider.rs` | `LlmProvider` trait | No changes, but provider/model stored per session |
| `synapse-core/Cargo.toml` | Dependencies | Add `sqlx`, `uuid`, `chrono` |

### New Files to Create

| File | Purpose |
|------|---------|
| `synapse-core/src/session.rs` | `Session` struct definition |
| `synapse-core/src/storage.rs` | `SessionStore` trait (port) |
| `synapse-core/src/storage/sqlite.rs` | `SqliteStore` implementation (adapter) |
| `synapse-core/migrations/` | SQLx migrations directory |

### synapse-cli Structure

| File | Purpose | Relevance to SY-9 |
|------|---------|-------------------|
| `synapse-cli/src/main.rs` | CLI entry point | Add session commands, wire storage, save messages |
| `synapse-cli/Cargo.toml` | Dependencies | May need `uuid` for parsing session IDs |

---

## Current Endpoints and Contracts

### Config Struct (Current)

Located in `synapse-core/src/config.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
}
```

**Extension needed**: Add `session` field with `SessionConfig` struct:

```rust
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub provider: String,
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default)]
    pub session: SessionConfig,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_max_sessions")]
    pub max_sessions: u32,           // default: 100
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,         // default: 90
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup: bool,          // default: true
}
```

### Message Struct (Current)

Located in `synapse-core/src/message.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}
```

**Extension needed**: Add Serialize/Deserialize derives for database storage and add `id`/`timestamp` for messages:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

// Extended message for storage (separate type or extend existing)
#[derive(Debug, Clone, PartialEq)]
pub struct StoredMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_results: Option<serde_json::Value>,
}
```

### Database Schema (from vision.md)

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    name TEXT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    system_prompt TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_calls TEXT,
    tool_results TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX idx_messages_session ON messages(session_id);
CREATE INDEX idx_sessions_updated ON sessions(updated_at);
```

### CLI Commands (from PRD)

Current:
```bash
synapse "message"           # One-shot mode
echo "message" | synapse    # Stdin mode
```

New:
```bash
# Session management
synapse sessions list              # List all sessions
synapse sessions show <id>         # Show messages in session
synapse sessions delete <id>       # Delete a session

# Continue existing session
synapse --session <id> "message"   # Continue specific session
synapse -s <id> "message"          # Short form
```

---

## Patterns Used

### Hexagonal Architecture (Ports and Adapters)

The project follows hexagonal architecture. For session storage:

**Port (Trait)**:
```rust
// synapse-core/src/storage.rs
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create_session(&self, session: &Session) -> Result<(), StorageError>;
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError>;
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError>;
    async fn delete_session(&self, id: Uuid) -> Result<bool, StorageError>;
    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError>;
    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError>;
    async fn cleanup(&self, config: &SessionConfig) -> Result<CleanupResult, StorageError>;
}
```

**Adapter (Implementation)**:
```rust
// synapse-core/src/storage/sqlite.rs
pub struct SqliteStore {
    pool: sqlx::SqlitePool,
}
```

### Error Handling Pattern

Using `thiserror` in library, `anyhow` in CLI:

```rust
// synapse-core/src/storage.rs
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("session not found: {0}")]
    NotFound(Uuid),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}
```

### Factory Pattern

Similar to `create_provider()` in `provider/factory.rs`:

```rust
// synapse-core/src/storage.rs
pub async fn create_storage(database_url: &str) -> Result<Box<dyn SessionStore>, StorageError> {
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(Box::new(SqliteStore::new(pool)))
}
```

### Compile-Time Checked Queries (sqlx)

Since user selected compile-time checked queries:

```rust
// Requires: DATABASE_URL environment variable during compilation
// And: sqlx prepare --database-url sqlite:./sessions.db

let session = sqlx::query_as!(
    Session,
    r#"
    SELECT id, name, provider, model, system_prompt, created_at, updated_at
    FROM sessions WHERE id = ?
    "#,
    id.to_string()
)
.fetch_optional(&self.pool)
.await?;
```

### Session Struct Design

```rust
// synapse-core/src/session.rs
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub name: Option<String>,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: Uuid,
    pub name: Option<String>,
    pub provider: String,
    pub model: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: u32,
    pub preview: Option<String>,  // First user message truncated
}
```

---

## Dependencies

### New Dependencies for synapse-core/Cargo.toml

```toml
[dependencies]
# Existing
async-trait = "0.1"
dirs = "6.0.0"
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["rt", "macros"] }
toml = "0.9.8"
async-stream = "0.3"
eventsource-stream = "0.2"
futures = "0.3"

# New for session storage
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
uuid = { version = "1", features = ["v8", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### synapse-cli/Cargo.toml

```toml
[dependencies]
# Existing
anyhow = "1"
clap = { version = "4.5.54", features = ["derive"] }
synapse-core = { path = "../synapse-core" }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std", "signal"] }
futures = "0.3"

# New
uuid = { version = "1", features = ["v8"] }  # For parsing session IDs from CLI args
```

### SQLx CLI Tool

For compile-time checked queries, the sqlx-cli tool is required:

```bash
cargo install sqlx-cli --no-default-features --features sqlite

# During development:
export DATABASE_URL="sqlite:~/.config/synapse/sessions.db"
sqlx database create
sqlx migrate run

# Before commit (generates query metadata):
cargo sqlx prepare
```

---

## Database URL Resolution

Priority order:
1. `DATABASE_URL` environment variable (highest priority)
2. `session.database_url` in config.toml
3. Default: `sqlite:~/.config/synapse/sessions.db`

Implementation:
```rust
fn get_database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let config_dir = dirs::config_dir()
            .expect("Could not determine config directory")
            .join("synapse");
        format!("sqlite:{}/sessions.db", config_dir.display())
    })
}
```

---

## Limitations and Risks

### Limitations

1. **SQLite Only**: This phase implements SQLite only. PostgreSQL/MySQL are future enhancements.

2. **No Concurrent Writers**: SQLite handles concurrent readers well but has limitations with concurrent writes. Single-user CLI use case is fine.

3. **Compile-Time Queries**: Requires sqlx-cli installed and DATABASE_URL set during development. CI will need mock database or `cargo sqlx prepare` artifacts.

4. **No Message Pagination**: Initial implementation loads all messages for a session. Large sessions may be slow.

5. **No Full-Text Search**: Session search limited to ID matching. Content search is a future enhancement.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| R-1: Database file permissions | Medium | High | Create file with restricted permissions (0600) |
| R-2: Migration failures on schema changes | Medium | High | Use proper migration versioning, test rollbacks |
| R-3: Compile-time query errors in CI | High | Medium | Commit sqlx-data.json, use offline mode in CI |
| R-4: DateTime parsing issues | Medium | Medium | Store as ISO 8601 strings, use chrono consistently |
| R-5: WAL mode issues on some filesystems | Low | High | Test on NFS/network drives, provide fallback |
| R-6: Large session performance | Low | Medium | Add message count limit warning, defer pagination |

### R-3 Mitigation: CI with SQLx

For compile-time checked queries to work in CI without a database:

```yaml
# In CI workflow
env:
  SQLX_OFFLINE: true  # Use cached query metadata

# Before pushing:
cargo sqlx prepare --database-url sqlite:./test.db
# Commit the generated .sqlx/ directory
```

### R-4 Mitigation: DateTime Storage

SQLite stores dates as TEXT. Use ISO 8601 format:

```rust
// Store
let created_at = Utc::now().to_rfc3339();

// Retrieve
let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&row.created_at)
    .unwrap()
    .with_timezone(&Utc);
```

---

## New Technical Questions

Questions discovered during research that may need follow-up:

1. **Session Auto-Creation**: When running `synapse "message"` without `--session`, should we:
   - (a) Always create a new session (current choice per user preference)
   - (b) Resume the most recent session if it's less than X minutes old

   **Resolved**: User chose (a) - always create new session.

2. **Message Storage Timing**: Should we save messages:
   - (a) Before sending to LLM (user message) + After receiving (assistant)
   - (b) Only after successful completion of the exchange

   **Recommendation**: Option (a) - allows partial recovery if streaming is interrupted.

3. **Session Name Generation**: Should sessions have auto-generated names?
   - (a) Use first user message as preview/name
   - (b) Generate with timestamp
   - (c) Leave null, user can name later

   **Recommendation**: Option (c) for simplicity, with preview from first message for display.

4. **Cleanup Timing**: When should auto-cleanup run?
   - (a) On startup only
   - (b) After each message
   - (c) Periodic background task

   **Recommendation**: Option (a) for simplicity. Background tasks add complexity.

5. **Database Initialization**: When to create the database?
   - (a) On first run (lazily)
   - (b) Explicit init command

   **Recommendation**: Option (a) - create on first access.

---

## Implementation Sequence

Recommended order per phase-8.md tasks:

1. **Task 8.1**: Create `session.rs` with `Session` and `SessionSummary` structs
2. **Task 8.2**: Create `storage.rs` with `SessionStore` trait and `StorageError`
3. **Task 8.3**: Add sqlx/uuid/chrono deps, create `storage/sqlite.rs` with `SqliteStore`
4. **Task 8.4**: Create migrations in `migrations/` directory
5. **Task 8.5**: Wire into CLI - add session commands, save messages
6. **Task 8.6**: Implement cleanup in `SqliteStore::cleanup()`
7. **Task 8.7**: Run cleanup on startup via `SqliteStore::new()` or explicit call

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `synapse-core/Cargo.toml` | Modify | Add `sqlx`, `uuid`, `chrono` dependencies |
| `synapse-core/src/session.rs` | Create | `Session`, `SessionSummary`, `StoredMessage` structs |
| `synapse-core/src/storage.rs` | Create | `SessionStore` trait, `StorageError`, `mod sqlite;` |
| `synapse-core/src/storage/sqlite.rs` | Create | `SqliteStore` implementation |
| `synapse-core/src/config.rs` | Modify | Add `SessionConfig` struct, update `Config` |
| `synapse-core/src/message.rs` | Modify | Add `Serialize`, `Deserialize` derives to `Role` |
| `synapse-core/src/lib.rs` | Modify | Export session and storage types |
| `synapse-core/migrations/001_initial.sql` | Create | Sessions and messages tables |
| `synapse-cli/Cargo.toml` | Modify | Add `uuid` dependency |
| `synapse-cli/src/main.rs` | Modify | Add session subcommands, wire storage |
| `config.example.toml` | Modify | Already has `[session]` section - no changes needed |

---

## Test Plan

### Unit Tests

1. **Session struct**:
   - `test_session_new` - Create session with defaults
   - `test_session_summary_from_session` - Convert session to summary

2. **SessionConfig defaults**:
   - `test_session_config_defaults` - Verify default values
   - `test_session_config_parse` - Parse from TOML

3. **Role serialization**:
   - `test_role_serialize` - Role to string
   - `test_role_deserialize` - String to Role

### Integration Tests

1. **SqliteStore operations**:
   - `test_sqlite_create_session` - Create and retrieve session
   - `test_sqlite_list_sessions` - List multiple sessions
   - `test_sqlite_add_get_messages` - Add and retrieve messages
   - `test_sqlite_delete_session` - Delete cascades to messages
   - `test_sqlite_cleanup_max_sessions` - Oldest deleted when exceeded
   - `test_sqlite_cleanup_retention` - Old sessions purged

### CLI Tests

```bash
# Create session implicitly
synapse "Hello"

# List sessions
synapse sessions list

# Show session
synapse sessions show <id>

# Continue session
synapse --session <id> "Follow up question"

# Delete session
synapse sessions delete <id>
```

---

## References

- `docs/prd/SY-9.prd.md` - PRD document
- `docs/phase/phase-8.md` - Phase task breakdown
- `docs/vision.md` - Database schema and architecture
- `config.example.toml` - Session configuration example
- [sqlx documentation](https://docs.rs/sqlx/)
- [uuid crate](https://docs.rs/uuid/)
- [chrono crate](https://docs.rs/chrono/)
