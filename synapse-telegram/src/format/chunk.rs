//! HTML chunking utilities for Telegram's 4096-character message limit.
//!
//! Splits long HTML strings into ≤4096-character chunks while keeping HTML tags
//! balanced: closes open tags at each split boundary and reopens them in the
//! next chunk so that Telegram always receives well-formed HTML.

/// Telegram maximum message length.
pub const TELEGRAM_MSG_LIMIT: usize = 4096;

/// A tracked open tag for balanced-tag chunking.
#[derive(Clone)]
pub(super) struct OpenTag {
    pub(super) open_str: String,
    pub(super) close_str: String,
}

/// Split an HTML string into ≤4096-character chunks with balanced tags.
///
/// Tags are closed at each split point and reopened in the next chunk so that
/// Telegram always receives well-formed HTML.
pub fn chunk_html(html: &str) -> Vec<String> {
    if html.len() <= TELEGRAM_MSG_LIMIT {
        return vec![html.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut remaining = html;
    let mut open_tags: Vec<OpenTag> = Vec::new();

    while !remaining.is_empty() {
        // Build a prefix that reopens any still-open tags from the previous chunk.
        let prefix: String = open_tags.iter().map(|t| t.open_str.as_str()).collect();

        // If remaining content (plus prefix) fits in one chunk, finish up.
        if prefix.len() + remaining.len() <= TELEGRAM_MSG_LIMIT {
            let mut chunk = String::with_capacity(prefix.len() + remaining.len());
            chunk.push_str(&prefix);
            chunk.push_str(remaining);
            chunks.push(chunk);
            break;
        }

        // Maximum bytes we can use for content (before adding closing tags).
        let max_content = TELEGRAM_MSG_LIMIT.saturating_sub(prefix.len());
        if max_content == 0 {
            // Pathological: prefix alone fills the limit. Emit prefix + one char.
            let ch_len = remaining.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            let mut chunk = String::with_capacity(prefix.len() + ch_len);
            chunk.push_str(&prefix);
            chunk.push_str(&remaining[..ch_len]);
            chunks.push(chunk);
            remaining = &remaining[ch_len..];
            continue;
        }

        let slice_len = floor_char_boundary(remaining, max_content.min(remaining.len()));
        let mut split_at = find_split_point(&remaining[..slice_len]);

        // Simulate the tag state after this chunk to know the actual closing size.
        let mut temp_tags = open_tags.clone();
        update_open_tags(&mut temp_tags, &remaining[..split_at]);
        let closing: String = temp_tags
            .iter()
            .rev()
            .map(|t| t.close_str.as_str())
            .collect();

        // If the chunk would exceed the limit, shrink split_at by the excess and retry once.
        let total = prefix.len() + split_at + closing.len();
        if total > TELEGRAM_MSG_LIMIT {
            let excess = total - TELEGRAM_MSG_LIMIT;
            let reduced = floor_char_boundary(remaining, split_at.saturating_sub(excess));
            split_at = find_split_point(&remaining[..reduced.max(1)]);
            temp_tags = open_tags.clone();
            update_open_tags(&mut temp_tags, &remaining[..split_at]);
        }

        let closing: String = temp_tags
            .iter()
            .rev()
            .map(|t| t.close_str.as_str())
            .collect();
        let chunk_content = &remaining[..split_at];
        remaining = remaining[split_at..].trim_start_matches('\n');

        let mut chunk = String::with_capacity(prefix.len() + split_at + closing.len());
        chunk.push_str(&prefix);
        chunk.push_str(chunk_content);
        chunk.push_str(&closing);
        chunks.push(chunk);

        open_tags = temp_tags;
    }

    chunks
}

/// Round `idx` down to the nearest valid UTF-8 character boundary in `s`.
pub(crate) fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Find the best byte offset at which to split `slice` (≤ slice.len()).
///
/// Priority: `\n\n` > `\n` > ` ` > hard split.
/// Never splits inside an HTML tag (`<...>`).
fn find_split_point(slice: &str) -> usize {
    // Helper: check that a byte offset is not inside a `<...>` tag.
    let not_in_tag = |pos: usize| -> bool {
        let before = &slice[..pos];
        let open_count = before.chars().filter(|&c| c == '<').count();
        let close_count = before.chars().filter(|&c| c == '>').count();
        open_count <= close_count
    };

    // Try paragraph boundary.
    if let Some(pos) = rfind_not_in_tag(slice, "\n\n", not_in_tag) {
        return pos + 2;
    }
    // Try newline.
    if let Some(pos) = rfind_char_not_in_tag(slice, '\n', &not_in_tag) {
        return pos + 1;
    }
    // Try space.
    if let Some(pos) = rfind_char_not_in_tag(slice, ' ', &not_in_tag) {
        return pos + 1;
    }
    // Hard split.
    slice.len()
}

fn rfind_not_in_tag(slice: &str, pat: &str, not_in_tag: impl Fn(usize) -> bool) -> Option<usize> {
    // Use an exclusive `end` so slice[..end] is always a valid UTF-8 boundary.
    // `pat` is ASCII, so positions returned by `rfind` are always char boundaries.
    let mut end = slice.len();
    while let Some(found) = slice[..end].rfind(pat) {
        if not_in_tag(found) {
            return Some(found);
        }
        if found == 0 {
            break;
        }
        end = found; // exclude current match in next search
    }
    None
}

fn rfind_char_not_in_tag(
    slice: &str,
    ch: char,
    not_in_tag: &impl Fn(usize) -> bool,
) -> Option<usize> {
    for (i, c) in slice.char_indices().rev() {
        if c == ch && not_in_tag(i) {
            return Some(i);
        }
    }
    None
}

/// Walk `content` and update `open_tags` to reflect the net open tags after processing it.
pub(super) fn update_open_tags(open_tags: &mut Vec<OpenTag>, content: &str) {
    // Simple tag scanner for Telegram's small HTML subset.
    let mut pos = 0;
    let bytes = content.as_bytes();
    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            // Find closing `>`.
            if let Some(end) = content[pos..].find('>') {
                let tag_inner = &content[pos + 1..pos + end]; // e.g. "b", "/b", "a href=\"...\""
                if let Some(rest) = tag_inner.strip_prefix('/') {
                    // Closing tag — pop from stack.
                    let name = rest.trim();
                    // Pop the last matching open tag.
                    if let Some(idx) = open_tags
                        .iter()
                        .rposition(|t| tag_name_of(&t.open_str) == name)
                    {
                        open_tags.remove(idx);
                    }
                } else if !tag_inner.starts_with('!') && !tag_inner.ends_with('/') {
                    // Opening tag (not comment, not self-closing).
                    let full_open = &content[pos..pos + end + 1];
                    let name = tag_inner.split_ascii_whitespace().next().unwrap_or("");
                    let close_str = format!("</{}>", name);
                    // Special case: <pre><code> is emitted as one unit.
                    // We track them as a single entry.
                    open_tags.push(OpenTag {
                        open_str: full_open.to_string(),
                        close_str,
                    });
                }
                pos += end + 1;
                continue;
            }
        }
        pos += 1;
    }
}

/// Extract the tag name from an open tag string like `<a href="...">` → `"a"`.
fn tag_name_of(open_str: &str) -> &str {
    let inner = open_str.trim_start_matches('<').trim_end_matches('>');
    inner.split_ascii_whitespace().next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_html_short_message() {
        let html = "<b>Hello</b>";
        let chunks = chunk_html(html);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], html);
    }

    #[test]
    fn test_chunk_html_splits_at_paragraph_boundary() {
        // Two paragraphs that together exceed 4096 chars.
        let para1 = "a".repeat(2500);
        let para2 = "b".repeat(2500);
        let html = format!("{}\n\n{}", para1, para2);
        let chunks = chunk_html(&html);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
        }
    }

    #[test]
    fn test_chunk_html_closes_and_reopens_tags() {
        // A bold span that spans more than 4096 chars.
        let inner = "x".repeat(4100);
        let html = format!("<b>{}</b>", inner);
        let chunks = chunk_html(&html);
        // Must have more than one chunk.
        assert!(chunks.len() >= 2);
        // Every chunk must fit within the limit.
        for chunk in &chunks {
            assert!(
                chunk.len() <= TELEGRAM_MSG_LIMIT,
                "chunk too long: {}",
                chunk.len()
            );
        }
        // The first chunk should close <b> and the second should reopen it.
        assert!(chunks[0].ends_with("</b>"), "first chunk should close <b>");
        assert!(
            chunks[1].starts_with("<b>"),
            "second chunk should reopen <b>"
        );
    }

    #[test]
    fn test_chunk_html_never_splits_inside_tag() {
        // Construct HTML where a long attribute value could cause a mid-tag split.
        let url = "https://example.com/".to_string() + &"path/".repeat(500);
        let html = format!("<a href=\"{}\">link text here</a>", url);
        let chunks = chunk_html(&html);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
            // A chunk should not contain an unclosed `<` without a matching `>`.
            let open = chunk.chars().filter(|&c| c == '<').count();
            let close = chunk.chars().filter(|&c| c == '>').count();
            assert_eq!(open, close, "unbalanced angle brackets in chunk: {}", chunk);
        }
    }

    #[test]
    fn test_chunk_html_long_code_block() {
        let code = "x".repeat(5000);
        let html = format!("<pre><code>{}</code></pre>", code);
        let chunks = chunk_html(&html);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
        }
    }

    #[test]
    fn test_chunk_html_multibyte_text() {
        // Each Cyrillic character is 2 bytes; build ~5000 bytes worth.
        let cyrillic = "Привет ".repeat(400); // ~2800 chars, ~5600 bytes
        let chunks = chunk_html(&cyrillic);
        // Must not panic and every chunk must be valid UTF-8 within the byte limit.
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
            // Verify it's valid UTF-8 (would panic on invalid slice otherwise).
            let _ = chunk.chars().count();
        }
    }

    #[test]
    fn test_chunk_html_emoji_boundary() {
        // Each emoji is 4 bytes. Fill to just over the limit so the split lands
        // inside an emoji if boundaries are not respected.
        let emoji = "😀".repeat(1025); // 4100 bytes
        let chunks = chunk_html(&emoji);
        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.len() <= TELEGRAM_MSG_LIMIT);
            let _ = chunk.chars().count(); // panics if invalid UTF-8 slice
        }
    }
}
