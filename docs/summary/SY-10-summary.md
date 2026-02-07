# SY-10 Summary: Phase 9 - CLI REPL

**Status:** COMPLETE
**Date:** 2026-02-07

---

## Overview

SY-10 implements an interactive REPL (Read-Eval-Print Loop) mode for the Synapse CLI using `ratatui` and `crossterm`. Users can now have multi-turn conversations with LLMs in a full terminal user interface, complete with streaming responses, session persistence, and the ability to resume previous sessions.

The key change is that running `synapse --repl` enters an interactive TUI where users can chat with their configured LLM provider across multiple turns, with every message automatically saved to the session store. Users can later resume any REPL session with `synapse --repl --session <uuid>`.

---

## What Was Built

### New Components

1. **REPL Module** (`synapse-cli/src/repl.rs` -- 1035 lines)
   - `TerminalGuard` struct with `Drop` implementation for terminal state safety (raw mode, alternate screen)
   - `ReplApp` struct holding all REPL state: messages, input buffer, cursor position, scroll offset, streaming state, session ID, status message
   - `DisplayMessage` struct for rendering conversation history with role labels
   - Three-area vertical layout: scrollable conversation history, input area with cursor, status bar with session/provider info
   - Async event loop via `tokio::select!` multiplexing crossterm `EventStream` and LLM stream events
   - Full input editing: character insert, backspace, cursor movement (left/right/home/end)
   - History scrolling: up/down (line), page up/page down (page)
   - Command parsing: `/quit` for clean exit
   - Session persistence: user and assistant messages stored on each exchange
   - Session resume: loads existing session history into display and LLM context
   - `run_repl()` public entry point accepting config, provider, storage, and optional session ID

2. **CLI Flag** (`--repl` / `-r` in `synapse-cli/src/main.rs`)
   - Routes to `repl::run_repl()` when set
   - Combines with `--session <id>` for session resume
   - Creates storage and provider before entering REPL (errors caught before terminal mode)
   - Runs auto-cleanup on startup (matching one-shot mode behavior)

### Modified Components

1. **CLI Entry Point** (`synapse-cli/src/main.rs`)
   - Added `mod repl;` module declaration
   - Added `--repl` / `-r` flag to `Args` struct
   - Added REPL mode routing block between subcommand check and message handling
   - 4 new unit tests for `--repl` flag parsing

2. **CLI Dependencies** (`synapse-cli/Cargo.toml`)
   - Added `ratatui = "0.30.0"`
   - Added `crossterm = { version = "0.29.0", features = ["event-stream"] }`

---

## Key Decisions

### 1. ratatui + crossterm for TUI

**Decision:** Use `ratatui` with `crossterm` backend as specified in the phase description and CLAUDE.md.

**Rationale:** `ratatui` provides a structured layout system (constraints, blocks, paragraphs) and crossterm provides cross-platform terminal handling. The `event-stream` feature on crossterm enables async-compatible key event reading via `EventStream`, which integrates with the `tokio::select!` event loop.

### 2. TerminalGuard with Drop for Safety

**Decision:** Wrap terminal setup/teardown in a `TerminalGuard` struct that restores terminal state in its `Drop` implementation.

**Rationale:** Terminal state corruption is the highest-impact risk for a TUI application. By using a RAII guard, terminal restoration happens on normal exit, error propagation, and panic unwind -- covering all exit paths. This avoids the need for manual cleanup in every error branch.

### 3. tokio::select! Event Loop

**Decision:** Use `tokio::select!` to multiplex terminal key events and LLM streaming events in a single loop.

**Rationale:** This follows the established pattern from the one-shot streaming implementation (SY-8). It allows the REPL to remain responsive to user input (including Ctrl+C) while simultaneously rendering streaming tokens. The `if app.is_streaming` guard on the stream branch ensures clean state transitions.

### 4. app.messages as Single Source of Truth

**Decision:** Use `app.messages` as the sole source of conversation history for both display and LLM context (RF1 fix).

**Rationale:** The initial implementation maintained separate `history` (from storage) and `app.messages` vectors, leading to duplicate messages when resuming sessions. By loading history into `app.messages` at startup and building the LLM conversation vector exclusively from `app.messages`, the duplication was eliminated.

### 5. Input Blocked During Streaming

**Decision:** Silently ignore character input while the LLM is streaming a response.

**Rationale:** Accepting input during streaming would create confusing UX (where does the cursor go? what happens when the user submits?). Ctrl+C remains active during streaming for emergency exit. This is the simplest correct behavior for an MVP.

### 6. Session ID Printed to stderr on Exit

**Decision:** Print the session ID to stderr (not stdout) after REPL exit.

**Rationale:** Stderr is the correct channel for informational messages in CLI tools. This allows the session ID to be captured even when stdout is redirected. Users can copy the ID to resume later with `--repl --session <id>`.

### 7. No Changes to synapse-core

**Decision:** All REPL code lives in `synapse-cli` with zero modifications to `synapse-core`.

**Rationale:** Following hexagonal architecture, the REPL is a user interface concern. It consumes the existing `LlmProvider` and `SessionStore` traits without requiring any new ports or adapters in the core library.

---

## Data Flows

### New REPL Session
```
User: synapse --repl
  |
CLI: parse args (args.repl == true, args.session == None)
  |
CLI: create_storage() + cleanup()
  |
CLI: create_provider()
  |
CLI: repl::run_repl(config, provider, storage, None)
  |
REPL: Session::new(provider, model)
  |
REPL: storage.create_session(&session)
  |
REPL: TerminalGuard::new() (raw mode, alternate screen)
  |
REPL: Enter tokio::select! event loop
  |
User types message, presses Enter
  |
REPL: storage.add_message(user_msg)
  |
REPL: provider.stream(&messages) -> streaming starts
  |
REPL: TextDelta tokens render to history area
  |
REPL: StreamEvent::Done -> storage.add_message(assistant_msg)
  |
REPL: Await next input...
  |
User presses Ctrl+C or types /quit
  |
REPL: Drop TerminalGuard (restore terminal)
  |
REPL: Print session ID to stderr
```

### Resume Session
```
User: synapse --repl --session <uuid>
  |
CLI: repl::run_repl(config, provider, storage, Some(uuid))
  |
REPL: storage.get_session(uuid) -> session
  |
REPL: storage.get_messages(uuid) -> history
  |
REPL: Populate app.messages with DisplayMessage entries
  |
REPL: Enter event loop (history displayed in TUI)
  |
User types follow-up message
  |
REPL: Build conversation from app.messages (history + new)
  |
REPL: provider.stream(&messages) (full context)
  |
[continues as normal...]
```

---

## Testing

### REPL Unit Tests (25 tests in `repl.rs`)

| Test | Coverage |
|------|----------|
| `test_repl_app_new` | ReplApp initialization, default field values |
| `test_insert_char` | Character insertion, cursor advancement |
| `test_insert_char_at_position` | Mid-string insertion via cursor movement |
| `test_delete_char_before_cursor` | Backspace at end of input |
| `test_delete_char_at_beginning` | Backspace at position 0 (no-op) |
| `test_cursor_movement` | Left, Right, Home, End cursor operations |
| `test_cursor_bounds` | Cursor stays within valid range |
| `test_take_input` | Input extraction and buffer reset |
| `test_is_quit_command` | /quit detection with whitespace handling |
| `test_scroll` | Up/Down scroll with saturating arithmetic |
| `test_scroll_page` | PageUp/PageDown scroll by page size |
| `test_append_stream_delta` | First delta creates assistant message |
| `test_append_stream_delta_after_user_message` | Delta after user creates new assistant message |
| `test_last_assistant_content` | Content extraction for storage |
| `test_build_history_lines_empty` | Empty message list renders no lines |
| `test_build_history_lines_with_messages` | Message rendering with role labels |
| `test_handle_key_event_ctrl_c` | Ctrl+C returns Exit action |
| `test_handle_key_event_enter_empty` | Empty input returns Continue |
| `test_handle_key_event_enter_with_input` | Non-empty input returns Submit |
| `test_handle_key_event_quit_command` | /quit returns Exit |
| `test_handle_key_event_char_input` | Character key inserts into input |
| `test_handle_key_event_ignored_while_streaming` | Input blocked during streaming |
| `test_handle_key_event_release_ignored` | Key release events skipped |
| `test_no_duplicate_messages_on_session_resume` | RF1 fix: no message duplication |
| `test_insert_multibyte_char` | UTF-8 multi-byte character handling |

### CLI Flag Tests (4 new tests in `main.rs`)

| Test | Coverage |
|------|----------|
| `test_args_repl_flag` | `--repl` flag parsing |
| `test_args_repl_short_flag` | `-r` short flag |
| `test_args_repl_with_session` | `--repl --session <id>` combination |
| `test_args_repl_default_false` | `repl` defaults to false |

**Total new tests: 29 (25 REPL + 4 CLI)**
**Total project tests: 130 passed, 0 failed, 1 ignored**

---

## UI Layout

```
+---------------- Synapse REPL ------------------+
| [USER] Hello, how are you?                     |
| [ASSISTANT] I'm doing well, thanks for asking! |
| ...                                            |
| (scrollable conversation history)              |
+------------------------------------------------+
| > user input here_                             |
+------------------------------------------------+
| Session: <uuid> | Provider: deepseek | /quit   |
+------------------------------------------------+
```

Three vertical areas:
1. **History area** (flex): Scrollable paragraph with role-labeled messages (`[USER]`, `[ASSISTANT]`)
2. **Input area** (3 lines): Current user input with visible cursor
3. **Status bar** (1 line): Session ID (truncated), provider, model, `/quit` hint

---

## Usage

### Start Interactive REPL
```bash
# Enter interactive mode (new session)
synapse --repl

# Short form
synapse -r
```

### Resume a Previous Session
```bash
# Resume an existing session in REPL mode
synapse --repl --session 01234567-89ab-cdef-0123-456789abcdef

# Short form
synapse -r -s 01234567-89ab-cdef-0123-456789abcdef
```

### Key Bindings
| Key | Action |
|-----|--------|
| Enter | Submit input |
| Backspace | Delete character before cursor |
| Left/Right | Move cursor |
| Home/End | Jump to start/end of input |
| Up/Down | Scroll history |
| PageUp/PageDown | Scroll history by page |
| Ctrl+C | Exit REPL |

### Commands
| Command | Action |
|---------|--------|
| `/quit` | Exit REPL cleanly |

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-cli/src/repl.rs` | New -- Full REPL implementation (1035 lines) |
| `synapse-cli/src/main.rs` | Modified -- `mod repl;`, `--repl` flag, REPL routing |
| `synapse-cli/Cargo.toml` | Modified -- Added `ratatui` 0.30.0, `crossterm` 0.29.0 |

---

## Module Structure

```
synapse-cli/src/
  main.rs               # mod repl; --repl/-r flag, REPL routing
  repl.rs               # TerminalGuard, ReplApp, DisplayMessage,
                        # run_repl(), event loop, UI rendering, tests
```

No changes to synapse-core module structure.

---

## Known Limitations

1. **No input history**: Up-arrow does not recall previous inputs (scrolls history instead)
2. **No multi-line input**: Each Enter submits; no Shift+Enter for newlines
3. **No Markdown rendering**: Assistant responses displayed as plain text
4. **Wide character cursor misalignment**: East Asian characters (2-cell width) may misalign the cursor position
5. **Unbounded message history**: All messages held in memory; very long sessions may consume significant memory
6. **No panic hook**: Relies on `TerminalGuard::Drop` for terminal restoration; `abort`-on-panic would bypass it
7. **TerminalGuard/ratatui::init() overlap**: Both set up terminal state (harmless but redundant)

---

## Future Work

This implementation enables:
- **Input history** -- Up-arrow to recall previous inputs
- **Multi-line input** -- Shift+Enter for newlines within a message
- **Markdown rendering** -- Rich formatting for assistant responses
- **Wide character support** -- Correct cursor alignment for CJK characters
- **Panic hook** -- Additional terminal restoration safety
- **Message search** -- Search within conversation history
- **Export** -- Save REPL conversation to Markdown/JSON
