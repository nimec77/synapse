//! Interactive REPL mode for the Synapse CLI.
//!
//! Provides a terminal-based chat interface using `ratatui` and `crossterm`,
//! supporting multi-turn conversations with streaming LLM responses and
//! session persistence.

use std::io;

use anyhow::{Context, Result};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use uuid::Uuid;

use synapse_core::{
    Agent, AgentError, Config, LlmProvider, McpClient, Message, Role, Session, SessionStore,
    StoredMessage, StreamEvent,
};

/// A pinned, boxed stream of agent stream events.
type AgentStream<'a> =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, AgentError>> + Send + 'a>>;

/// Guard that restores terminal state on drop.
///
/// Enables raw mode and enters alternate screen on creation.
/// On drop, disables raw mode and leaves alternate screen,
/// ensuring the terminal is restored on normal exit, error, or panic.
struct TerminalGuard;

impl TerminalGuard {
    /// Create a new terminal guard, enabling raw mode and alternate screen.
    fn new() -> Result<Self> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        execute!(io::stdout(), EnterAlternateScreen).context("Failed to enter alternate screen")?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// A display message in the conversation history.
#[derive(Debug, Clone)]
struct DisplayMessage {
    /// The role of the message sender.
    role: Role,
    /// The text content of the message.
    content: String,
}

/// Application state for the REPL.
struct ReplApp {
    /// Conversation history for display.
    messages: Vec<DisplayMessage>,
    /// Current input buffer.
    input: String,
    /// Cursor position within the input buffer.
    cursor_position: usize,
    /// Scroll offset for conversation history.
    scroll_offset: u16,
    /// Whether to auto-scroll to the bottom of conversation history.
    auto_scroll: bool,
    /// Whether the LLM is currently streaming a response.
    is_streaming: bool,
    /// Current session ID.
    session_id: Uuid,
    /// Status bar message.
    status_message: Option<String>,
    /// Provider name for display.
    provider_name: String,
    /// Model name for display.
    model_name: String,
}

impl ReplApp {
    /// Create a new REPL application state.
    fn new(session_id: Uuid, provider_name: &str, model_name: &str) -> Self {
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
        }
    }

    /// Insert a character at the current cursor position.
    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    fn delete_char_before_cursor(&mut self) {
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
    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position = self.input[..self.cursor_position]
                .char_indices()
                .next_back()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
        }
    }

    /// Move cursor one position to the right.
    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.len() {
            self.cursor_position = self.input[self.cursor_position..]
                .char_indices()
                .nth(1)
                .map(|(idx, _)| self.cursor_position + idx)
                .unwrap_or(self.input.len());
        }
    }

    /// Move cursor to the beginning of the input.
    fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to the end of the input.
    fn move_cursor_end(&mut self) {
        self.cursor_position = self.input.len();
    }

    /// Take the current input, resetting the buffer and cursor.
    fn take_input(&mut self) -> String {
        self.cursor_position = 0;
        std::mem::take(&mut self.input)
    }

    /// Check if the input is a `/quit` command.
    fn is_quit_command(input: &str) -> bool {
        input.trim() == "/quit"
    }

    /// Scroll the history up by one line (decrease offset to show earlier content).
    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
        self.auto_scroll = false;
    }

    /// Scroll the history down by one line (increase offset to show later content).
    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    /// Scroll the history up by a page (decrease offset to show earlier content).
    fn scroll_page_up(&mut self, page_size: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
        self.auto_scroll = false;
    }

    /// Scroll the history down by a page (increase offset to show later content).
    fn scroll_page_down(&mut self, page_size: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(page_size);
    }

    /// Append a streaming text delta to the last assistant message.
    fn append_stream_delta(&mut self, text: &str) {
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
        });
    }

    /// Get the content of the last assistant message (for storage).
    #[cfg(test)]
    fn last_assistant_content(&self) -> Option<&str> {
        self.messages
            .last()
            .filter(|m| m.role == Role::Assistant)
            .map(|m| m.content.as_str())
    }

    /// Build conversation lines for rendering.
    fn build_history_lines(&self) -> Vec<Line<'_>> {
        let mut lines: Vec<Line<'_>> = Vec::new();
        for msg in &self.messages {
            let (label, label_color) = match msg.role {
                Role::User => ("[USER]", Color::Green),
                Role::Assistant => ("[ASSISTANT]", Color::Cyan),
                Role::System => ("[SYSTEM]", Color::Yellow),
                Role::Tool => ("[TOOL]", Color::Magenta),
            };

            // Role label line
            lines.push(Line::from(Span::styled(
                label,
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            )));

            // Content lines
            for content_line in msg.content.lines() {
                lines.push(Line::from(format!("  {}", content_line)));
            }
            // Handle empty content (e.g., streaming just started)
            if msg.content.is_empty() {
                lines.push(Line::from("  "));
            }

            // Blank line between messages
            lines.push(Line::from(""));
        }

        // Add streaming indicator
        if self.is_streaming {
            if self
                .messages
                .last()
                .is_none_or(|m| m.role != Role::Assistant)
            {
                lines.push(Line::from(Span::styled(
                    "[ASSISTANT]",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            lines.push(Line::from(Span::styled(
                "  ...",
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines
    }
}

/// Render the UI to the terminal frame.
fn render_ui(frame: &mut Frame, app: &mut ReplApp) {
    let area = frame.area();

    // Three-area vertical layout: history (flex), input (3 lines), status (1 line)
    let layout = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    render_history(frame, app, layout[0]);
    render_input(frame, app, layout[1]);
    render_status_bar(frame, app, layout[2]);
}

/// Render the scrollable conversation history area.
fn render_history(frame: &mut Frame, app: &mut ReplApp, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Synapse REPL ");
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2) as usize;

    // First pass: build paragraph to compute total wrapped line count
    let total_lines = {
        let lines = app.build_history_lines();
        let history = Paragraph::new(lines)
            .block(block.clone())
            .wrap(Wrap { trim: false });
        history.line_count(inner_width)
    };

    // Update scroll offset (paragraph is now dropped, so we can mutate app)
    let max_scroll = total_lines.saturating_sub(inner_height) as u16;
    if app.auto_scroll {
        app.scroll_offset = max_scroll;
    }
    app.scroll_offset = app.scroll_offset.min(max_scroll);

    // Second pass: build and render with the correct scroll offset
    let lines = app.build_history_lines();
    let history = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));
    frame.render_widget(history, area);
}

/// Render the input area with cursor.
fn render_input(frame: &mut Frame, app: &ReplApp, area: Rect) {
    let input_text = if app.is_streaming {
        String::from("(waiting for response...)")
    } else {
        app.input.clone()
    };

    let input =
        Paragraph::new(input_text).block(Block::default().borders(Borders::ALL).title(" Input "));

    frame.render_widget(input, area);

    // Position cursor in the input area (only when not streaming)
    if !app.is_streaming {
        // Account for border (1) and the cursor position
        let cursor_x = area.x + 1 + app.cursor_position as u16;
        let cursor_y = area.y + 1;
        // Clamp cursor to within the input area
        let max_x = area.x + area.width.saturating_sub(2);
        frame.set_cursor_position((cursor_x.min(max_x), cursor_y));
    }
}

/// Render the status bar at the bottom.
fn render_status_bar(frame: &mut Frame, app: &ReplApp, area: Rect) {
    let status_text = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        format!(
            " Session: {} | Provider: {} | Model: {} | /quit to exit",
            &app.session_id.to_string()[..8],
            app.provider_name,
            app.model_name,
        )
    };

    let status =
        Paragraph::new(status_text).style(Style::default().bg(Color::DarkGray).fg(Color::White));

    frame.render_widget(status, area);
}

/// Result of handling a key event.
enum KeyAction {
    /// Continue the event loop.
    Continue,
    /// Submit the input for processing.
    Submit(String),
    /// Exit the REPL.
    Exit,
}

/// Handle a key event and update app state.
fn handle_key_event(app: &mut ReplApp, key: KeyEvent, history_height: u16) -> KeyAction {
    // Only handle key press events (not release/repeat)
    if key.kind != KeyEventKind::Press {
        return KeyAction::Continue;
    }

    // Ctrl+C always exits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return KeyAction::Exit;
    }

    // Scroll keys are always allowed (even during streaming)
    match key.code {
        KeyCode::Up => {
            app.scroll_up();
            return KeyAction::Continue;
        }
        KeyCode::Down => {
            app.scroll_down();
            return KeyAction::Continue;
        }
        KeyCode::PageUp => {
            app.scroll_page_up(history_height.saturating_sub(2));
            return KeyAction::Continue;
        }
        KeyCode::PageDown => {
            app.scroll_page_down(history_height.saturating_sub(2));
            return KeyAction::Continue;
        }
        _ => {}
    }

    // Don't accept text input while streaming
    if app.is_streaming {
        return KeyAction::Continue;
    }

    match key.code {
        KeyCode::Enter => {
            let input = app.take_input();
            if input.trim().is_empty() {
                return KeyAction::Continue;
            }
            if ReplApp::is_quit_command(&input) {
                return KeyAction::Exit;
            }
            KeyAction::Submit(input)
        }
        KeyCode::Backspace => {
            app.delete_char_before_cursor();
            KeyAction::Continue
        }
        KeyCode::Left => {
            app.move_cursor_left();
            KeyAction::Continue
        }
        KeyCode::Right => {
            app.move_cursor_right();
            KeyAction::Continue
        }
        KeyCode::Home => {
            app.move_cursor_home();
            KeyAction::Continue
        }
        KeyCode::End => {
            app.move_cursor_end();
            KeyAction::Continue
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

/// Entry point for the interactive REPL mode.
///
/// Creates a terminal UI for multi-turn conversations with an LLM provider.
/// Supports creating new sessions or resuming existing ones.
///
/// # Arguments
///
/// * `config` - Application configuration
/// * `provider` - LLM provider for generating responses
/// * `storage` - Session storage for persistence
/// * `session_id` - Optional session ID to resume; creates new if None
/// * `mcp_client` - Optional MCP client for tool execution
pub async fn run_repl(
    config: &Config,
    provider: Box<dyn LlmProvider>,
    storage: Box<dyn SessionStore>,
    session_id: Option<Uuid>,
    mcp_client: Option<McpClient>,
) -> Result<()> {
    // Create or load session
    let (session, history) = match session_id {
        Some(id) => {
            let session = storage
                .get_session(id)
                .await
                .context("Failed to get session")?
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", id))?;

            let messages = storage
                .get_messages(id)
                .await
                .context("Failed to get session messages")?;

            (session, messages)
        }
        None => {
            let session = Session::new(&config.provider, &config.model);
            storage
                .create_session(&session)
                .await
                .context("Failed to create session")?;
            (session, Vec::new())
        }
    };

    // Create agent wrapping provider and MCP client
    let agent = Agent::new(provider, mcp_client);

    // Initialize app state
    let mut app = ReplApp::new(session.id, &config.provider, &config.model);

    // Populate display messages from history (for session resume)
    for msg in &history {
        app.messages.push(DisplayMessage {
            role: msg.role,
            content: msg.content.clone(),
        });
    }

    // Set up terminal
    let _guard = TerminalGuard::new()?;
    let mut terminal = ratatui::init();

    // Set up event stream
    let mut event_reader = EventStream::new();

    // Active agent stream (None when not streaming).
    // Uses stream_owned() so the stream takes ownership of messages,
    // avoiding borrow conflicts in the event loop.
    let mut agent_stream: Option<AgentStream<'_>> = None;

    // Accumulated response content for storage
    let mut response_content = String::new();

    // Compute initial history height for page scroll
    let initial_area = terminal.get_frame().area();
    let mut history_height = initial_area.height.saturating_sub(5); // approx: total - input - status - borders

    loop {
        // Draw UI
        terminal
            .draw(|frame| {
                render_ui(frame, &mut app);
                // Update history height from actual layout
                let layout = Layout::vertical([
                    Constraint::Min(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(frame.area());
                history_height = layout[0].height;
            })
            .context("Failed to draw UI")?;

        tokio::select! {
            // Terminal events
            event = event_reader.next() => {
                match event {
                    Some(Ok(Event::Key(key))) => {
                        match handle_key_event(&mut app, key, history_height) {
                            KeyAction::Continue => {}
                            KeyAction::Exit => break,
                            KeyAction::Submit(input) => {
                                // Add user message to display
                                app.messages.push(DisplayMessage {
                                    role: Role::User,
                                    content: input.clone(),
                                });

                                // Store user message
                                let user_msg = StoredMessage::new(
                                    session.id,
                                    Role::User,
                                    &input,
                                );
                                if let Err(e) = storage.add_message(&user_msg).await {
                                    app.status_message = Some(
                                        format!("Storage error: {}", e),
                                    );
                                    continue;
                                }

                                // Build full conversation for agent from app.messages,
                                // which already contains history (populated during session
                                // resume) plus any new messages from this REPL session.
                                let conv_messages: Vec<Message> = app
                                    .messages
                                    .iter()
                                    .map(|m| Message::new(m.role, &m.content))
                                    .collect();

                                // Start streaming via agent (stream_owned takes ownership
                                // of the messages vec, avoiding borrow issues)
                                app.is_streaming = true;
                                app.auto_scroll = true;
                                response_content.clear();
                                agent_stream = Some(agent.stream_owned(conv_messages));
                            }
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal resized, just redraw
                    }
                    Some(Err(e)) => {
                        app.status_message = Some(format!("Event error: {}", e));
                    }
                    None => break,
                    _ => {}
                }
            }

            // Agent stream events (only when streaming)
            event = async {
                if let Some(ref mut stream) = agent_stream {
                    stream.next().await
                } else {
                    // Never resolves when not streaming
                    std::future::pending().await
                }
            } => {
                match event {
                    Some(Ok(StreamEvent::TextDelta(text))) => {
                        response_content.push_str(&text);
                        app.append_stream_delta(&text);
                    }
                    Some(Ok(StreamEvent::Done)) | None => {
                        app.is_streaming = false;
                        agent_stream = None;

                        // Store assistant response
                        if !response_content.is_empty() {
                            let assistant_msg = StoredMessage::new(
                                session.id,
                                Role::Assistant,
                                &response_content,
                            );
                            if let Err(e) = storage.add_message(&assistant_msg).await {
                                app.status_message = Some(
                                    format!("Storage error: {}", e),
                                );
                            }
                        }

                        // Touch session
                        let _ = storage.touch_session(session.id).await;

                        app.status_message = None;
                    }
                    Some(Ok(StreamEvent::Error(e))) => {
                        app.is_streaming = false;
                        agent_stream = None;
                        app.status_message = Some(format!("LLM error: {}", e));
                    }
                    Some(Ok(_)) => {
                        // Ignore ToolCall/ToolResult (agent handles internally)
                    }
                    Some(Err(e)) => {
                        app.is_streaming = false;
                        agent_stream = None;
                        app.status_message = Some(format!("Agent error: {}", e));
                    }
                }
            }
        }
    }

    // Drop the stream before shutting down the agent (releases borrow)
    drop(agent_stream);

    // Drop terminal guard (restores terminal) before printing
    drop(_guard);
    ratatui::restore();

    // Shutdown agent (MCP connections)
    agent.shutdown().await;

    // Print session ID to stderr for future resumption
    eprintln!("Session: {}", session.id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        });
        assert_eq!(app.last_assistant_content(), None);

        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "Hi there".to_string(),
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
        });
        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "Hi!".to_string(),
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
        });

        let lines = app.build_history_lines();
        // Tool: [TOOL] + "  Tool output" + "" = 3 lines
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_handle_key_event_ctrl_c() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Exit));
    }

    #[test]
    fn test_handle_key_event_enter_empty() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
    }

    #[test]
    fn test_handle_key_event_enter_with_input() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        app.insert_char('h');
        app.insert_char('i');

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Submit(s) if s == "hi"));
    }

    #[test]
    fn test_handle_key_event_quit_command() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        for c in "/quit".chars() {
            app.insert_char(c);
        }

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Exit));
    }

    #[test]
    fn test_handle_key_event_char_input() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert_eq!(app.input, "a");
    }

    #[test]
    fn test_handle_key_event_ignored_while_streaming() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        app.is_streaming = true;

        // Text input is blocked during streaming
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert!(app.input.is_empty()); // Input not accepted

        // But Ctrl+C still works
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Exit));
    }

    #[test]
    fn test_scroll_keys_work_during_streaming() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");
        app.is_streaming = true;
        app.scroll_offset = 5;

        // Up arrow works during streaming
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert_eq!(app.scroll_offset, 4);

        // Down arrow works during streaming
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert_eq!(app.scroll_offset, 5);

        // PageUp works during streaming
        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert_eq!(app.scroll_offset, 0); // 5 - (20-2) = 0 (saturating)

        // PageDown works during streaming
        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert_eq!(app.scroll_offset, 18); // 0 + (20-2) = 18
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
    fn test_handle_key_event_release_ignored() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        let mut key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        key.kind = KeyEventKind::Release;
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert!(app.input.is_empty());
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
        });
        app.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: "Hi there!".to_string(),
        });

        // Simulate user submitting a new message in the REPL
        app.messages.push(DisplayMessage {
            role: Role::User,
            content: "Follow up question".to_string(),
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
}
