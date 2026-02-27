//! Text utility functions for the Synapse core library.

/// Truncate a string to a maximum number of Unicode characters.
///
/// If the string exceeds `max_chars`, the result is the first `max_chars - 3`
/// characters followed by `...`. If `max_chars <= 3`, returns `"."` repeated
/// `max_chars` times. Strings at or below the limit are returned unchanged.
///
/// Uses `.chars()` for multi-byte safety.
pub fn truncate(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars <= 3 {
        ".".repeat(max_chars)
    } else {
        let truncated: String = s.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_limit_unchanged() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_longer_string_adds_ellipsis() {
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_at_limit_three_returns_dots() {
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn test_truncate_at_limit_two_returns_dots() {
        assert_eq!(truncate("hello", 2), "..");
    }

    #[test]
    fn test_truncate_at_limit_one_returns_dot() {
        assert_eq!(truncate("hello", 1), ".");
    }

    #[test]
    fn test_truncate_at_limit_zero_returns_empty() {
        assert_eq!(truncate("hello", 0), "");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 10), "");
        assert_eq!(truncate("", 0), "");
    }

    #[test]
    fn test_truncate_exact_boundary() {
        // "hi" is 2 chars, limit is 2 → unchanged
        assert_eq!(truncate("hi", 2), "hi");
    }

    #[test]
    fn test_truncate_multibyte_chars() {
        // Cyrillic 'а' is 2 bytes; naive byte slicing would panic
        let s: String = "а".repeat(20);
        let result = truncate(&s, 10);
        assert_eq!(result.chars().count(), 10);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_multibyte_below_limit() {
        let s: String = "а".repeat(5);
        let result = truncate(&s, 10);
        assert_eq!(result, s);
    }

    #[test]
    fn test_truncate_emoji() {
        // Each emoji is multiple bytes but 1 char
        let s = "🦀🦀🦀🦀🦀";
        let result = truncate(s, 3);
        assert_eq!(result, "...");
    }

    #[test]
    fn test_truncate_unicode_large_limit() {
        let s = "hello world";
        assert_eq!(truncate(s, 150), "hello world");
    }
}
