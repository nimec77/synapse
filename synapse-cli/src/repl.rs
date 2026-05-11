//! Interactive REPL mode for the Synapse CLI.
//!
//! Provides a terminal-based chat interface using `ratatui` and `crossterm`,
//! supporting multi-turn conversations with streaming LLM responses and
//! session persistence.
//!
//! # Module layout
//!
//! - `app`   — [`ReplApp`] struct, state fields, transitions, and helpers
//! - `render` — `render_ui` and layout/draw functions
//! - `input`  — `handle_key_event` and key bindings

mod app;
mod input;
mod render;

use std::io;

use anyhow::{Context, Result};
use crossterm::{
    event::{Event, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::layout::{Constraint, Layout};

use app::{DisplayMessage, ReplApp};
use input::{KeyAction, handle_key_event};
use render::{REPL_INPUT_HEIGHT, REPL_MIN_HISTORY_HEIGHT, REPL_STATUS_HEIGHT, render_ui};
use synapse_core::{
    Agent, AgentError, Config, McpClient, Message, Role, Session, SessionStore, StoredMessage,
    StreamEvent,
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

/// Entry point for the interactive REPL mode.
///
/// Creates a terminal UI for multi-turn conversations with an LLM provider.
/// The session and history must be resolved by the caller (via
/// `session::load_or_create_session`) before invoking this function.
///
/// # Arguments
///
/// * `config` - Application configuration (provider, model, system prompt, etc.)
/// * `storage` - Session storage for persistence
/// * `session` - The session to use (already created or loaded)
/// * `history` - Existing message history for the session
/// * `mcp_client` - Optional MCP client for tool execution
pub async fn run_repl(
    config: &Config,
    storage: Box<dyn SessionStore>,
    session: Session,
    history: Vec<StoredMessage>,
    mcp_client: Option<McpClient>,
) -> Result<()> {
    // Create agent from config and MCP client
    let agent = Agent::from_config(config, mcp_client).context("Failed to create agent")?;

    // Initialize app state
    let mut app = ReplApp::new(session.id, &config.provider, &config.model);
    // Apply show_reasoning from config (default true when [cli] section is absent)
    app.show_reasoning = config.cli.as_ref().is_none_or(|c| c.show_reasoning);

    // Populate display messages from history (for session resume).
    // reasoning_content is preserved so session-resumed conversations honour
    // the DeepSeek tool-call history rule when building conv_messages.
    for msg in &history {
        app.messages.push(DisplayMessage {
            role: msg.role,
            content: msg.content.clone(),
            reasoning_content: msg.reasoning_content.clone(),
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
    // Accumulated reasoning content for storage (persisted regardless of show_reasoning flag)
    let mut reasoning_content = String::new();

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
                    Constraint::Min(REPL_MIN_HISTORY_HEIGHT),
                    Constraint::Length(REPL_INPUT_HEIGHT),
                    Constraint::Length(REPL_STATUS_HEIGHT),
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
                                    reasoning_content: None,
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
                                // reasoning_content is carried through to honour the
                                // DeepSeek tool-call history rule on session resume.
                                let conv_messages: Vec<Message> = app
                                    .messages
                                    .iter()
                                    .map(|m| {
                                        let mut msg = Message::new(m.role, &m.content);
                                        if let Some(ref r) = m.reasoning_content {
                                            msg = msg.with_reasoning(r.clone());
                                        }
                                        msg
                                    })
                                    .collect();

                                // Start streaming via agent (stream_owned takes ownership
                                // of the messages vec, avoiding borrow issues)
                                app.is_streaming = true;
                                app.auto_scroll = true;
                                response_content.clear();
                                reasoning_content.clear();
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
                    Some(Ok(StreamEvent::ReasoningDelta(reasoning))) => {
                        // Accumulate reasoning for storage (always).
                        reasoning_content.push_str(&reasoning);
                    }
                    Some(Ok(StreamEvent::Done)) | None => {
                        app.is_streaming = false;
                        agent_stream = None;

                        // Attach accumulated reasoning to the last assistant DisplayMessage
                        // so build_history_lines can render it when show_reasoning is true.
                        if !reasoning_content.is_empty()
                            && let Some(last) = app.messages.last_mut()
                            && last.role == Role::Assistant
                        {
                            last.reasoning_content = Some(reasoning_content.clone());
                        }

                        // Store assistant response — always include reasoning_content so
                        // multi-turn tool conversations honour the DeepSeek history rule.
                        if !response_content.is_empty() {
                            let mut assistant_msg = StoredMessage::new(
                                session.id,
                                Role::Assistant,
                                &response_content,
                            );
                            if !reasoning_content.is_empty() {
                                assistant_msg = assistant_msg
                                    .with_reasoning(reasoning_content.clone());
                            }
                            if let Err(e) = storage.add_message(&assistant_msg).await {
                                app.status_message = Some(
                                    format!("Storage error: {}", e),
                                );
                            }
                        }

                        // Reset accumulators for next turn
                        reasoning_content.clear();

                        // Touch session
                        let _ = storage.touch_session(session.id).await;

                        app.status_message = None;
                    }
                    Some(Err(e)) => {
                        app.is_streaming = false;
                        agent_stream = None;
                        app.status_message = Some(format!("Agent error: {}", e));
                    }
                    Some(Ok(_)) => {
                        // Unknown future StreamEvent variant — ignore.
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

    // Log session ID for future resumption
    tracing::info!("Session: {}", session.id);

    Ok(())
}
