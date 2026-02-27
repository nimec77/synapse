//! Synapse core library.
//!
//! Provides the agent orchestrator, LLM provider abstraction,
//! session management, and MCP integration.

pub mod agent;
pub mod config;
pub mod mcp;
pub mod message;
pub mod provider;
pub mod session;
pub mod storage;
pub mod text;

pub use agent::{Agent, AgentError};
pub use config::{Config, TelegramConfig};
pub use mcp::{McpClient, init_mcp_client, load_mcp_config};
pub use message::{Message, Role};
pub use provider::{LlmProvider, StreamEvent, create_provider};
pub use session::{Session, SessionSummary, StoredMessage};
pub use storage::{SessionStore, create_storage};
