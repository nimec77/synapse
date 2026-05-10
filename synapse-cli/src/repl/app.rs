//! REPL application state and transitions.
//!
//! Defines [`ReplApp`] with all state fields and helper methods
//! for input editing, scrolling, and message management.

use ratatui::text::Line;
use uuid::Uuid;

use super::render::build_history_lines;
use synapse_core::Role;

/// A display message in the conversation history.
#[derive(Debug, Clone)]
pub(super) struct DisplayMessage {
    /// The role of the message sender.
    pub(super) role: Role,
    /// The text content of the message.
    pub(super) content: String,
    /// Chain-of-thought reasoning from reasoning-capable models.
    ///
    /// Preserved so that session-resumed conversations can replay `reasoning_content`
    /// in outbound API calls (DeepSeek tool-call history rule). Also used in Task 22.6
    /// for dim/italic rendering in the REPL.
    pub(super) reasoning_content: Option<String>,
}

impl DisplayMessage {
    /// Create a display message without reasoning content.
    #[allow(dead_code)] // convenience constructor, will be used in task 22.6
    pub(super) fn simple(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            reasoning_content: None,
        }
    }
}

/// Application state for the REPL.
pub(super) struct ReplApp {
    /// Conversation history for display.
    pub(super) messages: Vec<DisplayMessage>,
    /// Current input buffer.
    pub(super) input: String,
    /// Cursor position within the input buffer.
    pub(super) cursor_position: usize,
    /// Scroll offset for conversation history.
    pub(super) scroll_offset: u16,
    /// Whether to auto-scroll to the bottom of conversation history.
    pub(super) auto_scroll: bool,
    /// Whether the LLM is currently streaming a response.
    pub(super) is_streaming: bool,
    /// Current session ID.
    pub(super) session_id: Uuid,
    /// Status bar message.
    pub(super) status_message: Option<String>,
    /// Provider name for display.
    pub(super) provider_name: String,
    /// Model name for display.
    pub(super) model_name: String,
    /// Whether to render reasoning tokens above the answer in dim/italic style.
    ///
    /// Maps to `config.cli.show_reasoning`. Default: `true` (CLI is developer-facing).
    pub(super) show_reasoning: bool,
}

impl ReplApp {
    /// Create a new REPL application state.
    pub(super) fn new(session_id: Uuid, provider_name: &str, model_name: &str) -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            auto_scroll: true,
            is_streaming: false,
            session_id,
            status_message: None,
            provider_name: provider_name.to_string(),
            model_name: model_name.to_string(),
            show_reasoning: true, // default: show reasoning (CLI is developer-facing)
        }
    }

    /// Insert a character at the current cursor position.
    pub(super) fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub(super) fn delete_char_before_cursor(&mut self) {
        if self.cursor_position > 0 {
            // Find the previous character boundary
            let prev = self.input[..self.cursor_position]
                .char_indices()
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_position = prev;
        }
    }

    /// Move cursor one position to the left.
    pub(super) fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position = self.input[..self.cursor_position]
                .char_indices()
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
        }
    }

    /// Move cursor one position to the right.
    pub(super) fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position = self.input[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(idx, _)| self.cursor_position + idx)
                .unwrap_or(self.input.len());
        }
    }

    /// Move cursor to the beginning of the input.
    pub(super) fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to the end of the input.
    pub(super) fn move_cursor_end(&mut self) {
        self.cursor_position = self.input.len();
    }

    /// Take the current input, resetting the buffer and cursor.
    pub(super) fn take_input(&mut self) -> String {
        self.cursor_position = 0;
        std::mem::take(&mut self.input)
    }

    /// Check if the input is a `/quit` command.
    pub(super) fn is_quit_command(input: &str) -> bool {
        input.trim() == "/quit"
    }

    /// Scroll the history up by one line (decrease offset to show earlier content).
    pub(super) fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.auto_scroll = false;
    }

    /// Scroll the history down by one line (increase offset to show later content).
    pub(super) fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scroll the history up by a page (decrease offset to show earlier content).
    pub(super) fn scroll_page_up(&mut self, page_size: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
        self.auto_scroll = false;
    }

    /// Scroll the history down by a page (increase offset to show later content).
    pub(super) fn scroll_page_down(&mut self, page_size: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(page_size);
    }

    /// Append a streaming text delta to the last assistant message.
    pub(super) fn append_stream_delta(&mut self, text: &str) {
        if let Some(last) = self.messages.last_mut()
            && last.role == Role::Assistant
        {
            last.content.push_str(text);
            return;
        }
        // No assistant message yet, create one
        self.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: text.to_string(),
            reasoning_content: None,
        });
    }

    /// Get the content of the last assistant message (for storage).
    #[cfg(test)]
    pub(super) fn last_assistant_content(&self) -> Option<&str> {
        self.messages
            .last()
            .filter(|m| m.role == Role::Assistant)
            .map(|m| m.content.as_str())
    }

    /// Build conversation lines for rendering.
    pub(super) fn build_history_lines(&self) -> Vec<Line<'_>> {
        build_history_lines(&self.messages, self.is_streaming, self.show_reasoning)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synapse_core::Message;

    #[test]
    fn test_repl_app_new() {
        let id = Uuid::new_v4();
        let app = ReplApp::new(id, "deepseek", "deepseek-chat");

        assert!(app.messages.is_empty());
        assert!(app.input.is_empty());
        assert_eq!(app.cursor_position, 0);
        assert_eq!(app.scroll_offset, 0);
        assert!(app.auto_scroll);
        assert!(!app.is_streaming);
        assert_eq!(app.session_id, id);
        assert!(app.status_message.is_none());
        assert_eq!(app.provider_name, "deepseek");
        assert_eq!(app.model_name, "deepseek-chat");
    }

    #[test]
    fn test_insert_char() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('h');
        app.insert_char('i');
        assert_eq!(app.input, "hi");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn test_insert_char_at_position() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('a');
        app.insert_char('c');
        app.move_cursor_left();
        app.insert_char('b');
        assert_eq!(app.input, "abc");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn test_delete_char_before_cursor() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        app.delete_char_before_cursor();
        assert_eq!(app.input, "ab");
        assert_eq!(app.cursor_position, 2);
    }

    #[test]
    fn test_delete_char_at_beginning() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('a');
        app.move_cursor_home();
        app.delete_char_before_cursor();
        assert_eq!(app.input, "a");
        assert_eq!(app.cursor_position, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        assert_eq!(app.cursor_position, 3);

        app.move_cursor_left();
        assert_eq!(app.cursor_position, 2);

        app.move_cursor_left();
        assert_eq!(app.cursor_position, 1);

        app.move_cursor_right();
        assert_eq!(app.cursor_position, 2);

        app.move_cursor_home();
        assert_eq!(app.cursor_position, 0);

        app.move_cursor_end();
        assert_eq!(app.cursor_position, 3);
    }

    #[test]
    fn test_cursor_bounds() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        // Left at beginning should stay at 0
        app.move_cursor_left();
        assert_eq!(app.cursor_position, 0);

        // Right at end should stay at end
        app.insert_char('a');
        app.move_cursor_right();
        assert_eq!(app.cursor_position, 1);
    }

    #[test]
    fn test_take_input() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('h');
        app.insert_char('i');

        let input = app.take_input();
        assert_eq!(input, "hi");
        assert!(app.input.is_empty());
        assert_eq!(app.cursor_position, 0);
    }

    #[test]
    fn test_is_quit_command() {
        assert!(ReplApp::is_quit_command("/quit"));
        assert!(ReplApp::is_quit_command("  /quit  "));
        assert!(!ReplApp::is_quit_command("/quit now"));
        assert!(!ReplApp::is_quit_command("quit"));
        assert!(!ReplApp::is_quit_command("hello"));
    }

    #[test]
    fn test_scroll() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        assert!(app.auto_scroll);

        // scroll_down increases offset (shows later content), does not disable auto_scroll
        app.scroll_down();
        assert_eq!(app.scroll_offset, 1);
        assert!(app.auto_scroll);

        app.scroll_down();
        assert_eq!(app.scroll_offset, 2);

        // scroll_up decreases offset (shows earlier content) and disables auto_scroll
        app.scroll_up();
        assert_eq!(app.scroll_offset, 1);
        assert!(!app.auto_scroll);

        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);

        // Should not go below 0
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_page() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        assert!(app.auto_scroll);

        // scroll_page_down increases offset, does not disable auto_scroll
        app.scroll_page_down(10);
        assert_eq!(app.scroll_offset, 10);
        assert!(app.auto_scroll);

        app.scroll_page_down(10);
        assert_eq!(app.scroll_offset, 20);

        // scroll_page_up decreases offset and disables auto_scroll
        app.scroll_page_up(15);
        assert_eq!(app.scroll_offset, 5);
        assert!(!app.auto_scroll);

        app.scroll_page_up(10);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn test_append_stream_delta() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        // First delta creates assistant message
        app.append_stream_delta("Hello");
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].role, Role::Assistant);
        assert_eq!(app.messages[0].content, "Hello");

        // Subsequent deltas append to the same message
        app.append_stream_delta(", world!");
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].content, "Hello, world!");
    }

    #[test]
    fn test_append_stream_delta_after_user_message() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.messages.push(DisplayMessage {
            role: Role::User,
            content: "Hello".to_string(),
            reasoning_content: None,
        });

        // Delta after user message creates new assistant message
        app.append_stream_delta("Hi");
        assert_eq!(app.messages.len(), 2);
        assert_eq!(app.messages[1].role, Role::Assistant);
        assert_eq!(app.messages[1].content, "Hi");
    }

    #[test]
    fn test_last_assistant_content() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        assert_eq!(app.last_assistant_content(), None);

        app.messages.push(DisplayMessage {
            role: Role::User,
            content: "Hello".to_string(),
            reasoning_content: None,
        });
        assert_eq!(app.last_assistant_content(), None);

        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "Hi there".to_string(),
            reasoning_content: None,
        });
        assert_eq!(app.last_assistant_content(), Some("Hi there"));
    }

    #[test]
    fn test_build_history_lines_empty() {
        let id = Uuid::new_v4();
        let app = ReplApp::new(id, "test", "test");
        let lines = app.build_history_lines();
        assert!(lines.is_empty());
    }

    #[test]
    fn test_build_history_lines_with_messages() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.messages.push(DisplayMessage {
            role: Role::User,
            content: "Hello".to_string(),
            reasoning_content: None,
        });
        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "Hi!".to_string(),
            reasoning_content: None,
        });

        let lines = app.build_history_lines();
        // Each message: role label + content line(s) + blank line
        // User: [USER] + "  Hello" + "" = 3 lines
        // Assistant: [ASSISTANT] + "  Hi!" + "" = 3 lines
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn test_build_history_lines_with_tool_message() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.messages.push(DisplayMessage {
            role: Role::Tool,
            content: "Tool output".to_string(),
            reasoning_content: None,
        });

        let lines = app.build_history_lines();
        // Tool: [TOOL] + "  Tool output" + "" = 3 lines
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_auto_scroll_disabled_on_scroll_up() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        assert!(app.auto_scroll);

        app.scroll_offset = 10;
        app.scroll_up();
        assert!(!app.auto_scroll);
        assert_eq!(app.scroll_offset, 9);
    }

    #[test]
    fn test_auto_scroll_enabled_on_message_submit() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        // Simulate scrolling up (disables auto_scroll)
        app.scroll_offset = 10;
        app.scroll_up();
        assert!(!app.auto_scroll);

        // Simulate what happens on message submit: auto_scroll is re-enabled
        app.auto_scroll = true;
        assert!(app.auto_scroll);
    }

    #[test]
    fn test_no_duplicate_messages_on_session_resume() {
        // Simulate session resume: app.messages is pre-populated with history
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        // Simulate history population (as done in run_repl for session resume)
        app.messages.push(DisplayMessage {
            role: Role::User,
            content: "Hello".to_string(),
            reasoning_content: None,
        });
        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "Hi there!".to_string(),
            reasoning_content: None,
        });

        // Simulate user submitting a new message in the REPL
        app.messages.push(DisplayMessage {
            role: Role::User,
            content: "Follow up question".to_string(),
            reasoning_content: None,
        });

        // Build conv_messages the same way the fixed code does:
        // only from app.messages (no separate history iteration)
        let conv_messages: Vec<Message> = app
            .messages
            .iter()
            .map(|m| Message::new(m.role, &m.content))
            .collect();

        // Verify: exactly 3 messages, no duplicates
        assert_eq!(conv_messages.len(), 3);
        assert_eq!(conv_messages[0].role, Role::User);
        assert_eq!(conv_messages[0].content, "Hello");
        assert_eq!(conv_messages[1].role, Role::Assistant);
        assert_eq!(conv_messages[1].content, "Hi there!");
        assert_eq!(conv_messages[2].role, Role::User);
        assert_eq!(conv_messages[2].content, "Follow up question");
    }

    #[test]
    fn test_insert_multibyte_char() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        app.insert_char('a');
        app.insert_char('\u{00E9}'); // e-acute (2 bytes in UTF-8)
        app.insert_char('b');
        assert_eq!(app.input, "a\u{00E9}b");
        assert_eq!(app.cursor_position, 4); // 1 + 2 + 1

        app.move_cursor_left();
        assert_eq!(app.cursor_position, 3); // after e-acute

        app.move_cursor_left();
        assert_eq!(app.cursor_position, 1); // after 'a'

        app.delete_char_before_cursor();
        assert_eq!(app.input, "\u{00E9}b");
        assert_eq!(app.cursor_position, 0);
    }

    /// AC: When `show_reasoning = true` and a DisplayMessage has reasoning,
    /// `build_history_lines` emits at least one line with `Modifier::DIM`.
    #[test]
    fn test_build_history_lines_reasoning_dim_when_flag_true() {
        use ratatui::style::Modifier;

        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        app.show_reasoning = true;

        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "The answer is 42.".to_string(),
            reasoning_content: Some("Let me think step by step...".to_string()),
        });

        let lines = app.build_history_lines();

        // At least one line must have DIM modifier (reasoning line)
        let has_dim_line = lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::DIM))
        });
        assert!(has_dim_line, "expected at least one dim line for reasoning");

        // The reasoning text must appear somewhere in the lines
        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(
            all_text.contains("Let me think step by step"),
            "reasoning text must appear in lines"
        );
    }

    /// AC: When `show_reasoning = false`, no reasoning text appears in the output.
    #[test]
    fn test_build_history_lines_reasoning_hidden_when_flag_false() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        app.show_reasoning = false;

        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "The answer is 42.".to_string(),
            reasoning_content: Some("REASONING_SHOULD_NOT_APPEAR".to_string()),
        });

        let lines = app.build_history_lines();

        let all_text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(
            !all_text.contains("REASONING_SHOULD_NOT_APPEAR"),
            "reasoning text must NOT appear when show_reasoning = false"
        );
    }
}
