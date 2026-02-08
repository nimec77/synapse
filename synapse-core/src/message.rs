//! Message types for LLM conversations.
//!
//! Provides the [`Role`] enum, [`Message`] struct, and [`ToolCallData`]
//! that represent conversation messages across all LLM providers.

use serde::{Deserialize, Serialize};

/// Role of a message in the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System instructions for the model.
    System,
    /// User input.
    User,
    /// Model response.
    Assistant,
    /// Tool result message.
    Tool,
}

/// Data for a single tool call requested by the LLM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallData {
    /// Unique identifier for this tool call.
    pub id: String,
    /// Name of the tool to invoke.
    pub name: String,
    /// JSON input for the tool.
    pub input: serde_json::Value,
}

/// A single message in a conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The role of this message.
    pub role: Role,
    /// The text content of this message.
    pub content: String,
    /// Tool calls requested by the assistant (present when role == Assistant).
    pub tool_calls: Option<Vec<ToolCallData>>,
    /// Tool call ID this message responds to (present when role == Tool).
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a new message with the given role and content.
    ///
    /// # Arguments
    ///
    /// * `role` - The role of the message sender
    /// * `content` - The message content (accepts `&str` or `String`)
    ///
    /// # Examples
    ///
    /// ```
    /// use synapse_core::message::{Message, Role};
    ///
    /// let msg = Message::new(Role::User, "Hello!");
    /// assert_eq!(msg.role, Role::User);
    /// assert_eq!(msg.content, "Hello!");
    /// assert!(msg.tool_calls.is_none());
    /// assert!(msg.tool_call_id.is_none());
    /// ```
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a tool result message.
    ///
    /// Creates a message with `Role::Tool` that carries the result of a tool
    /// invocation back to the LLM.
    ///
    /// # Arguments
    ///
    /// * `tool_call_id` - The ID of the tool call this result responds to
    /// * `content` - The tool result content
    ///
    /// # Examples
    ///
    /// ```
    /// use synapse_core::message::{Message, Role};
    ///
    /// let msg = Message::tool_result("call_1", "result text");
    /// assert_eq!(msg.role, Role::Tool);
    /// assert_eq!(msg.tool_call_id, Some("call_1".to_string()));
    /// assert_eq!(msg.content, "result text");
    /// ```
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_new_with_str() {
        let msg = Message::new(Role::User, "Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn test_message_new_with_string() {
        let msg = Message::new(Role::Assistant, String::from("Response"));
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Response");
    }

    #[test]
    fn test_role_equality() {
        assert_eq!(Role::System, Role::System);
        assert_ne!(Role::User, Role::Assistant);
    }

    #[test]
    fn test_message_clone() {
        let original = Message::new(Role::System, "Instructions");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_role_copy() {
        let role = Role::User;
        let copied = role;
        assert_eq!(role, copied);
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(serde_json::to_string(&Role::System).unwrap(), "\"system\"");
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&Role::Assistant).unwrap(),
            "\"assistant\""
        );
        assert_eq!(serde_json::to_string(&Role::Tool).unwrap(), "\"tool\"");
    }

    #[test]
    fn test_role_deserialization() {
        assert_eq!(
            serde_json::from_str::<Role>("\"system\"").unwrap(),
            Role::System
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"user\"").unwrap(),
            Role::User
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"assistant\"").unwrap(),
            Role::Assistant
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"tool\"").unwrap(),
            Role::Tool
        );
    }

    #[test]
    fn test_role_tool_serialization() {
        // AC1: Role::Tool serializes to "tool" and "tool" deserializes to Role::Tool
        let serialized = serde_json::to_string(&Role::Tool).unwrap();
        assert_eq!(serialized, "\"tool\"");

        let deserialized: Role = serde_json::from_str("\"tool\"").unwrap();
        assert_eq!(deserialized, Role::Tool);
    }

    #[test]
    fn test_message_new_backward_compatible() {
        // AC2: Message::new(Role::User, "hello") still works
        let msg = Message::new(Role::User, "hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "hello");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn test_message_tool_result() {
        // AC3: Message::tool_result creates correct message
        let msg = Message::tool_result("call_1", "result text");
        assert_eq!(msg.role, Role::Tool);
        assert_eq!(msg.tool_call_id, Some("call_1".to_string()));
        assert_eq!(msg.content, "result text");
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_message_with_tool_calls() {
        // AC4: Message with tool_calls populated serializes/deserializes correctly
        let tool_calls = vec![
            ToolCallData {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
                input: serde_json::json!({"location": "London"}),
            },
            ToolCallData {
                id: "call_2".to_string(),
                name: "list_files".to_string(),
                input: serde_json::json!({"path": "/tmp"}),
            },
        ];

        // Serialize/deserialize ToolCallData
        let json = serde_json::to_string(&tool_calls).unwrap();
        let deserialized: Vec<ToolCallData> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].id, "call_1");
        assert_eq!(deserialized[0].name, "get_weather");
        assert_eq!(deserialized[1].id, "call_2");
        assert_eq!(deserialized[1].name, "list_files");
    }

    #[test]
    fn test_tool_call_data_clone() {
        let tool_call = ToolCallData {
            id: "call_1".to_string(),
            name: "test_tool".to_string(),
            input: serde_json::json!({"key": "value"}),
        };
        let cloned = tool_call.clone();
        assert_eq!(tool_call, cloned);
    }
}
