# Implementation Plan: SY-9 - Session Storage

Status: PLAN_APPROVED

## Overview

This plan outlines the implementation of Phase 8: Session Storage for Synapse. The goal is to persist conversations to SQLite, enabling multi-turn dialogues that survive application restarts.

---

## 1. Components and Modules

### 1.1 New Modules in synapse-core

| Module | File | Responsibility |
|--------|------|----------------|
| `session` | `synapse-core/src/session.rs` | `Session`, `SessionSummary`, `StoredMessage` structs |
| `storage` | `synapse-core/src/storage.rs` | `SessionStore` trait (port), `StorageError`, module declarations |
| `storage/sqlite` | `synapse-core/src/storage/sqlite.rs` | `SqliteStore` implementation (adapter) |

### 1.2 Modified Modules

| Module | Changes |
|--------|---------|
| `config.rs` | Add `SessionConfig` struct, add `session` field to `Config` |
| `message.rs` | Add `Serialize`/`Deserialize` derives to `Role` |
| `lib.rs` | Export new session and storage types |

### 1.3 New Files

| File | Purpose |
|------|---------|
| `synapse-core/migrations/20250125_001_initial.sql` | Database schema (sessions + messages tables) |
| `.sqlx/` directory | Compile-time query metadata for offline mode |

### 1.4 CLI Changes

| File | Changes |
|------|---------|
| `synapse-cli/src/main.rs` | Add `--session` flag, session subcommands, wire storage |

---

## 2. API Contract

### 2.1 Session Types (`synapse-core/src/session.rs`)

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A conversation session containing metadata.
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

/// Summary information for listing sessions.
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

/// A message stored in the database with full metadata.
#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}
```

### 2.2 Configuration Types (`synapse-core/src/config.rs`)

```rust
/// Session storage configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_max_sessions")]
    pub max_sessions: u32,           // default: 100
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,         // default: 90
    #[serde(default = "default_auto_cleanup")]
    pub auto_cleanup: bool,          // default: true
}

fn default_max_sessions() -> u32 { 100 }
fn default_retention_days() -> u32 { 90 }
fn default_auto_cleanup() -> bool { true }
```

### 2.3 Storage Trait (`synapse-core/src/storage.rs`)

```rust
use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

use crate::config::SessionConfig;
use crate::session::{Session, SessionSummary, StoredMessage};

/// Errors that can occur during storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("session not found: {0}")]
    NotFound(Uuid),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("invalid data: {0}")]
    InvalidData(String),
}

/// Result of a cleanup operation.
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub sessions_deleted: u32,
    pub by_max_limit: u32,
    pub by_retention: u32,
}

/// Port for session storage implementations.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Create a new session.
    async fn create_session(&self, session: &Session) -> Result<(), StorageError>;

    /// Get a session by ID.
    async fn get_session(&self, id: Uuid) -> Result<Option<Session>, StorageError>;

    /// List all sessions with summaries (most recent first).
    async fn list_sessions(&self) -> Result<Vec<SessionSummary>, StorageError>;

    /// Update session's updated_at timestamp.
    async fn touch_session(&self, id: Uuid) -> Result<(), StorageError>;

    /// Delete a session and all its messages.
    async fn delete_session(&self, id: Uuid) -> Result<bool, StorageError>;

    /// Add a message to a session.
    async fn add_message(&self, message: &StoredMessage) -> Result<(), StorageError>;

    /// Get all messages for a session (ordered by timestamp).
    async fn get_messages(&self, session_id: Uuid) -> Result<Vec<StoredMessage>, StorageError>;

    /// Run cleanup based on configuration.
    async fn cleanup(&self, config: &SessionConfig) -> Result<CleanupResult, StorageError>;
}
```

### 2.4 Factory Function

```rust
/// Create a storage backend from database URL.
///
/// Defaults to `sqlite:~/.config/synapse/sessions.db` if no URL provided.
pub async fn create_storage(database_url: Option<&str>) -> Result<Box<dyn SessionStore>, StorageError>;
```

### 2.5 CLI Arguments

```rust
/// Synapse CLI arguments
#[derive(Parser)]
struct Args {
    /// Message to send (reads from stdin if not provided)
    message: Option<String>,

    /// Continue an existing session by ID
    #[arg(short, long)]
    session: Option<Uuid>,

    /// Subcommands for session management
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Session management commands
    Sessions {
        #[command(subcommand)]
        action: SessionAction,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List,
    /// Show messages in a session
    Show { id: Uuid },
    /// Delete a session
    Delete { id: Uuid },
}
```

---

## 3. Data Flows

### 3.1 New Session Flow (Default)

```
User runs: synapse "Hello"
     |
     v
[CLI] Load config
     |
     v
[CLI] Create storage (SqliteStore)
     |
     v
[CLI] If auto_cleanup enabled, run cleanup
     |
     v
[CLI] Create new Session (UUID v4, provider, model from config)
     |
     v
[Storage] INSERT session into database
     |
     v
[CLI] Create StoredMessage for user message
     |
     v
[Storage] INSERT message into database
     |
     v
[CLI] Call provider.stream() with [user_message]
     |
     v
[CLI] Collect response, create StoredMessage for assistant
     |
     v
[Storage] INSERT assistant message, UPDATE session.updated_at
     |
     v
[CLI] Display response to user
```

### 3.2 Continue Session Flow

```
User runs: synapse --session <uuid> "Follow up"
     |
     v
[CLI] Load config, create storage
     |
     v
[Storage] SELECT session by ID (error if not found)
     |
     v
[Storage] SELECT all messages for session
     |
     v
[CLI] Append new user message
     |
     v
[Storage] INSERT user message
     |
     v
[CLI] Call provider.stream() with full conversation history
     |
     v
[CLI] Collect response, store assistant message
     |
     v
[Storage] UPDATE session.updated_at
```

### 3.3 Cleanup Flow

```
[CLI] Startup with auto_cleanup=true
     |
     v
[Storage] SELECT COUNT(*) from sessions
     |
     +--> If count > max_sessions:
     |        DELETE oldest sessions until count = max_sessions
     |
     v
[Storage] DELETE sessions WHERE updated_at < (now - retention_days)
     |
     v
[Storage] Return CleanupResult
```

---

## 4. Database Schema

### 4.1 Migration: `20250125_001_initial.sql`

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

### 4.2 Data Types

- **IDs**: UUID stored as TEXT (SQLite has no native UUID type)
- **Timestamps**: ISO 8601 format (RFC 3339) stored as TEXT
- **Role**: Enum stored as TEXT ("system", "user", "assistant")

---

## 5. Non-Functional Requirements

### 5.1 Performance

| Operation | Target | Implementation |
|-----------|--------|----------------|
| Session load | < 50ms | Index on session_id, single query |
| Message retrieval | < 50ms | Index on (session_id, timestamp) |
| Cleanup | < 1s for 1000 sessions | Batch DELETE with LIMIT |
| Database open | < 100ms | WAL mode, connection pooling |

### 5.2 Reliability

- **WAL mode**: Enable SQLite WAL for crash recovery
- **Foreign keys**: CASCADE DELETE for referential integrity
- **Transaction safety**: Use transactions for multi-statement operations
- **Idempotent migrations**: Use IF NOT EXISTS clauses

### 5.3 Security

- **File permissions**: Create database with 0600 permissions
- **No secrets in DB**: Only store conversation content, not API keys
- **Input validation**: Validate UUIDs before queries

### 5.4 Maintainability

- **Compile-time queries**: Use `sqlx::query!` for type safety
- **Offline mode**: Commit `.sqlx/` directory for CI builds
- **Trait abstraction**: Easy to add PostgreSQL/MySQL later

---

## 6. Risks and Mitigations

### R1: Compile-Time Query Complexity

**Risk**: `sqlx::query!` requires DATABASE_URL at compile time.
**Likelihood**: High
**Impact**: Medium (blocks CI builds)
**Mitigation**:
1. Run `cargo sqlx prepare` before commits
2. Commit `.sqlx/` directory with query metadata
3. Set `SQLX_OFFLINE=true` in CI

### R2: DateTime Parsing

**Risk**: Inconsistent datetime formats cause parsing failures.
**Likelihood**: Medium
**Impact**: Medium
**Mitigation**:
1. Always use `Utc::now().to_rfc3339()` for storage
2. Parse with `DateTime::parse_from_rfc3339()`
3. Add unit tests for round-trip serialization

### R3: Large Session Performance

**Risk**: Sessions with 1000+ messages may be slow to load.
**Likelihood**: Low (unusual use case)
**Impact**: Low
**Mitigation**:
1. Document limitation in help text
2. Consider pagination in future phase (not SY-9)

### R4: Database File Permissions

**Risk**: Database created with world-readable permissions.
**Likelihood**: Medium
**Impact**: Medium (privacy concern)
**Mitigation**:
1. Create parent directory with 0700
2. Create database file with 0600
3. Check and warn on startup if permissions too open

### R5: Migration Failures

**Risk**: Schema changes break existing databases.
**Likelihood**: Low (first migration)
**Impact**: High
**Mitigation**:
1. Use IF NOT EXISTS for all CREATE statements
2. Test migration on fresh and existing databases
3. Document manual recovery steps

---

## 7. Implementation Tasks

### Phase 1: Foundation (Tasks 8.1-8.2)

1. **Task 8.1**: Create `synapse-core/src/session.rs`
   - `Session` struct with all fields
   - `SessionSummary` struct for listing
   - `StoredMessage` struct for database messages
   - Unit tests for struct construction

2. **Task 8.2**: Create `synapse-core/src/storage.rs`
   - `SessionStore` trait definition
   - `StorageError` enum with thiserror
   - `CleanupResult` struct
   - Module declaration for sqlite submodule

### Phase 2: Database (Tasks 8.3-8.4)

3. **Task 8.3**: Add dependencies and create SqliteStore
   - Add `sqlx`, `uuid`, `chrono` to Cargo.toml
   - Create `synapse-core/src/storage/sqlite.rs`
   - Implement `SqliteStore::new()` with connection pool
   - Implement `create_storage()` factory

4. **Task 8.4**: Schema migrations
   - Create `synapse-core/migrations/` directory
   - Write `20250125_001_initial.sql`
   - Test migration runs correctly
   - Generate `.sqlx/` metadata

### Phase 3: Storage Implementation (Tasks 8.5-8.6)

5. **Task 8.5**: Implement SessionStore methods
   - `create_session`, `get_session`, `list_sessions`
   - `delete_session`, `touch_session`
   - `add_message`, `get_messages`
   - Integration tests with temp database

6. **Task 8.6**: Implement cleanup
   - `cleanup()` method with max_sessions logic
   - retention_days deletion logic
   - Unit tests for cleanup scenarios

### Phase 4: CLI Integration (Task 8.7)

7. **Task 8.7**: Wire storage into CLI
   - Add `SessionConfig` to `Config`
   - Add `--session` flag to Args
   - Add `sessions` subcommand with list/show/delete
   - Save messages after each exchange
   - Run auto-cleanup on startup
   - Update exports in `lib.rs`

---

## 8. Dependencies

### New Dependencies for synapse-core/Cargo.toml

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

### New Dependencies for synapse-cli/Cargo.toml

```toml
uuid = { version = "1", features = ["v4"] }
```

### Development Tool

```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

---

## 9. Testing Strategy

### Unit Tests

- `session.rs`: Session/SessionSummary construction
- `config.rs`: SessionConfig defaults and parsing
- `message.rs`: Role serialization/deserialization

### Integration Tests

- `SqliteStore`: All CRUD operations with temp database
- Cleanup: Max sessions and retention scenarios
- Migration: Fresh database and idempotent re-run

### Manual Testing

```bash
# Create sessions
synapse "Hello"
synapse "Another topic"

# List sessions
synapse sessions list

# Show session
synapse sessions show <id>

# Continue session
synapse --session <id> "Follow up"

# Delete session
synapse sessions delete <id>
```

---

## 10. Open Questions

None - all questions resolved during research phase:

1. **SQL Mode**: Compile-time checked queries (resolved)
2. **Session ID Format**: UUID v4 (resolved)
3. **Default Behavior**: New session (resolved)

---

## 11. Acceptance Criteria

From PRD:

- [ ] `synapse sessions list` shows previous conversations
- [ ] Sessions persist across application restarts
- [ ] Messages maintain correct ordering within sessions
- [ ] Provider and model information stored with each session
- [ ] Cleanup respects configured limits
- [ ] Session load time < 50ms
- [ ] Database operations do not block streaming responses
