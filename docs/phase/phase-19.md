# Phase 19: Telegram Command Fixes & Interactive Keyboards (SY-20)

**Goal:** Fix slash commands that fall through to the LLM instead of being handled by the dispatcher, and add inline keyboard UX for `/switch` and `/delete` when used without an argument.

## Tasks

- [x] 19.1 Change `Switch(usize)` → `Switch(String)` and `Delete(usize)` → `Delete(String)` in `Command` enum; add `parse_session_arg()` helper; update match arms
- [x] 19.2 Add `Start` variant to `Command` enum with a welcome message handler
- [x] 19.3 Add defensive command guard in `handlers::handle_message` — if text starts with a known command but `filter_command` missed it, reply with a hint instead of forwarding to the LLM
- [x] 19.4 Add `build_session_keyboard()`, `cmd_switch_keyboard()`, and `cmd_delete_keyboard()` in `commands.rs` — `InlineKeyboardMarkup` with one button per session, callback data `"action:index"`
- [x] 19.5 Add `handle_callback()` in `commands.rs` for `CallbackQuery` updates; refactor switch/delete core logic into `do_switch()`/`do_delete()` returning `String` so both slash-command and callback paths share the same logic
- [x] 19.6 Restructure dispatcher in `main.rs`: wrap in `dptree::entry()` with two branches — `Update::filter_message()` (existing) and `Update::filter_callback_query()` → `handle_callback`
- [x] 19.7 Add unit tests for `parse_session_arg`, `build_session_keyboard` callback data format, and defensive guard logic

## Acceptance Criteria

**Test:** `/help` returns the command list (not an LLM response); `/switch` with no argument shows a clickable session list; tapping a button switches/deletes the session and removes the keyboard; `/start` shows a welcome message; unknown commands are not forwarded to the LLM.

## Dependencies

- Phase 18 complete

## Implementation Notes

### Root Cause of Command Fall-Through

`Switch(usize)` and `Delete(usize)` in the `Command` enum cause `BotCommands::parse` to fail when no argument is provided (empty string → `usize::from_str("")` fails). Since `filter_command` uses `.ok()` to convert parse errors to `None`, the entire branch is silently rejected and the message falls through to `handle_message`, which forwards it to the LLM.

The fix is to change both variants to `Switch(String)` / `Delete(String)` so teloxide accepts them with or without an argument. The `parse_session_arg()` helper then distinguishes empty (→ show keyboard) from numeric (→ execute directly).

### Defensive Guard

Even after fixing the enum types, add a defensive guard early in `handle_message`:

```
if text.starts_with('/') && known_commands.contains(command_name) {
    reply("Use /help ..."); return;
}
```

This prevents any future regressions from silently forwarding commands to the LLM.

### Interactive Keyboard Flow

```
User taps /switch from menu
  → filter_command parses Switch("")
  → handle_command → cmd_switch_keyboard
  → sends message with InlineKeyboardMarkup
    [1. * 2025-01-10 | 5 msgs | Hello...]
    [2.   2025-01-09 | 3 msgs | What is...]
  → user taps button 2
  → CallbackQuery arrives with data "switch:2"
  → handle_callback → do_switch(2, ...) → edits message to "Switched to session 2."
```

### Callback Data Format

- `"switch:N"` — switch to 1-based session index N
- `"delete:N"` — delete 1-based session index N

`handle_callback` must call `bot.answer_callback_query(q.id)` immediately (before any DB calls) to dismiss the loading spinner in Telegram.

After executing the action, edit the original keyboard message to replace the buttons with a plain result string (`bot.edit_message_text`). This removes the keyboard and prevents accidental double-execution.

### Dispatcher Restructure

Current (message-only):
```rust
let handler = Update::filter_message()
    .branch(filter_command → handle_command)
    .branch(handle_message);
```

New (messages + callback queries):
```rust
let handler = dptree::entry()
    .branch(Update::filter_message()
        .branch(filter_command → handle_command)
        .branch(handle_message))
    .branch(Update::filter_callback_query()
        .endpoint(handle_callback));
```

### No Delete Confirmation

`/delete` (both slash and keyboard) deletes immediately. Sessions are cheap to recreate with `/new`. A confirmation dialog would add a third callback state (`confirm_delete:N`) and is not worth the added complexity.
