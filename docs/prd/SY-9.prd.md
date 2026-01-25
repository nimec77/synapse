# SY-9: Session Storage

Status: PRD_READY

## Context / Idea

**Phase 8: Session Storage** - Persist conversations to SQLite.

Synapse is an AI agent that serves as a unified interface to interact with multiple LLM providers. A core functionality specified in the project idea is "Session-based conversation memory: Maintain conversation context within a session for coherent multi-turn dialogues."

Currently, the CLI operates in a stateless manner where each invocation starts fresh without access to prior conversation history. This phase introduces persistent session storage using SQLite, enabling:

- Multi-turn conversations that survive application restarts
- Session management commands (`synapse sessions list`, etc.)
- Configurable retention policies and cleanup

### Technical Context from Vision

The architecture defines:
- **SessionStore trait**: A port (interface) for storage implementations
- **SqliteStore**: Default adapter implementation using sqlx
- **Database location**: `~/.config/synapse/sessions.db` (configurable via `DATABASE_URL`)
- **Schema**: `sessions` and `messages` tables with proper indexing

### Session Storage Configuration

From `config.example.toml`:
- `[session]` section with:
  - `max_sessions`: Maximum number of sessions to keep (default: 100)
  - `retention_days`: Delete sessions older than this (default: 90)
  - `auto_cleanup`: Enable automatic cleanup (default: true)

### Cleanup Behavior

- Max sessions limit: oldest auto-deleted when exceeded
- Retention period: sessions older than `retention_days` purged
- Auto-cleanup runs on startup and periodically

### Tasks from Phase Plan

- 8.1 Create `synapse-core/src/session.rs` with `Session` struct
- 8.2 Create `synapse-core/src/storage.rs` with `SessionStore` trait
- 8.3 Add `sqlx` (sqlite feature), create `storage/database.rs`
- 8.4 Implement schema migrations (sessions + messages tables)
- 8.5 Wire storage into CLI: save messages after each exchange
- 8.6 Implement session limits (max sessions, retention period)
- 8.7 Add automatic cleanup job for expired sessions

## Goals

1. **Enable persistent conversations**: Users can continue conversations across CLI invocations
2. **Provide session management**: Users can list, view, resume, and delete sessions
3. **Implement configurable retention**: Automatic cleanup based on max sessions and retention period
4. **Maintain clean architecture**: SessionStore trait allows future database backends (PostgreSQL, MySQL)
5. **Ensure data integrity**: Proper schema with foreign keys and indexes for performance

## User Stories

### US-1: Continue Previous Conversation
As a user, I want to resume a previous conversation so that I can continue my interaction with context from earlier messages.

### US-2: List My Sessions
As a user, I want to see a list of my previous sessions so that I can find and resume a specific conversation.

### US-3: View Session Details
As a user, I want to view the messages in a specific session so that I can review what was discussed.

### US-4: Delete a Session
As a user, I want to delete a session I no longer need so that I can manage my conversation history.

### US-5: Automatic Cleanup
As a user, I want old sessions to be automatically cleaned up based on my configuration so that storage does not grow unbounded.

### US-6: Start Fresh Session
As a user, I want to explicitly start a new session so that I can begin a conversation without prior context.

## Main Scenarios

### Scenario 1: New User First Run
1. User runs `synapse "Hello"` for the first time
2. System creates SQLite database at default location
3. System creates a new session and stores the user message
4. Provider generates response
5. System stores the assistant response
6. Response displayed to user

### Scenario 2: Resume Session
1. User runs `synapse sessions list`
2. System displays list of sessions with IDs, creation dates, and preview
3. User runs `synapse -r --session <id>` (REPL mode with session)
4. System loads session history from database
5. User continues conversation with full context available

### Scenario 3: Auto-Cleanup on Startup
1. User has configured `max_sessions = 50` and `retention_days = 30`
2. User runs any synapse command
3. System checks session count and ages
4. System deletes sessions exceeding limits (oldest first)
5. Normal operation continues

### Scenario 4: Session Management
1. User runs `synapse sessions list` - sees all sessions
2. User runs `synapse sessions show <id>` - sees messages in session
3. User runs `synapse sessions delete <id>` - session is removed
4. Confirmation message displayed

## Success / Metrics

### Functional Criteria
- `synapse sessions list` shows previous conversations
- Sessions persist across application restarts
- Messages maintain correct ordering within sessions
- Provider and model information is stored with each session
- Cleanup respects configured limits

### Performance Criteria
- Session load time < 50ms (per vision.md target)
- Database operations do not block streaming responses
- Cleanup operations complete within reasonable time for 1000+ sessions

### Technical Criteria
- Database schema matches vision.md specification
- sqlx migrations are idempotent and reversible
- SessionStore trait is generic enough for future backends
- Error handling provides meaningful feedback

## Constraints and Assumptions

### Constraints
- SQLite is the only implemented backend for this phase
- Database file permissions should be restricted (security)
- Must work offline (no network dependency for storage)
- Must be compatible with current Config system

### Assumptions
- Users have write access to `~/.config/synapse/` directory
- Single-user scenarios (no concurrent access concerns for SQLite)
- Reasonable session sizes (not millions of messages per session)
- Configuration `[session]` section is already defined in Config struct

### Technical Constraints
- Use `sqlx` with SQLite feature as specified in vision.md
- Follow hexagonal architecture (SessionStore as port)
- Maintain async compatibility with existing provider layer

## Risks

### R1: Database Corruption
**Risk**: SQLite database could become corrupted if application crashes during write.
**Mitigation**: Use WAL mode, proper transaction handling, and consider periodic backups.

### R2: Migration Failures
**Risk**: Schema migrations could fail on existing databases.
**Mitigation**: Implement idempotent migrations with rollback capability.

### R3: Performance Degradation with Large Sessions
**Risk**: Loading sessions with many messages could be slow.
**Mitigation**: Consider pagination or lazy loading for very long sessions.

### R4: Storage Space Exhaustion
**Risk**: Without cleanup, database could grow unbounded.
**Mitigation**: Default cleanup enabled, clear warnings when cleanup is disabled.

## Open Questions

None - the PRD is complete based on available context from:
- `docs/vision.md` - database schema, architecture, and data model
- `docs/phase/phase-8.md` - task breakdown and acceptance criteria
- `config.example.toml` - session configuration options
- `docs/idea.md` - project goals including session-based memory
