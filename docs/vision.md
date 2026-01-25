# Synapse: Technical Architecture Vision

## 1. Technology Stack

### Programming Language
- **Rust** (nightly)
- Edition: 2024

### Design Decision: Custom LLM Provider Implementation

Rather than using existing libraries (rig, genai, async-openai), the LLM provider layer will be built from scratch. This decision prioritizes:

1. **Deep learning** - Hands-on experience with async HTTP, SSE streaming, trait design
2. **Full control** - Custom retry logic, caching, error handling tailored to Synapse
3. **Minimal dependencies** - No heavy framework overhead
4. **MCP integration** - Seamless integration with rmcp without adapter layers

### Core Dependencies

| Category | Crate | Purpose |
|----------|-------|---------|
| Async Runtime | `tokio` | Async execution, I/O, timers |
| HTTP Client | `reqwest` | LLM API requests with streaming |
| SSE Parsing | `eventsource-stream` | Server-Sent Events stream parsing |
| Serialization | `serde`, `serde_json` | JSON parsing for API responses |
| Configuration | `toml`, `serde` | TOML config file parsing |
| CLI Framework | `clap` | Command-line argument parsing |
| CLI UI | `ratatui` + `crossterm` | Interactive REPL with rich display |
| Database | `sqlx` | Session persistence (supports SQLite, PostgreSQL, MySQL) |
| Error Handling | `thiserror`, `anyhow` | Library and application errors |
| Logging | `tracing`, `tracing-subscriber` | Structured logging |
| Telegram | `teloxide` | Telegram bot interface |
| MCP | `rmcp` | Model Context Protocol support |
| Testing | `tokio-test`, `mockall` | Async testing and mocking |
| Async Utilities | `futures`, `async-stream` | Stream combinators, async generators |

### Development Tools
- **cargo-watch**: Auto-rebuild on file changes
- **cargo-nextest**: Faster test runner
- **cargo-clippy**: Linting
- **cargo-fmt**: Code formatting
- **cargo-audit**: Security vulnerability scanning

---

## 2. Architecture Pattern

### Pattern: Hexagonal Architecture (Ports and Adapters)

```
                    ┌─────────────────────────────────────────────────────────┐
                    │                    Interfaces                           │
                    │  ┌─────────┐   ┌──────────────┐   ┌─────────────────┐   │
                    │  │   CLI   │   │   Telegram   │   │  Backend API    │   │
                    │  └────┬────┘   └──────┬───────┘   └────────┬────────┘   │
                    └───────┼───────────────┼────────────────────┼────────────┘
                            │               │                    │
                            ▼               ▼                    ▼
                    ┌─────────────────────────────────────────────────────────┐
                    │                 Application Layer                       │
                    │  ┌─────────────────────────────────────────────────┐    │
                    │  │              Agent Orchestrator                 │    │
                    │  │  - Session management                           │    │
                    │  │  - Message routing                              │    │
                    │  │  - MCP tool coordination                        │    │
                    │  └─────────────────────────────────────────────────┘    │
                    └─────────────────────────────────────────────────────────┘
                                              │
                            ┌─────────────────┼─────────────────┐
                            ▼                 ▼                 ▼
                    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
                    │   Ports      │  │   Ports      │  │   Ports      │
                    │  (Traits)    │  │  (Traits)    │  │  (Traits)    │
                    │              │  │              │  │              │
                    │ LlmProvider  │  │ SessionStore │  │  McpClient   │
                    └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
                           │                 │                 │
                           ▼                 ▼                 ▼
                    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
                    │   Adapters   │  │   Adapters   │  │   Adapters   │
                    │              │  │              │  │              │
                    │DeepSeek/Claude│  │   Database   │  │  MCP Server  │
                    └──────────────┘  └──────────────┘  └──────────────┘
```

### Justification

1. **Testability**: Core logic is isolated from external dependencies via traits (ports)
2. **Flexibility**: Easy to swap LLM providers or storage backends
3. **Maintainability**: Clear boundaries between layers
4. **Learning**: Demonstrates advanced Rust patterns (traits, generics, async)

---

## 3. Project Structure

### Flat Workspace Layout

```
synapse/
├── Cargo.toml                 # Workspace manifest
├── CLAUDE.md                  # Claude Code guidance
├── README.md                  # Project documentation
├── config.example.toml        # Example configuration
│
├── synapse-core/              # Core library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Public API exports
│       ├── agent.rs           # Agent orchestrator
│       ├── config.rs          # Configuration types
│       ├── error.rs           # Error types (thiserror)
│       ├── message.rs         # Message types
│       ├── session.rs         # Session management
│       │
│       ├── provider.rs        # Provider trait + module declarations
│       ├── provider/          # LLM provider implementations
│       │   ├── anthropic.rs   # Claude implementation
│       │   ├── openai.rs      # OpenAI implementation
│       │   └── streaming.rs   # Streaming response handling
│       │
│       ├── storage.rs         # Storage trait + module declarations
│       ├── storage/           # Persistence implementations
│       │   └── database.rs    # sqlx database implementation
│       │
│       ├── mcp.rs             # MCP client + module declarations
│       └── mcp/               # MCP components
│           ├── protocol.rs    # Protocol types
│           └── tools.rs       # Tool execution
│
├── synapse-cli/               # CLI binary crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Entry point
│       ├── args.rs            # CLI arguments (clap)
│       ├── repl.rs            # Interactive REPL
│       ├── oneshot.rs         # One-shot mode
│       └── ui.rs              # Terminal UI (ratatui)
│
├── synapse-telegram/          # Telegram bot crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Bot entry point
│       ├── handlers.rs        # Message handlers
│       └── keyboard.rs        # Inline keyboards
│
├── doc/                       # Documentation
│   ├── idea.md                # Project concept
│   └── vision.md              # Technical architecture (this file)
│
└── tests/                     # Integration tests
    ├── provider_tests.rs      # LLM provider tests
    └── session_tests.rs       # Session persistence tests
```

---

## 4. Core Components

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              synapse-core                               │
│                                                                         │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐   │
│  │      Agent       │───▶│  SessionManager  │───▶│   SessionStore   │   │
│  │   Orchestrator   │    │                  │    │     (trait)      │   │
│  └────────┬─────────┘    └──────────────────┘    └──────────────────┘   │
│           │                                                │            │
│           │              ┌──────────────────┐              ▼            │
│           ├─────────────▶│    McpClient     │    ┌──────────────────┐   │
│           │              │                  │    │  SqliteStore     │   │
│           │              └──────────────────┘    │  (impl)          │   │
│           │                       │              └──────────────────┘   │
│           ▼                       ▼                                     │
│  ┌──────────────────┐    ┌──────────────────┐                           │
│  │   LlmProvider    │    │   ToolRegistry   │                           │
│  │     (trait)      │    │                  │                           │
│  └────────┬─────────┘    └──────────────────┘                           │
│           │                                                             │
│     ┌─────┴─────┐                                                       │
│     ▼           ▼                                                       │
│ ┌────────┐ ┌────────┐                                                   │
│ │Anthropic│ │ OpenAI │                                                  │
│ └────────┘ └────────┘                                                   │
└──────────────────────────────────────────────────────────────────────── ┘
```

### Data Flow

1. **User Input** → Interface (CLI/Telegram) → Agent Orchestrator
2. **Agent** loads session from SessionStore
3. **Agent** appends user message to session
4. **Agent** calls LlmProvider with full conversation context
5. **LlmProvider** streams response tokens back
6. **Agent** checks for tool calls → McpClient executes tools
7. **Agent** saves updated session to SessionStore
8. **Response** streams back to Interface → User

### External Integrations

| Integration | Protocol | Purpose |
|-------------|----------|---------|
| DeepSeek API | HTTPS REST + SSE | DeepSeek models (default) |
| Anthropic API | HTTPS REST + SSE | Claude LLM provider |
| OpenAI API | HTTPS REST + SSE | GPT models |
| MCP Servers | JSON-RPC over stdio/SSE | Tool execution |
| Telegram API | HTTPS (via teloxide) | Bot interface |

---

## 5. Data Model

### Key Entities

```
┌─────────────────────┐       ┌─────────────────────┐
│      Session        │       │      Message        │
├─────────────────────┤       ├─────────────────────┤
│ id: Uuid            │ 1   * │ id: Uuid            │
│ name: Option<String>│◄─────▶│ session_id: Uuid    │
│ provider: String    │       │ role: Role          │
│ model: String       │       │ content: String     │
│ system_prompt: Text │       │ timestamp: DateTime │
│ created_at: DateTime│       │ tool_calls: Json    │
│ updated_at: DateTime│       │ tool_results: Json  │
└─────────────────────┘       └─────────────────────┘

┌─────────────────────┐       ┌─────────────────────┐
│   Configuration     │       │    McpServer        │
├─────────────────────┤       ├─────────────────────┤
│ default_provider    │       │ command: String     │
│ providers: Map      │       │ args: Vec<String>   │
│ system_prompt       │       │ env: Map<String,    │
│ session_db_path     │       │      String>        │
└─────────────────────┘       └─────────────────────┘

┌─────────────────────┐
│   McpConfig         │
├─────────────────────┤
│ mcpServers: Map<    │
│   String,McpServer> │
└─────────────────────┘
```

### Role Enum
```rust
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}
```

### Database Schema (SQLite default)

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
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tool_calls TEXT,  -- JSON array
    tool_results TEXT, -- JSON array
    timestamp TEXT NOT NULL
);

CREATE INDEX idx_messages_session ON messages(session_id);
CREATE INDEX idx_sessions_updated ON sessions(updated_at);
```

### Storage Strategy
- **Sessions**: Database (SQLite default at `~/.config/synapse/sessions.db`, configurable to PostgreSQL/MySQL)
- **Configuration**: TOML file at `~/.config/synapse/config.toml`
- **MCP Servers**: JSON file at `~/.config/synapse/mcp_servers.json` (standard format compatible with Claude Desktop, Windsurf, etc.)
- **Environment override**: `SYNAPSE_CONFIG` env var for custom path

### MCP Server Configuration Format

```json
{
  "mcpServers": {
    "figma-remote-mcp-server": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "https://mcp.figma.com/mcp"
      ],
      "env": {}
    },
    "dart-mcp-server": {
      "command": "dart",
      "args": ["mcp-server", "--force-roots-fallback"],
      "env": {}
    },
    "telegram": {
      "command": "/Applications/telegram-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

This format is compatible with other MCP-enabled agents, allowing users to share configurations.

---

## 6. API Design

### Internal API (synapse-core public interface)

```rust
// Agent creation and interaction
pub struct Agent { ... }

impl Agent {
    pub async fn new(config: Config) -> Result<Self>;
    pub async fn chat(&self, session_id: Uuid, message: &str) -> impl Stream<Item = Result<StreamEvent>>;
    pub async fn create_session(&self, options: SessionOptions) -> Result<Session>;
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>>;
    pub async fn delete_session(&self, id: Uuid) -> Result<()>;
}

// Stream events
pub enum StreamEvent {
    TextDelta(String),
    ToolCall { id: String, name: String, input: Value },
    ToolResult { id: String, output: Value },
    Done,
    Error(Error),
}
```

### CLI Interface

```bash
# One-shot mode
synapse "What is Rust?"
synapse -p openai "Explain async/await"
echo "Hello" | synapse

# REPL mode
synapse --repl
synapse -r --session <id>

# Session management
synapse sessions list
synapse sessions show <id>
synapse sessions delete <id>

# Configuration
synapse config show
synapse config set default_provider anthropic
```

### Error Handling Strategy

```rust
// Library errors (synapse-core) - typed with thiserror
#[derive(Debug, thiserror::Error)]
pub enum SynapseError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
}

// Application errors (CLI/Telegram) - use anyhow for ergonomics
fn main() -> anyhow::Result<()> { ... }
```

---

## 7. Performance Considerations

### Target Response Times
| Operation | Target | Notes |
|-----------|--------|-------|
| First token (streaming) | < 500ms | Depends on provider latency |
| Session load | < 50ms | SQLite with indexes |
| Config load | < 10ms | TOML parsing |
| MCP tool discovery | < 100ms | Cached after first call |

### Streaming Implementation
```rust
// Use async streams for token-by-token output
pub fn chat(...) -> impl Stream<Item = Result<StreamEvent>> {
    async_stream::stream! {
        // SSE parsing from provider
        while let Some(event) = sse_stream.next().await {
            yield Ok(StreamEvent::TextDelta(event.text));
        }
    }
}
```

### Caching Strategy
- **MCP tool schemas**: Cached in memory per server connection
- **Provider clients**: Reused HTTP connections via `reqwest::Client`
- **Session context**: Keep current session in memory, persist on changes

### Scalability Approach
- **Concurrent sessions**: Each CLI/Telegram instance is independent
- **Provider rate limiting**: Respect API rate limits, implement backoff
- **Database**: sqlx with connection pooling (SQLite WAL mode, or PostgreSQL/MySQL for multi-instance)

---

## 8. Testing Strategy

### Unit Test Targets
- Configuration parsing (`config.rs`)
- Message serialization (`message.rs`)
- Session management logic (`session.rs`)
- Provider response parsing (without HTTP calls)
- Error type conversions

### Integration Test Scope
- Database storage operations via sqlx (using temp databases)
- Full request/response cycle with mock HTTP server
- MCP protocol handling with test server
- CLI argument parsing

### E2E Test Coverage
- CLI one-shot mode with mock provider
- REPL session workflow
- Session persistence across restarts
- Configuration file handling

### Test Utilities
```rust
// Mock provider for testing
#[cfg(test)]
pub fn mock_provider() -> impl LlmProvider {
    MockProvider::new()
        .with_response("Hello! I'm a test response.")
}

// Test fixtures
#[cfg(test)]
mod fixtures {
    pub fn sample_session() -> Session { ... }
    pub fn sample_config() -> Config { ... }
}
```

---

## 9. Deployment & Environment

### Development Setup
```bash
# Clone and setup
git clone https://github.com/nimec77/synapse.git
cd synapse

# Install development tools
cargo install cargo-watch cargo-nextest

# Create config
cp config.example.toml ~/.config/synapse/config.toml
# Edit to add API keys

# Run in development
cargo watch -x 'run -p synapse-cli -- --repl'
```

### Building for Release
```bash
# Build optimized binaries
cargo build --release

# Binaries located at:
# target/release/synapse-cli
# target/release/synapse-telegram
```

### Installation Locations
| Platform | Config Path | Data Path |
|----------|-------------|-----------|
| Linux | `~/.config/synapse/` | `~/.local/share/synapse/` |
| macOS | `~/.config/synapse/` | `~/Library/Application Support/synapse/` |
| Windows | `%APPDATA%\synapse\` | `%LOCALAPPDATA%\synapse\` |

### Environment Variables
| Variable | Purpose | Default |
|----------|---------|---------|
| `SYNAPSE_CONFIG` | Config file path | Platform default |
| `SYNAPSE_MCP_CONFIG` | MCP servers JSON file path | `~/.config/synapse/mcp_servers.json` |
| `SYNAPSE_LOG` | Log level | `info` |
| `DATABASE_URL` | Database connection string | `sqlite:~/.config/synapse/sessions.db` |
| `DEEPSEEK_API_KEY` | DeepSeek API key | From config |
| `ANTHROPIC_API_KEY` | Claude API key | From config |
| `OPENAI_API_KEY` | OpenAI API key | From config |

---

## 10. Monitoring & Logging

### Logging Strategy
```rust
// Using tracing crate
use tracing::{debug, info, warn, error, instrument};

#[instrument(skip(self, message))]
pub async fn chat(&self, session_id: Uuid, message: &str) -> Result<...> {
    info!(session_id = %session_id, "Starting chat");
    debug!(message_len = message.len(), "Message received");
    // ...
}
```

### Log Levels
| Level | Usage |
|-------|-------|
| `error` | Unrecoverable errors, API failures |
| `warn` | Rate limits, retries, deprecations |
| `info` | Session starts, provider selection |
| `debug` | Request/response details, parsing |
| `trace` | Raw API payloads (development only) |

### Metrics (Future)
- Request latency per provider
- Token usage per session
- Error rates by type
- Session duration and message count

### Error Tracking
- Log errors with full context (session ID, provider, model)
- Stderr for CLI errors with user-friendly messages
- Structured JSON logs available via `SYNAPSE_LOG_FORMAT=json`

---

## 11. Security Considerations

### API Key Management
- **Never** store API keys in code or version control
- Configuration file should have restricted permissions (`chmod 600`)
- Support environment variables as override
- Warn user if config file has open permissions

### Secure Defaults
```rust
// Validate config permissions on load
fn check_config_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = path.metadata()?.permissions().mode();
        if mode & 0o077 != 0 {
            warn!("Config file has open permissions, consider: chmod 600 {:?}", path);
        }
    }
    Ok(())
}
```

### HTTPS Requirements
- All provider APIs use HTTPS exclusively
- TLS certificate validation enabled (reqwest default)
- No HTTP fallback

### MCP Security
- MCP servers run as separate processes
- Sandboxing via subprocess isolation
- Configurable allowed commands in config
- Log all tool executions

### Input Validation
- Sanitize user input before logging (no secrets in logs)
- Validate session IDs as valid UUIDs
- Limit message size to prevent memory exhaustion

---

## 12. Development Workflow

### Version Control Strategy
- **Main branch**: `master` - always deployable
- **Feature branches**: `feature/<name>` - for new features
- **Fix branches**: `fix/<issue>` - for bug fixes

### Branch Management
```bash
# Create feature branch
git checkout -b feature/mcp-support

# Keep up to date
git fetch origin
git rebase origin/master

# Merge via PR
gh pr create --title "Add MCP support"
```

### Commit Convention
```
<type>: <short description>

<optional body>

Types: feat, fix, docs, refactor, test, chore
```

### Code Review Process
1. Create PR with description and test plan
2. Ensure CI passes (tests, clippy, fmt)
3. Self-review changes
4. Merge when ready (personal project)

### Pre-commit Checks
```bash
# Format check
cargo fmt --check

# Lint
cargo clippy -- -D warnings

# Test
cargo test

# Security audit
cargo audit
```

### Release Process
1. Update version in Cargo.toml files
2. Update CHANGELOG.md
3. Create git tag: `git tag v0.1.0`
4. Build release binaries
5. Create GitHub release with binaries

---

## Summary

This architecture provides:

- **Modularity**: Clean separation between core logic and interfaces
- **Testability**: Traits enable easy mocking and testing
- **Extensibility**: New providers and interfaces can be added without changing core
- **Learning**: Demonstrates Rust ownership, async, traits, and error handling
- **Production-ready patterns**: Proper logging, configuration, and security

The project structure supports incremental development: start with CLI + one provider, then add more providers, then Telegram, then MCP.
