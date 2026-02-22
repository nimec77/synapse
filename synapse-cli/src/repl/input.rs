//! Key event handling and input bindings for the REPL.
//!
//! Defines [`KeyAction`] and [`handle_key_event`] which translate crossterm key
//! events into discrete actions consumed by the `run_repl` event loop.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::app::ReplApp;

/// Result of handling a key event.
pub(super) enum KeyAction {
    /// Continue the event loop.
    Continue,
    /// Submit the input for processing.
    Submit(String),
    /// Exit the REPL.
    Exit,
}

/// Handle a key event and update app state.
pub(super) fn handle_key_event(app: &mut ReplApp, key: KeyEvent, history_height: u16) -> KeyAction {
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

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

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
    fn test_handle_key_event_release_ignored() {
        let id = Uuid::new_v4();
        let mut app = ReplApp::new(id, "test", "test");

        let mut key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        key.kind = KeyEventKind::Release;
        let action = handle_key_event(&mut app, key, 20);
        assert!(matches!(action, KeyAction::Continue));
        assert!(app.input.is_empty());
    }
}
