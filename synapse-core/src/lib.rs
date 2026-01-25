//! Synapse core library.
//!
//! Provides the agent orchestrator, LLM provider abstraction,
//! session management, and MCP integration.

pub mod config;
pub mod message;
pub mod provider;
pub mod session;
pub mod storage;

pub use config::{Config, ConfigError, SessionConfig};
pub use message::{Message, Role};
pub use provider::{
    AnthropicProvider, DeepSeekProvider, LlmProvider, MockProvider, ProviderError, StreamEvent,
    create_provider,
};
pub use session::{Session, SessionSummary, StoredMessage};
pub use storage::{CleanupResult, SessionStore, SqliteStore, StorageError, create_storage};

/// Placeholder module for initial setup.
pub mod placeholder {
    /// Returns a greeting message.
    pub fn hello() -> &'static str {
        "Hello from synapse-core!"
    }
}
