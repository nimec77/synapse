# Phase 16: Telegram Markdown Formatting (SY-17)

**Goal:** Render LLM Markdown responses as formatted text in Telegram instead of raw symbols.

## Tasks

- [x] 16.1 Add `pulldown-cmark = { version = "0.13", default-features = false }` to `synapse-telegram/Cargo.toml`
- [x] 16.2 Create `synapse-telegram/src/format.rs` with `escape_html()`, `md_to_telegram_html()`, and `chunk_html()` plus 25 unit tests
- [x] 16.3 Declare `mod format;` in `synapse-telegram/src/main.rs`
- [x] 16.4 Update `handlers.rs` send loop: convert response via `md_to_telegram_html` + `chunk_html`, send with `ParseMode::Html`, fall back to plain-text `chunk_message` on rejection

## Acceptance Criteria

**Test:** `cargo test -p synapse-telegram` — 39 tests pass. Sending prompts that produce code blocks, bold/italic, lists, and links renders formatted output in Telegram.

## Dependencies

- Phase 15 complete

## Implementation Notes

- HTML chosen over MarkdownV2: HTML escapes 3 chars; MarkdownV2 escapes 18+ (fragile for LLM output)
- `chunk_html` uses simulate-then-adjust: computes closing tags after candidate split, reduces split point by the excess if over 4096 chars, ensuring balanced tags never push chunks over the limit
- `ERROR_REPLY` always sent as plain text (no parse mode) to avoid formatting errors on error paths
