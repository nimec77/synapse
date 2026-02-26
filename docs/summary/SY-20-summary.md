# SY-20 Summary: Improve /history Command

**Ticket:** SY-20
**Status:** COMPLETE
**Branch:** feature/sy-20-phase20
**Date:** 2026-02-26

---

## Overview

SY-20 replaces the unbounded full-dump `/history` command in the Synapse Telegram bot with a
compact "last 10 messages" view. Before this change, `/history` iterated over every stored message
in the session — including `System` and `Tool` role messages — with no limit and no truncation.
This produced noisy, hard-to-read output, especially for sessions with MCP tool calls where
internal messages cluttered the view and long assistant responses flooded the chat.

The fix extracts two pure synchronous helper functions (`truncate_content`, `format_history`) from
the async handler, refactors `cmd_history` to delegate to `format_history`, updates the
`Command::History` description in `/help`, and adds 11 unit tests covering all truncation,
filtering, and edge-case scenarios. All changes are confined to a single file:
`synapse-telegram/src/commands.rs`.

---

## What Was Done

### Task 1 — `truncate_content` helper function

A pure function was extracted to perform Unicode-safe character truncation:

```rust
fn truncate_content(content: &str, max_chars: usize) -> String {
    let char_count = content.chars().count();
    if char_count <= max_chars {
        content.to_string()
    } else {
        let truncated: String = content.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}
```

Truncation uses `.chars().take(max_chars)` (Unicode scalar values, not bytes), consistent with the
existing `cmd_list` preview pattern. The ellipsis is three ASCII dots `...`, not the Unicode
character `\u{2026}`.

### Task 2 — `format_history` helper function

A pure function was extracted to filter, limit, and format stored messages:

```rust
fn format_history(messages: &[StoredMessage]) -> String {
    let filtered: Vec<&StoredMessage> = messages
        .iter()
        .filter(|m| matches!(m.role, Role::User | Role::Assistant))
        .collect();
    let skip = filtered.len().saturating_sub(10);
    let recent = &filtered[skip..];

    let mut output = String::new();
    for m in recent {
        let role_label = match m.role {
            Role::User => "You",
            Role::Assistant => "Assistant",
            _ => unreachable!(),
        };
        let timestamp = chrono::Utc
            .from_utc_datetime(&m.timestamp.naive_utc())
            .format("%Y-%m-%d %H:%M")
            .to_string();
        let content = truncate_content(&m.content, 150);
        output.push_str(&format!("[{}] {}\n{}\n\n", role_label, timestamp, content));
    }
    output
}
```

The function:
1. Filters the message slice to retain only `Role::User` and `Role::Assistant` messages, discarding
   `System` and `Tool` messages.
2. Computes `skip = filtered.len().saturating_sub(10)` and slices `&filtered[skip..]` to select
   the chronologically last 10 messages.
3. Formats each message with the role label (`"You"` or `"Assistant"`), a UTC timestamp in
   `%Y-%m-%d %H:%M` format, and the content after `truncate_content` with a 150-character limit.
4. Returns the accumulated output string, or an empty string if no messages pass the role filter.

### Task 3 — Refactored `cmd_history`

The `if messages.is_empty()` check and the inline `for m in &messages` formatting loop were
replaced with a delegation to `format_history`:

```rust
let messages = storage.get_messages(session_id).await.unwrap_or_default();
let output = format_history(&messages);

if output.is_empty() {
    bot.send_message(msg.chat.id, "No messages in current session.")
        .await?;
    return Ok(());
}

for chunk in chunk_message(output.trim()) {
    bot.send_message(msg.chat.id, chunk).await?;
}
```

The session-lookup logic and the `chunk_message` sending loop remain unchanged. This also fixes the
case where a session contains only `System` or `Tool` messages: previously the handler would
display them; now `format_history` returns `""` and the user receives "No messages in current
session." instead.

### Task 4 — Updated `Command::History` description

The `Command::History` variant annotation was changed from:

```rust
#[command(description = "Show conversation history")]
History,
```

to:

```rust
#[command(description = "Show recent messages")]
History,
```

This is the string Telegram derives for the `/help` command output.

### Tasks 5 and 6 — Unit tests

Eleven new unit tests were added in the `#[cfg(test)]` module:

**Tests for `truncate_content` (5):**

| Test | Coverage |
|------|----------|
| `test_truncate_content_short` | 10-char content returned unchanged, no `...` |
| `test_truncate_content_exact_limit` | Exactly 150-char content returned unchanged, no `...` |
| `test_truncate_content_over_limit` | 151-char content returns 150 chars + `...` (153 total) |
| `test_truncate_content_long` | 500-char content returns 153 chars ending with `...` |
| `test_truncate_content_empty` | Empty string returns empty string, no `...` |

**Tests for `format_history` (6):**

| Test | Coverage |
|------|----------|
| `test_format_history_filters_system_and_tool` | All four roles input; output contains only `"You"` and `"Assistant"` |
| `test_format_history_keeps_user_and_assistant` | 3 User + 3 Assistant messages; all 6 appear in output |
| `test_format_history_last_10_limit` | 15 messages; only the last 10 appear (first 5 absent) |
| `test_format_history_fewer_than_10` | 3 messages; all 3 appear |
| `test_format_history_empty` | System and Tool messages only; returns empty string |
| `test_format_history_truncates_long_content` | 200-char content; output contains `...` and is at most 153 chars |

A `make_stored_message(role: Role, content: &str) -> StoredMessage` helper was added to the test
module, analogous to the existing `make_session` helper.

### Task 7 — Documentation verification (no file changes)

All three documentation files were verified to already match the target state before implementation:

- `CLAUDE.md` — Workspace Crates table described `/history` as "(last 10 messages, truncated to
  150 chars)" (pre-populated ahead of this ticket).
- `README.md` — Commands table row for `/history` already read "Show recent messages from the
  current session".
- `CHANGELOG.md` — `[Unreleased]` section already contained the entry for this change.

No edits were required to these files during implementation.

---

## Key Design Decisions

**Extract to pure helpers, not inline in async handler** — Embedding the filtering and truncation
logic directly in `cmd_history` would make it impossible to unit-test without a live Telegram bot
and database. Extracting `truncate_content` and `format_history` as pure synchronous functions
allows comprehensive unit testing with in-process data only.

**`.chars().take(150)` for truncation** — The PRD requires character-based (not byte-based)
truncation. `.chars()` iterates Unicode scalar values, which ensures no multi-byte character is
split mid-sequence. This is consistent with the existing `cmd_list` preview pattern that uses the
same approach.

**`saturating_sub(10)` for the last-10 limit** — `filtered.len().saturating_sub(10)` computes the
skip count and returns `0` when the list has fewer than 10 messages, avoiding underflow. Slicing
`&filtered[skip..]` then naturally includes all elements when `skip == 0`.

**`unreachable!()` in the role match arm** — The `_ => unreachable!()` arm inside `format_history`
is safe because the `filter` step immediately above it guarantees that only `User` and `Assistant`
messages reach the match. This documents the invariant explicitly and would surface immediately in
tests if the filter were ever changed.

**Empty string as the "no output" sentinel** — `format_history` returns `""` when no messages pass
the role filter. The `cmd_history` caller checks `output.is_empty()` and sends "No messages in
current session." This unifies three previously separate cases (empty session, session with only
System/Tool messages) into a single code path.

**Plain text output, no `ParseMode::Html`** — The `/history` reply uses plain text, matching the
existing convention for all other slash command replies (`/list`, `/new`, etc.). The HTML parse mode
is only used for regular message responses in `handlers.rs`.

---

## Data Flow

```
User sends "/history"
  |
  v
handle_command -> cmd_history(bot, msg, storage, chat_map)
  |
  v
Read active session ID from chat_map (in-memory, no DB)
  |
  +--> None: "No active session. Send a message or use /new to start one."
  |
  v (Some(session_id))
storage.get_messages(session_id) -> Vec<StoredMessage>
  (all messages in chronological order: User, Assistant, System, Tool)
  |
  v
format_history(&messages):
  1. Filter: keep only Role::User and Role::Assistant
  2. Limit: skip = saturating_sub(10), slice last 10
  3. Format each: truncate_content(content, 150), "[role] timestamp\ncontent\n\n"
  4. Return concatenated string (empty if no messages pass filter)
  |
  +--> output.is_empty(): "No messages in current session."
  |
  v (non-empty output)
chunk_message(output.trim()) -> send each chunk as plain-text message
```

---

## Files Changed

| File | Change |
|------|--------|
| `synapse-telegram/src/commands.rs` | `truncate_content` and `format_history` helpers added; `cmd_history` refactored to delegate to `format_history`; `Command::History` description changed to `"Show recent messages"`; 11 new unit tests; `make_stored_message` test helper added |

## Files NOT Changed

- `synapse-core/` — All changes confined to `synapse-telegram`; hexagonal architecture preserved
- `synapse-cli/` — CLI is entirely unaffected
- `synapse-telegram/src/handlers.rs` — No changes needed
- `synapse-telegram/src/main.rs` — No changes needed
- `Cargo.toml` / `Cargo.lock` — No new dependencies; uses existing `chrono` and `synapse_core` types

---

## Risk Notes

Three low-severity risks from the QA report, all acknowledged and accepted:

- **R-1 (Full retrieval before filtering)**: `storage.get_messages()` returns all messages for the
  session before the 10-message cap is applied in memory. For typical Telegram usage patterns where
  sessions rarely exceed hundreds of messages, this is not a performance concern. Pagination can be
  added in a future ticket if needed.

- **R-2 (Unicode display width)**: `.chars().take(150)` counts Unicode scalar values, not grapheme
  clusters or display columns. CJK characters or emoji may appear visually shorter than 150
  "characters". This is consistent with the existing `cmd_list` preview pattern and was accepted in
  the PRD as a deliberate constraint.

- **R-3 (Timestamp always UTC)**: `chrono::Utc.from_utc_datetime()` always displays times in UTC.
  Users in non-UTC timezones see UTC times. This is a pre-existing behavior shared with the prior
  `/history` implementation and is out of scope for SY-20.

None are blocking.

---

## Metrics

| Metric | Target | Achieved |
|--------|--------|----------|
| `cargo fmt --check` | Passes | Passes |
| `cargo clippy -- -D warnings` | Zero warnings | Zero warnings |
| `cargo test -p synapse-telegram` | All green | 79 passed, 0 failed |
| Role filtering | User/Assistant only | `matches!(m.role, Role::User \| Role::Assistant)` filter |
| Last-10 limit | Correct 10 selected | `saturating_sub(10)` skip applied |
| Truncation at 150 chars | With `...` suffix | `.chars().take(150)` + `format!("{}...", truncated)` |
| Exact-150-char content | No ellipsis | `char_count <= max_chars` branch |
| Command description | "Show recent messages" | `#[command(description = "Show recent messages")]` |
| Unit tests added | 11 | 11 (5 for `truncate_content`, 6 for `format_history`) |
| No changes to `synapse-core` | Required | Confirmed |
| No new crate dependencies | Required | Confirmed |
