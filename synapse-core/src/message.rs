//! Message types for LLM conversations.
//!
//! Provides the [`Role`] enum and [`Message`] struct that represent
//! conversation messages across all LLM providers.

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
}

/// A single message in a conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// The role of this message.
    pub role: Role,
    /// The text content of this message.
    pub content: String,
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
    /// ```
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
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
    }
}
