//! Markdown-to-Telegram-HTML conversion and HTML chunking utilities.
//!
//! Telegram supports a limited HTML subset for formatted messages. This module
//! converts LLM Markdown output into that subset and splits long messages into
//! ≤4096-character chunks while keeping tags balanced.

mod chunk;

pub(crate) use chunk::floor_char_boundary;
pub use chunk::{TELEGRAM_MSG_LIMIT, chunk_html};

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Escape the three characters special in Telegram HTML: `&`, `<`, `>`.
pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c => out.push(c),
        }
    }
    out
}

/// Convert a Markdown string to a Telegram-compatible HTML string.
///
/// Supported elements:
/// - Bold, italic, strikethrough, inline code
/// - Fenced and indented code blocks
/// - Links, headings (rendered as bold), blockquotes
/// - Unordered and ordered lists, task lists
/// - Tables (rendered as monospace `<pre>`)
/// - Images (text fallback)
pub fn md_to_telegram_html(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(markdown, options);

    let mut out = String::new();
    // Stack tracking list type: None = unordered, Some(n) = ordered with next counter n.
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut first_paragraph = true;
    // Accumulate table cells for monospace rendering.
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    for event in parser {
        match event {
            // --- Formatting ---
            Event::Start(Tag::Strong) => out.push_str("<b>"),
            Event::End(TagEnd::Strong) => out.push_str("</b>"),
            Event::Start(Tag::Emphasis) => out.push_str("<i>"),
            Event::End(TagEnd::Emphasis) => out.push_str("</i>"),
            Event::Start(Tag::Strikethrough) => out.push_str("<s>"),
            Event::End(TagEnd::Strikethrough) => out.push_str("</s>"),

            // --- Inline code ---
            Event::Code(text) => {
                out.push_str("<code>");
                out.push_str(&escape_html(&text));
                out.push_str("</code>");
            }

            // --- Code blocks ---
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) if !lang.is_empty() => {
                out.push_str("<pre><code class=\"language-");
                out.push_str(&escape_html(&lang));
                out.push_str("\">");
            }
            Event::Start(Tag::CodeBlock(_)) => out.push_str("<pre><code>"),
            Event::End(TagEnd::CodeBlock) => out.push_str("</code></pre>"),

            // --- Links ---
            Event::Start(Tag::Link { dest_url, .. }) => {
                out.push_str("<a href=\"");
                out.push_str(&escape_html(&dest_url));
                out.push_str("\">");
            }
            Event::End(TagEnd::Link) => out.push_str("</a>"),

            // --- Headings → bold (Telegram has no heading tags) ---
            Event::Start(Tag::Heading { .. }) => out.push_str("\n<b>"),
            Event::End(TagEnd::Heading(_)) => out.push_str("</b>\n"),

            // --- Blockquotes ---
            Event::Start(Tag::BlockQuote(_)) => out.push_str("<blockquote>"),
            Event::End(TagEnd::BlockQuote(_)) => out.push_str("</blockquote>"),

            // --- Paragraphs ---
            Event::Start(Tag::Paragraph) if !first_paragraph => {
                out.push('\n');
            }
            Event::End(TagEnd::Paragraph) => {
                out.push('\n');
                first_paragraph = false;
            }

            // --- Lists ---
            Event::Start(Tag::List(None)) => list_stack.push(None),
            Event::Start(Tag::List(Some(start))) => list_stack.push(Some(start)),
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_stack.len().saturating_sub(1));
                match list_stack.last_mut() {
                    Some(Some(n)) => {
                        out.push_str(&indent);
                        out.push_str(&n.to_string());
                        out.push_str(". ");
                        *n += 1;
                    }
                    _ => {
                        out.push_str(&indent);
                        out.push_str("• ");
                    }
                }
            }
            Event::End(TagEnd::Item) => out.push('\n'),

            // --- Task list markers ---
            Event::TaskListMarker(checked) => {
                if checked {
                    out.push_str("[x] ");
                } else {
                    out.push_str("[ ] ");
                }
            }

            // --- Tables → monospace pre block ---
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                // Render accumulated table as <pre> monospace text.
                if !table_rows.is_empty() {
                    // Compute column widths.
                    let col_count = table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    let mut widths = vec![0usize; col_count];
                    for row in &table_rows {
                        for (i, cell) in row.iter().enumerate() {
                            if i < col_count {
                                widths[i] = widths[i].max(cell.len());
                            }
                        }
                    }
                    out.push_str("<pre>");
                    for (row_idx, row) in table_rows.iter().enumerate() {
                        let mut line = String::new();
                        for (i, cell) in row.iter().enumerate() {
                            if i < col_count {
                                line.push_str(cell);
                                let pad = widths[i].saturating_sub(cell.len());
                                for _ in 0..pad {
                                    line.push(' ');
                                }
                                if i + 1 < col_count {
                                    line.push_str(" | ");
                                }
                            }
                        }
                        out.push_str(&escape_html(&line));
                        out.push('\n');
                        // Separator after header row.
                        if row_idx == 0 {
                            let sep: String = widths
                                .iter()
                                .map(|&w| "-".repeat(w))
                                .collect::<Vec<_>>()
                                .join("-+-");
                            out.push_str(&escape_html(&sep));
                            out.push('\n');
                        }
                    }
                    out.push_str("</pre>");
                }
                table_rows.clear();
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(current_cell.clone());
                current_cell.clear();
            }

            // --- Images → text fallback ---
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                out.push_str("[image");
                if !title.is_empty() {
                    out.push_str(": ");
                    out.push_str(&escape_html(&title));
                }
                out.push_str("](");
                out.push_str(&escape_html(&dest_url));
                out.push(')');
            }
            Event::End(TagEnd::Image) => {} // alt text handled via Text events below

            // --- Text ---
            Event::Text(text) => {
                if in_table {
                    current_cell.push_str(&text);
                } else {
                    out.push_str(&escape_html(&text));
                }
            }

            // --- Breaks ---
            Event::SoftBreak | Event::HardBreak => out.push('\n'),

            // --- Horizontal rule ---
            Event::Rule => out.push_str("\n---\n"),

            // --- Raw HTML → escape and pass through as text ---
            Event::Html(html) | Event::InlineHtml(html) => {
                out.push_str(&escape_html(&html));
            }

            // Ignore everything else (footnotes, metadata, etc.)
            _ => {}
        }
    }

    // Trim leading/trailing blank lines.
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- escape_html ---

    #[test]
    fn test_escape_html_ampersand() {
        assert_eq!(escape_html("a&b"), "a&amp;b");
    }

    #[test]
    fn test_escape_html_angle_brackets() {
        assert_eq!(escape_html("<tag>"), "&lt;tag&gt;");
    }

    #[test]
    fn test_escape_html_no_change() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn test_escape_html_mixed() {
        assert_eq!(
            escape_html("<a href=\"x&y\">"),
            "&lt;a href=\"x&amp;y\"&gt;"
        );
    }

    // --- md_to_telegram_html ---

    #[test]
    fn test_md_to_telegram_html_empty() {
        assert_eq!(md_to_telegram_html(""), "");
    }

    #[test]
    fn test_md_to_telegram_html_plain_text() {
        let result = md_to_telegram_html("Hello, world!");
        assert!(result.contains("Hello, world!"));
    }

    #[test]
    fn test_md_to_telegram_html_bold() {
        let result = md_to_telegram_html("**bold**");
        assert!(result.contains("<b>bold</b>"));
    }

    #[test]
    fn test_md_to_telegram_html_italic() {
        let result = md_to_telegram_html("_italic_");
        assert!(result.contains("<i>italic</i>"));
    }

    #[test]
    fn test_md_to_telegram_html_strikethrough() {
        let result = md_to_telegram_html("~~strike~~");
        assert!(result.contains("<s>strike</s>"));
    }

    #[test]
    fn test_md_to_telegram_html_inline_code() {
        let result = md_to_telegram_html("`code`");
        assert!(result.contains("<code>code</code>"));
    }

    #[test]
    fn test_md_to_telegram_html_fenced_code_with_lang() {
        let md = "```rust\nfn main() {}\n```";
        let result = md_to_telegram_html(md);
        assert!(result.contains("<pre><code class=\"language-rust\">"));
        assert!(result.contains("fn main() {}"));
        assert!(result.contains("</code></pre>"));
    }

    #[test]
    fn test_md_to_telegram_html_fenced_code_no_lang() {
        let md = "```\nhello\n```";
        let result = md_to_telegram_html(md);
        assert!(result.contains("<pre><code>"));
        assert!(result.contains("hello"));
        assert!(result.contains("</code></pre>"));
    }

    #[test]
    fn test_md_to_telegram_html_link() {
        let md = "[Rust](https://www.rust-lang.org)";
        let result = md_to_telegram_html(md);
        assert!(result.contains("<a href=\"https://www.rust-lang.org\">Rust</a>"));
    }

    #[test]
    fn test_md_to_telegram_html_heading() {
        let result = md_to_telegram_html("# Title");
        assert!(result.contains("<b>"));
        assert!(result.contains("Title"));
        assert!(result.contains("</b>"));
    }

    #[test]
    fn test_md_to_telegram_html_blockquote() {
        let result = md_to_telegram_html("> quote");
        assert!(result.contains("<blockquote>"));
        assert!(result.contains("quote"));
        assert!(result.contains("</blockquote>"));
    }

    #[test]
    fn test_md_to_telegram_html_unordered_list() {
        let md = "- item one\n- item two";
        let result = md_to_telegram_html(md);
        assert!(result.contains("• item one"));
        assert!(result.contains("• item two"));
    }

    #[test]
    fn test_md_to_telegram_html_ordered_list() {
        let md = "1. first\n2. second";
        let result = md_to_telegram_html(md);
        assert!(result.contains("1. first"));
        assert!(result.contains("2. second"));
    }

    #[test]
    fn test_md_to_telegram_html_task_list() {
        let md = "- [x] done\n- [ ] todo";
        let result = md_to_telegram_html(md);
        assert!(result.contains("[x]"));
        assert!(result.contains("[ ]"));
    }

    #[test]
    fn test_md_to_telegram_html_nested_formatting() {
        let md = "**bold _italic_ bold**";
        let result = md_to_telegram_html(md);
        assert!(result.contains("<b>"));
        assert!(result.contains("<i>italic</i>"));
        assert!(result.contains("</b>"));
    }

    #[test]
    fn test_md_to_telegram_html_html_escaping_in_text() {
        let md = "Use `<b>` tags";
        let result = md_to_telegram_html(md);
        // The angle brackets inside backticks should be escaped.
        assert!(result.contains("&lt;b&gt;"));
    }

    #[test]
    fn test_md_to_telegram_html_html_escaping_in_code_block() {
        let md = "```\n<script>alert(1)</script>\n```";
        let result = md_to_telegram_html(md);
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
    }
}
