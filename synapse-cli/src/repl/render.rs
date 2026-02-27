//! TUI rendering functions for the REPL.
//!
//! Provides [`render_ui`] and its helpers that draw the conversation history,
//! input box, and status bar using `ratatui`.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::app::{DisplayMessage, ReplApp};
use synapse_core::Role;

/// Minimum height of the scrollable history area (in terminal rows).
pub(super) const REPL_MIN_HISTORY_HEIGHT: u16 = 3;

/// Height of the input box area (in terminal rows, including border).
pub(super) const REPL_INPUT_HEIGHT: u16 = 3;

/// Height of the status bar at the bottom (in terminal rows).
pub(super) const REPL_STATUS_HEIGHT: u16 = 1;

/// Render the UI to the terminal frame.
pub(super) fn render_ui(frame: &mut Frame, app: &mut ReplApp) {
    let area = frame.area();

    // Three-area vertical layout: history (flex), input, status
    let layout = Layout::vertical([
        Constraint::Min(REPL_MIN_HISTORY_HEIGHT),
        Constraint::Length(REPL_INPUT_HEIGHT),
        Constraint::Length(REPL_STATUS_HEIGHT),
    ])
    .split(area);

    render_history(frame, app, layout[0]);
    render_input(frame, app, layout[1]);
    render_status_bar(frame, app, layout[2]);
}

/// Render the scrollable conversation history area.
pub(super) fn render_history(frame: &mut Frame, app: &mut ReplApp, area: Rect) {
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
pub(super) fn render_input(frame: &mut Frame, app: &ReplApp, area: Rect) {
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
pub(super) fn render_status_bar(frame: &mut Frame, app: &ReplApp, area: Rect) {
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

/// Build conversation lines for rendering.
///
/// Separated from [`ReplApp`] so it can be unit-tested without depending on a
/// full app instance. Used by [`ReplApp::build_history_lines`].
pub(super) fn build_history_lines<'a>(
    messages: &'a [DisplayMessage],
    is_streaming: bool,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();
    for msg in messages {
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
    if is_streaming {
        if messages.last().is_none_or(|m| m.role != Role::Assistant) {
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
