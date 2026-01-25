# SY-9 Summary: Phase 8 - Session Storage

**Status:** COMPLETE
**Date:** 2026-01-25

---

## Overview

SY-9 implements persistent conversation storage using SQLite, enabling users to maintain context across CLI sessions and manage their conversation history. Users can now continue previous conversations by session ID, list all sessions, view message history, and delete sessions they no longer need.

The key change is that running `synapse "Hello"` now persists the conversation to a local SQLite database. Users can later run `synapse --session <uuid> "Follow up"` to continue that conversation with full context preserved.

---

## What Was Built

### New Components

1. **Session Types** (`synapse-core/src/session.rs`)
   - `Session` struct with id, name, provider, model, system_prompt, created_at, updated_at
   - `SessionSummary` struct for list view with message_count and preview
   - `StoredMessage` struct with id, session_id, role, content, timestamp
   - Builder pattern: `Session::new().with_name().with_system_prompt()`
   - UUID v7 (time-sortable) for session and message identifiers

2. **SessionStore Trait** (`synapse-core/src/storage.rs`)
   - Port definition following hexagonal architecture
   - CRUD operations: create_session, get_session, list_sessions, delete_session
   - Message operations: add_message, get_messages
   - Maintenance: touch_session, cleanup
   - `StorageError` enum: Database, NotFound, Migration, InvalidData
   - `CleanupResult` struct tracking deletion counts

3. **SqliteStore Implementation** (`synapse-core/src/storage/sqlite.rs`)
   - Connection pooling (max 5 connections)
   - WAL mode for better concurrent access
   - Auto-migration on startup via sqlx
   - Role serialization/parsing (system, user, assistant)
   - Cascade delete for messages when session deleted
   - `create_storage()` factory with URL resolution priority

4. **Database Schema** (`synapse-core/migrations/20250125_001_initial.sql`)
   - `sessions` table: id, name, provider, model, system_prompt, timestamps
   - `messages` table: id, session_id, role, content, timestamp
   - Foreign key with ON DELETE CASCADE
   - Indexes: idx_messages_session, idx_messages_timestamp, idx_sessions_updated

5. **SessionConfig** (`synapse-core/src/config.rs`)
   - `database_url: Option<String>` - custom database path
   - `max_sessions: u32` - default 100
   - `retention_days: u32` - default 90
   - `auto_cleanup: bool` - default true

### Modified Components

1. **CLI** (`synapse-cli/src/main.rs`)
   - `--session <uuid>` / `-s <uuid>` flag to continue existing session
   - `sessions list` subcommand - shows all sessions with metadata
   - `sessions show <uuid>` subcommand - displays full message history
   - `sessions delete <uuid>` subcommand - removes session and messages
   - Auto-cleanup on startup when `auto_cleanup: true`
   - Passes config `database_url` to storage factory

2. **Configuration** (`config.example.toml`)
   - Added `[session]` section with all session storage options
   - Documented DATABASE_URL environment variable priority

3. **Dependencies** (`synapse-core/Cargo.toml`)
   - Added: sqlx (runtime-tokio, sqlite), uuid (v4, v7, serde), chrono (serde), async-trait, dirs

---

## Key Decisions

### 1. SQLite as Default Storage
**Decision:** Use SQLite for local storage with configurable database URL.

**Rationale:** Simple deployment (single file), no external dependencies, sufficient for personal use workloads. The trait-based design allows future PostgreSQL/MySQL support without code changes.

### 2. UUID v7 for Identifiers
**Decision:** Use UUID v7 instead of v4.

**Rationale:** UUID v7 is time-sortable, meaning newer sessions naturally sort after older ones. This improves performance for time-based queries and makes debugging easier (IDs indicate creation order).

### 3. Database URL Priority
**Decision:** Resolution order is DATABASE_URL env var > config.toml > default path.

**Rationale:** Environment variables allow deployment-time configuration without file changes. Config file provides user customization. Default path (`~/.config/synapse/sessions.db`) requires no setup.

### 4. WAL Mode
**Decision:** Enable Write-Ahead Logging for SQLite.

**Rationale:** Better concurrent read/write performance, safer against corruption during crashes. WAL allows readers to proceed without blocking writers.

### 5. Auto-Cleanup on Startup
**Decision:** Run cleanup at CLI startup when `auto_cleanup: true`.

**Rationale:** Simple implementation, ensures limits are enforced. Alternative (background job) adds complexity without significant benefit for a CLI tool.

### 6. Cascade Delete
**Decision:** Messages are automatically deleted when their session is deleted.

**Rationale:** Follows normalized database design. Prevents orphan messages and simplifies the delete operation to a single session delete.

### 7. Plaintext Storage
**Decision:** Store messages in plaintext without encryption.

**Rationale:** Encryption at rest adds complexity and key management burden. Users can encrypt their home directory if needed. This is documented behavior for a local CLI tool.

---

## Database Schema

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

-- Messages table with foreign key
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

---

## Data Flows

### New Conversation
```
User: synapse "What is Rust?"
  |
CLI: create_storage(config.database_url)
  |
CLI: storage.cleanup(&config) (if auto_cleanup)
  |
CLI: Session::new(provider, model)
  |
CLI: storage.create_session(&session)
  |
CLI: StoredMessage::new(user_message)
  |
CLI: storage.add_message(&user_msg)
  |
CLI: provider.stream(&messages)
  |
CLI: [tokens stream to terminal]
  |
CLI: storage.add_message(&assistant_msg)
  |
User: Session saved, can be continued later
```

### Continue Session
```
User: synapse --session abc-123 "Tell me more"
  |
CLI: storage.get_session(abc-123)
  |
CLI: storage.get_messages(abc-123) -> history
  |
CLI: Convert history to Message[] for provider
  |
CLI: provider.stream(&full_history)
  |
CLI: storage.add_message(&user_msg)
CLI: storage.add_message(&assistant_msg)
  |
CLI: storage.touch_session(abc-123)
  |
User: Full context maintained
```

---

## Testing

### Unit Tests for Session Types (10 tests)
- `test_session_new` - Session creation with UUID v7
- `test_session_with_name` - Builder pattern for name
- `test_session_with_system_prompt` - Builder pattern for system prompt
- `test_session_field_access` - All fields accessible
- `test_session_summary_construction` - Summary with message count/preview
- `test_stored_message_new` - Message creation
- `test_stored_message_field_access` - All fields accessible
- `test_session_clone` - Clone semantics
- `test_stored_message_clone` - Clone semantics

### Unit Tests for Storage Trait (4 tests)
- `test_storage_error_display` - Error message formatting
- `test_cleanup_result_default` - Default values
- `test_cleanup_result_construction` - Manual construction
- `test_cleanup_result_clone` - Clone semantics

### Unit Tests for SqliteStore (17 tests)
- `test_role_to_string` - Role serialization
- `test_parse_role` - Role parsing
- `test_create_and_get_session` - CRUD create/read
- `test_get_session_not_found` - Returns None
- `test_list_sessions` - Multiple sessions listed
- `test_touch_session` - Timestamp update
- `test_touch_session_not_found` - NotFound error
- `test_delete_session` - Successful deletion
- `test_delete_session_not_found` - Returns false
- `test_add_and_get_messages` - Message CRUD
- `test_messages_ordered_by_timestamp` - Correct ordering
- `test_list_sessions_with_message_count_and_preview` - Summary data
- `test_preview_truncated_to_50_chars` - Truncation logic
- `test_cascade_delete` - Messages deleted with session
- `test_cleanup_by_max_sessions` - Limit enforcement
- `test_cleanup_no_action_when_under_limit` - No false positives
- `test_cleanup_result_counts` - Accurate counts

### Unit Tests for SessionConfig (6 tests)
- Default values verification
- Partial deserialization
- Full deserialization with database_url

**Total new tests: 37**

---

## Usage

### Start New Conversation
```bash
# Creates new session, persists messages
synapse "What is the capital of France?"
# Output: Paris is the capital of France...
```

### Continue Existing Session
```bash
# First, find your session ID
synapse sessions list
# ID                                   PROVIDER  MODEL         MSGS  PREVIEW
# 01234567-89ab-cdef-0123-456789abcdef deepseek  deepseek-chat 2     What is the capital...

# Continue that session
synapse --session 01234567-89ab-cdef-0123-456789abcdef "What about its population?"
# Output: Paris has approximately 2.2 million people...
```

### Manage Sessions
```bash
# List all sessions
synapse sessions list

# View full conversation
synapse sessions show 01234567-89ab-cdef-0123-456789abcdef

# Delete a session
synapse sessions delete 01234567-89ab-cdef-0123-456789abcdef
```

### Configuration
```toml
# config.toml
[session]
# Database URL (optional, DATABASE_URL env var takes priority)
# database_url = "sqlite:~/.config/synapse/sessions.db"

# Cleanup settings
max_sessions = 100
retention_days = 90
auto_cleanup = true
```

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-core/src/session.rs` | New - Session, SessionSummary, StoredMessage types |
| `synapse-core/src/storage.rs` | New - SessionStore trait, StorageError, CleanupResult |
| `synapse-core/src/storage/sqlite.rs` | New - SqliteStore implementation |
| `synapse-core/migrations/20250125_001_initial.sql` | New - Database schema |
| `synapse-core/src/config.rs` | Modified - Added SessionConfig |
| `synapse-core/src/lib.rs` | Modified - Export session/storage types |
| `synapse-core/Cargo.toml` | Modified - Added dependencies |
| `synapse-cli/src/main.rs` | Modified - Session commands, persistence |
| `config.example.toml` | Modified - Added [session] section |

---

## Module Structure

```
synapse-core/src/
  lib.rs                # pub use session::*, storage::*
  session.rs            # Session, SessionSummary, StoredMessage
  config.rs             # SessionConfig
  storage.rs            # mod sqlite; SessionStore trait, StorageError
  storage/
    sqlite.rs           # SqliteStore, create_storage()
  migrations/
    20250125_001_initial.sql

synapse-cli/src/
  main.rs               # --session flag, sessions subcommands
```

---

## Performance Characteristics

| Operation | Target | Implementation |
|-----------|--------|----------------|
| Session load | < 50ms | Index on session ID |
| Message insert | < 10ms | Single row INSERT |
| List sessions | < 100ms | Index on updated_at |
| Cleanup | < 500ms | Bulk DELETE with transactions |
| Startup overhead | < 100ms | Migration runs only first time |

---

## Security Considerations

- Database stored locally at `~/.config/synapse/sessions.db`
- Messages stored in plaintext (user responsibility to secure)
- No authentication (single-user local CLI tool)
- WAL mode provides crash safety
- Foreign key constraints prevent orphan data

---

## Future Work

This implementation enables:
- **REPL Mode** - Persistent session within interactive mode
- **Session Search** - Find sessions by content or metadata
- **Session Export** - Export conversations to Markdown/JSON
- **PostgreSQL Support** - Swap SqliteStore for PostgresStore
- **Session Sharing** - Export/import sessions between installations
- **Summarization** - Compress long sessions to fit context window
