//! Session types for conversation persistence.
//!
//! Provides data structures for storing and managing conversation sessions
//! and their associated messages.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::message::Role;

/// A conversation session containing metadata.
///
/// Sessions group related messages together and track provider/model configuration
/// used during the conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct Session {
    /// Unique identifier for the session.
    pub id: Uuid,
    /// Optional human-readable name for the session.
    pub name: Option<String>,
    /// The LLM provider used (e.g., "deepseek", "anthropic").
    pub provider: String,
    /// The model name used (e.g., "deepseek-chat", "claude-3-opus").
    pub model: String,
    /// Optional system prompt used for this session.
    pub system_prompt: Option<String>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last updated (message added).
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with the given provider and model.
    ///
    /// Generates a new UUID v7 (time-sortable) and sets timestamps to current time.
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            name: None,
            provider: provider.into(),
            model: model.into(),
            system_prompt: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set the name for this session.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the system prompt for this session.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

/// Summary information for listing sessions.
///
/// Contains essential metadata for displaying sessions in a list view,
/// including message count and a preview of the first user message.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSummary {
    /// Unique identifier for the session.
    pub id: Uuid,
    /// Optional human-readable name for the session.
    pub name: Option<String>,
    /// The LLM provider used.
    pub provider: String,
    /// The model name used.
    pub model: String,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When the session was last updated.
    pub updated_at: DateTime<Utc>,
    /// Number of messages in the session.
    pub message_count: u32,
    /// Preview of the first user message (truncated to 50 chars).
    pub preview: Option<String>,
}

/// A message stored in the database with full metadata.
///
/// Represents a single message within a session, including its unique ID,
/// the session it belongs to, and timestamp information.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredMessage {
    /// Unique identifier for the message.
    pub id: Uuid,
    /// The session this message belongs to.
    pub session_id: Uuid,
    /// The role of the message sender.
    pub role: Role,
    /// The text content of the message.
    pub content: String,
    /// JSON-serialized tool calls (for assistant messages with tool calls).
    pub tool_calls: Option<String>,
    /// JSON-serialized tool results (for tool result messages).
    pub tool_results: Option<String>,
    /// When the message was created.
    pub timestamp: DateTime<Utc>,
}

impl StoredMessage {
    /// Create a new stored message.
    ///
    /// Generates a UUID v7 (time-sortable) for the message ID.
    /// Tool-related fields default to `None`.
    pub fn new(session_id: Uuid, role: Role, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            session_id,
            role,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            timestamp: Utc::now(),
        }
    }

    /// Set tool calls JSON data on this message.
    pub fn with_tool_calls(mut self, tool_calls: impl Into<String>) -> Self {
        self.tool_calls = Some(tool_calls.into());
        self
    }

    /// Set tool results JSON data on this message.
    pub fn with_tool_results(mut self, tool_results: impl Into<String>) -> Self {
        self.tool_results = Some(tool_results.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new("deepseek", "deepseek-chat");

        assert!(!session.id.is_nil());
        assert_eq!(session.name, None);
        assert_eq!(session.provider, "deepseek");
        assert_eq!(session.model, "deepseek-chat");
        assert_eq!(session.system_prompt, None);
        assert!(session.created_at <= Utc::now());
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn test_session_with_name() {
        let session = Session::new("anthropic", "claude-3-opus").with_name("My Chat");

        assert_eq!(session.name, Some("My Chat".to_string()));
    }

    #[test]
    fn test_session_with_system_prompt() {
        let session =
            Session::new("openai", "gpt-4").with_system_prompt("You are a helpful assistant.");

        assert_eq!(
            session.system_prompt,
            Some("You are a helpful assistant.".to_string())
        );
    }

    #[test]
    fn test_session_field_access() {
        let session = Session::new("test-provider", "test-model")
            .with_name("Test Session")
            .with_system_prompt("Test prompt");

        assert_eq!(session.provider, "test-provider");
        assert_eq!(session.model, "test-model");
        assert_eq!(session.name.as_deref(), Some("Test Session"));
        assert_eq!(session.system_prompt.as_deref(), Some("Test prompt"));
    }

    #[test]
    fn test_session_summary_construction() {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let summary = SessionSummary {
            id,
            name: Some("Test".to_string()),
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            created_at: now,
            updated_at: now,
            message_count: 5,
            preview: Some("Hello, how are you?".to_string()),
        };

        assert_eq!(summary.id, id);
        assert_eq!(summary.name, Some("Test".to_string()));
        assert_eq!(summary.message_count, 5);
        assert_eq!(summary.preview, Some("Hello, how are you?".to_string()));
    }

    #[test]
    fn test_stored_message_new() {
        let session_id = Uuid::new_v4();
        let msg = StoredMessage::new(session_id, Role::User, "Hello!");

        assert!(!msg.id.is_nil());
        assert_eq!(msg.session_id, session_id);
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello!");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_results.is_none());
        assert!(msg.timestamp <= Utc::now());
    }

    #[test]
    fn test_stored_message_field_access() {
        let session_id = Uuid::new_v4();
        let msg = StoredMessage::new(session_id, Role::Assistant, "I'm doing well!");

        assert_eq!(msg.session_id, session_id);
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "I'm doing well!");
    }

    #[test]
    fn test_session_clone() {
        let original = Session::new("provider", "model").with_name("Test");
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_stored_message_clone() {
        let session_id = Uuid::new_v4();
        let original = StoredMessage::new(session_id, Role::User, "Test");
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_stored_message_with_tool_calls() {
        let session_id = Uuid::new_v4();
        let msg = StoredMessage::new(session_id, Role::Assistant, "")
            .with_tool_calls(r#"[{"id":"call_1","name":"test","input":{}}]"#);

        assert!(msg.tool_calls.is_some());
        assert_eq!(
            msg.tool_calls.as_deref(),
            Some(r#"[{"id":"call_1","name":"test","input":{}}]"#)
        );
        assert!(msg.tool_results.is_none());
    }

    #[test]
    fn test_stored_message_with_tool_results() {
        let session_id = Uuid::new_v4();
        let msg = StoredMessage::new(session_id, Role::Tool, "result content")
            .with_tool_results(r#"{"tool_call_id":"call_1"}"#);

        assert!(msg.tool_results.is_some());
        assert_eq!(
            msg.tool_results.as_deref(),
            Some(r#"{"tool_call_id":"call_1"}"#)
        );
        assert!(msg.tool_calls.is_none());
    }
}
