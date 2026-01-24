//! Synapse core library.
//!
//! Provides the agent orchestrator, LLM provider abstraction,
//! session management, and MCP integration.

pub mod config;
pub mod message;
pub mod provider;

pub use config::{Config, ConfigError};
pub use message::{Message, Role};
pub use provider::{LlmProvider, MockProvider, ProviderError};

/// Placeholder module for initial setup.
pub mod placeholder {
    /// Returns a greeting message.
    pub fn hello() -> &'static str {
        "Hello from synapse-core!"
    }
}
