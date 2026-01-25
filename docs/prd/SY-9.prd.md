# SY-9: Session Storage

Status: PRD_READY

## Context / Idea

### Phase 8: Session Storage

This phase implements persistent conversation storage using SQLite, enabling users to maintain context across sessions and manage their conversation history.

From `docs/phase/phase-8.md`:

**Goal:** Persist conversations to SQLite.

**Tasks:**
- 8.1 Create `synapse-core/src/session.rs` with `Session` struct
- 8.2 Create `synapse-core/src/storage.rs` with `SessionStore` trait
- 8.3 Add `sqlx` (sqlite feature), create `storage/database.rs`
- 8.4 Implement schema migrations (sessions + messages tables)
- 8.5 Wire storage into CLI: save messages after each exchange
- 8.6 Implement session limits (max sessions, retention period)
- 8.7 Add automatic cleanup job for expired sessions

**Session Storage Configuration** (from `config.example.toml`):
- `database_url`: Database connection URL (optional, env var takes priority)
- `max_sessions`: Maximum number of sessions to keep (default: 100)
- `retention_days`: Delete sessions older than this (default: 90)
- `auto_cleanup`: Enable automatic cleanup (default: true)

**Database URL Resolution Priority:**
1. `DATABASE_URL` environment variable (highest priority)
2. `session.database_url` in config.toml
3. Default: `sqlite:~/.config/synapse/sessions.db`

**Session IDs:** UUID v4 for universally unique identification.

**Database Schema:** Sessions and messages tables as defined in `docs/vision.md`:
- SQLite default at `~/.config/synapse/sessions.db`
- Configurable to PostgreSQL/MySQL via DATABASE_URL or config

**Cleanup Behavior:**
- Max sessions limit: oldest auto-deleted when exceeded
- Retention period: sessions older than `retention_days` purged
- Auto-cleanup runs on startup

### Project Context

From `docs/idea.md`: Synapse is an AI agent providing session-based conversation memory for coherent multi-turn dialogues. The project targets CLI, Telegram bot, and backend service interfaces.

From `docs/vision.md`: The architecture follows hexagonal patterns with `SessionStore` as a port (trait) and `SqliteStore` as an adapter. Storage strategy includes sessions in database (SQLite default, configurable to PostgreSQL/MySQL).

## Goals

1. **Persist conversations**: Store all user and assistant messages in SQLite for retrieval across CLI invocations
2. **Enable multi-turn context**: Allow users to continue previous conversations by session ID
3. **Provide session management**: List, view, and delete sessions via CLI commands
4. **Implement automatic cleanup**: Prevent unbounded database growth through configurable limits
5. **Maintain abstraction**: Keep storage backend swappable via the `SessionStore` trait

## User Stories

### US-1: Continue Previous Conversation
**As a** CLI user
**I want to** continue a previous conversation by session ID
**So that** I can maintain context across multiple CLI invocations

**Acceptance Criteria:**
- Running `synapse --session <id> "follow-up question"` loads conversation history
- The LLM receives the full conversation context
- New messages are appended to the existing session

### US-2: List Conversations
**As a** user
**I want to** see a list of my previous conversations
**So that** I can find and continue relevant sessions

**Acceptance Criteria:**
- Running `synapse sessions list` displays all sessions
- Each session shows: ID, provider, model, message count, preview
- Sessions are ordered by most recently updated first

### US-3: View Conversation History
**As a** user
**I want to** view the full message history of a session
**So that** I can review past conversations

**Acceptance Criteria:**
- Running `synapse sessions show <id>` displays all messages
- Messages show role (USER/ASSISTANT/SYSTEM) and content
- Session metadata (provider, model, created date) is displayed

### US-4: Delete Conversations
**As a** user
**I want to** delete sessions I no longer need
**So that** I can manage my conversation history

**Acceptance Criteria:**
- Running `synapse sessions delete <id>` removes the session
- All associated messages are also deleted (cascade)
- Confirmation message is displayed

### US-5: Automatic Cleanup
**As a** user
**I want** old sessions to be automatically cleaned up
**So that** my database does not grow unbounded

**Acceptance Criteria:**
- Sessions exceeding `max_sessions` limit are auto-deleted (oldest first)
- Sessions older than `retention_days` are purged
- Cleanup runs on startup when `auto_cleanup` is enabled
- Cleanup can be disabled via configuration

## Main Scenarios

### Scenario 1: New Conversation with Persistence
1. User runs `synapse "What is Rust?"`
2. System creates a new session with UUID
3. User message is stored in the database
4. LLM response streams to terminal
5. Assistant response is stored in the database
6. Session is available for continuation

### Scenario 2: Continue Existing Session
1. User runs `synapse --session abc-123 "Tell me more"`
2. System loads session abc-123 from database
3. Previous messages are retrieved
4. New user message is appended
5. LLM receives full conversation history
6. Response is stored and session updated

### Scenario 3: Session Management
1. User runs `synapse sessions list`
2. System displays table of sessions with metadata
3. User identifies session to continue or delete
4. User runs appropriate command

### Scenario 4: Automatic Cleanup on Startup
1. User runs any synapse command
2. If `auto_cleanup` is enabled, system checks limits
3. Sessions over `max_sessions` are deleted (oldest first)
4. Sessions older than `retention_days` are deleted
5. Normal command execution proceeds

## Success / Metrics

### Functional Metrics
- `synapse sessions list` shows previous conversations
- `synapse --session <id> "message"` successfully continues sessions
- Sessions persist across CLI restarts
- Cascade delete removes messages with sessions

### Performance Targets
| Operation | Target | Notes |
|-----------|--------|-------|
| Session load | < 50ms | SQLite with indexes |
| Message insert | < 10ms | Single row insert |
| List sessions | < 100ms | Even with 1000+ sessions |
| Cleanup | < 500ms | Bulk delete operations |

### Quality Metrics
- All storage operations have unit tests
- Integration tests verify persistence across restarts
- Migration runs automatically on first use
- WAL mode enabled for concurrent access

## Constraints and Assumptions

### Constraints
1. **SQLite as default**: Initial implementation uses SQLite only
2. **Local storage**: Database stored in user config directory
3. **No encryption**: Messages stored in plaintext (user responsibility)
4. **Single-user**: No authentication or multi-tenant support

### Assumptions
1. Users have write access to `~/.config/synapse/`
2. SQLite is sufficient for personal use workloads
3. Session history fits in memory for conversation context
4. Users understand session IDs are UUIDs

### Dependencies
- Phase 7 (Streaming Responses) complete
- `sqlx` crate with sqlite feature
- `uuid` crate for session IDs
- `chrono` crate for timestamps

## Risks

### R-1: Database Corruption (Low)
**Risk:** SQLite database corruption from crashes during writes
**Mitigation:** WAL mode enabled, sqlx handles transactions properly

### R-2: Large Session History (Medium)
**Risk:** Very long conversations may cause performance issues when loading context
**Mitigation:** Current implementation loads all messages; future phases may add pagination or summarization

### R-3: Migration Conflicts (Low)
**Risk:** Schema changes may conflict with existing databases
**Mitigation:** sqlx migrations with versioning; clear upgrade path documented

### R-4: Storage Path Issues (Low)
**Risk:** Users may not have access to default storage location
**Mitigation:** Configurable `database_url` in config or environment variable

## Open Questions

None - Implementation is complete and ready for final verification.

## Implementation Status

Based on codebase analysis, Phase 8 appears substantially implemented:

| Task | File | Status |
|------|------|--------|
| 8.1 Session struct | `synapse-core/src/session.rs` | Complete |
| 8.2 SessionStore trait | `synapse-core/src/storage.rs` | Complete |
| 8.3 SQLite storage | `synapse-core/src/storage/sqlite.rs` | Complete |
| 8.4 Schema migrations | `synapse-core/migrations/20250125_001_initial.sql` | Complete |
| 8.5 CLI integration | `synapse-cli/src/main.rs` | Complete |
| 8.6 Session limits | `SqliteStore::cleanup()` | Complete |
| 8.7 Auto-cleanup | CLI main.rs startup | Complete |

### Remaining Work
- Update `docs/tasklist.md` to mark tasks as complete
- Final integration testing
- Update CHANGELOG.md
