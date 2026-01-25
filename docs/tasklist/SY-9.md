# SY-9: Session Storage

Status: IMPLEMENT_STEP_OK

Context: PRD `docs/prd/SY-9.prd.md`; Plan `docs/plan/SY-9.md`

---

## Phase 1: Foundation

### Task 8.1: Create Session Types

- [x] Create `synapse-core/src/session.rs` with `Session`, `SessionSummary`, and `StoredMessage` structs
  - AC1: `Session` struct has fields: `id` (Uuid), `name` (Option<String>), `provider` (String), `model` (String), `system_prompt` (Option<String>), `created_at` (DateTime<Utc>), `updated_at` (DateTime<Utc>)
  - AC2: `SessionSummary` struct has fields for listing: `id`, `name`, `provider`, `model`, `created_at`, `updated_at`, `message_count` (u32), `preview` (Option<String>)
  - AC3: `StoredMessage` struct has fields: `id` (Uuid), `session_id` (Uuid), `role` (Role), `content` (String), `timestamp` (DateTime<Utc>)
  - AC4: Unit tests verify struct construction and field access

### Task 8.2: Create Storage Trait and Error Types

- [x] Create `synapse-core/src/storage.rs` with `SessionStore` trait and `StorageError` enum
  - AC1: `StorageError` enum has variants: `Database`, `NotFound(Uuid)`, `Migration`, `InvalidData(String)` using thiserror
  - AC2: `CleanupResult` struct has fields: `sessions_deleted`, `by_max_limit`, `by_retention` (all u32)
  - AC3: `SessionStore` trait declares all 8 async methods: `create_session`, `get_session`, `list_sessions`, `touch_session`, `delete_session`, `add_message`, `get_messages`, `cleanup`
  - AC4: Module declares `pub mod sqlite;` for sqlite submodule

---

## Phase 2: Database Setup

### Task 8.3: Add Dependencies and Create SqliteStore

- [x] Add required dependencies to `synapse-core/Cargo.toml`
  - AC1: `sqlx` added with features `["runtime-tokio", "sqlite"]`
  - AC2: `uuid` added with features `["v4", "serde"]`
  - AC3: `chrono` added with features `["serde"]`
  - AC4: `cargo check` passes

- [x] Create `synapse-core/src/storage/sqlite.rs` with `SqliteStore` struct
  - AC1: `SqliteStore` struct holds `SqlitePool`
  - AC2: `SqliteStore::new(database_url: &str)` creates connection pool with WAL mode enabled
  - AC3: `SqliteStore::new()` runs migrations automatically on startup
  - AC4: Factory function `create_storage(database_url: Option<&str>)` returns `Box<dyn SessionStore>`, defaults to `~/.config/synapse/sessions.db`

### Task 8.4: Schema Migrations

- [x] Create migration file and generate sqlx metadata
  - AC1: `synapse-core/migrations/20250125_001_initial.sql` creates `sessions` table with all columns
  - AC2: Migration creates `messages` table with foreign key to sessions (ON DELETE CASCADE)
  - AC3: Migration creates indexes: `idx_messages_session`, `idx_messages_timestamp`, `idx_sessions_updated`
  - AC4: Running `cargo sqlx prepare` generates `.sqlx/` directory with query metadata
  - AC5: Migration is idempotent (uses IF NOT EXISTS)

---

## Phase 3: Storage Implementation

### Task 8.5: Implement SessionStore CRUD Methods

- [x] Implement session CRUD operations in SqliteStore
  - AC1: `create_session()` inserts session with all fields, returns `Ok(())`
  - AC2: `get_session()` returns `Some(Session)` if found, `None` if not
  - AC3: `list_sessions()` returns `Vec<SessionSummary>` ordered by `updated_at` DESC, includes message count and preview (first user message, truncated to 50 chars)
  - AC4: `touch_session()` updates `updated_at` to current timestamp
  - AC5: `delete_session()` returns `true` if deleted, `false` if not found

- [x] Implement message operations in SqliteStore
  - AC1: `add_message()` inserts message and updates session's `updated_at`
  - AC2: `get_messages()` returns `Vec<StoredMessage>` ordered by `timestamp` ASC
  - AC3: Integration test creates session, adds messages, retrieves them in order

### Task 8.6: Implement Cleanup Logic

- [x] Implement `cleanup()` method in SqliteStore
  - AC1: When session count exceeds `config.max_sessions`, delete oldest sessions until at limit
  - AC2: Delete sessions where `updated_at` is older than `retention_days` from current time
  - AC3: Return `CleanupResult` with accurate counts for `sessions_deleted`, `by_max_limit`, `by_retention`
  - AC4: Integration test verifies cleanup deletes correct sessions based on config

---

## Phase 4: Configuration and CLI Integration

### Task 8.7a: Add SessionConfig to Configuration

- [x] Add `SessionConfig` struct to `synapse-core/src/config.rs`
  - AC1: `SessionConfig` has fields: `max_sessions` (u32, default 100), `retention_days` (u32, default 90), `auto_cleanup` (bool, default true)
  - AC2: `Config` struct has optional `session: Option<SessionConfig>` field
  - AC3: Unit test verifies defaults are applied when `[session]` section is omitted
  - AC4: Unit test verifies parsing of custom `[session]` values from TOML

- [x] Add Serialize/Deserialize to Role enum
  - AC1: `Role` enum in `message.rs` has `#[derive(Serialize, Deserialize)]`
  - AC2: Role serializes to lowercase strings: "system", "user", "assistant"

### Task 8.7b: Add Session Flag and Subcommands to CLI

- [x] Add `--session` flag to CLI arguments
  - AC1: `--session <UUID>` or `-s <UUID>` flag accepts session ID to continue
  - AC2: When `--session` provided, load existing session and messages before sending
  - AC3: Error message shown if session ID not found

- [x] Add `sessions` subcommand with actions
  - AC1: `synapse sessions list` displays table of sessions with ID, name, provider, model, created, messages count
  - AC2: `synapse sessions show <ID>` displays all messages in session with role and content
  - AC3: `synapse sessions delete <ID>` removes session and shows confirmation
  - AC4: Add `uuid` dependency to `synapse-cli/Cargo.toml`

### Task 8.7c: Wire Storage into Chat Flow

- [x] Integrate storage into main chat logic
  - AC1: On startup, create `SqliteStore` using config or default path
  - AC2: If `auto_cleanup` enabled, run cleanup before processing message
  - AC3: Each new conversation creates a session and stores user message before calling provider
  - AC4: After receiving response (streaming or non-streaming), store assistant message
  - AC5: When continuing session (`--session`), load history and append to conversation

- [x] Update exports in `synapse-core/src/lib.rs`
  - AC1: Export `Session`, `SessionSummary`, `StoredMessage` from session module
  - AC2: Export `SessionStore`, `StorageError`, `CleanupResult`, `create_storage` from storage module
  - AC3: Export `SessionConfig` from config module

---

## Verification

- [ ] Manual test: `synapse "Hello"` creates session and stores messages in database
- [ ] Manual test: `synapse sessions list` shows the created session
- [ ] Manual test: `synapse sessions show <id>` displays conversation history
- [ ] Manual test: `synapse --session <id> "Follow up"` continues conversation with context
- [ ] Manual test: `synapse sessions delete <id>` removes session
- [x] CI passes: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
