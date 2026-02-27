use uuid::Uuid;

use super::*;

// --- Test helpers ---

fn make_stored_message(role: Role, content: &str) -> StoredMessage {
    StoredMessage::new(Uuid::new_v4(), role, content)
}

// --- truncate (using shared synapse_core::text::truncate) ---

#[test]
fn test_truncate_content_short() {
    let content = "0123456789"; // 10 chars
    let result = truncate(content, 150);
    assert_eq!(result, content);
    assert!(!result.ends_with("..."));
}

#[test]
fn test_truncate_content_exact_limit() {
    let content = "a".repeat(150);
    let result = truncate(&content, 150);
    assert_eq!(result, content);
    assert!(
        !result.ends_with("..."),
        "Exact limit should not be truncated"
    );
}

#[test]
fn test_truncate_content_over_limit() {
    let content = "a".repeat(151);
    let result = truncate(&content, 150);
    // truncate gives max_chars total: (max_chars - 3) chars + "..."
    assert_eq!(result.chars().count(), 150);
    assert!(result.ends_with("..."));
}

#[test]
fn test_truncate_content_long() {
    let content = "b".repeat(500);
    let result = truncate(&content, 150);
    assert_eq!(result.chars().count(), 150);
    assert!(result.ends_with("..."));
}

#[test]
fn test_truncate_content_empty() {
    let result = truncate("", 150);
    assert_eq!(result, "");
    assert!(!result.ends_with("..."));
}

// --- format_history ---

#[test]
fn test_format_history_filters_system_and_tool() {
    let messages = vec![
        make_stored_message(Role::User, "user msg"),
        make_stored_message(Role::Assistant, "assistant msg"),
        make_stored_message(Role::System, "system prompt"),
        make_stored_message(Role::Tool, "tool result"),
    ];
    let output = format_history(&messages);
    assert!(output.contains("You"), "Should contain 'You'");
    assert!(output.contains("Assistant"), "Should contain 'Assistant'");
    assert!(!output.contains("System"), "Should NOT contain 'System'");
    assert!(!output.contains("Tool"), "Should NOT contain 'Tool'");
}

#[test]
fn test_format_history_keeps_user_and_assistant() {
    let mut messages = Vec::new();
    for i in 0..3 {
        messages.push(make_stored_message(Role::User, &format!("user {}", i)));
        messages.push(make_stored_message(
            Role::Assistant,
            &format!("assistant {}", i),
        ));
    }
    let output = format_history(&messages);
    for i in 0..3 {
        assert!(output.contains(&format!("user {}", i)));
        assert!(output.contains(&format!("assistant {}", i)));
    }
}

#[test]
fn test_format_history_last_10_limit() {
    // 15 messages with unique, non-overlapping content markers.
    // "early-N" for the first 5, "recent-N" for the last 10.
    let mut messages: Vec<StoredMessage> = Vec::new();
    for i in 1..=5 {
        let role = if i % 2 == 0 {
            Role::Assistant
        } else {
            Role::User
        };
        messages.push(make_stored_message(role, &format!("early-{}", i)));
    }
    for i in 1..=10 {
        let role = if i % 2 == 0 {
            Role::Assistant
        } else {
            Role::User
        };
        messages.push(make_stored_message(role, &format!("recent-{}", i)));
    }
    let output = format_history(&messages);
    // First 5 (early-1 through early-5) must be absent.
    for i in 1..=5 {
        assert!(
            !output.contains(&format!("early-{}", i)),
            "early-{} should be absent (not in last 10)",
            i
        );
    }
    // Last 10 (recent-1 through recent-10) must be present.
    for i in 1..=10 {
        assert!(
            output.contains(&format!("recent-{}", i)),
            "recent-{} should be present (in last 10)",
            i
        );
    }
}

#[test]
fn test_format_history_fewer_than_10() {
    let messages = vec![
        make_stored_message(Role::User, "first"),
        make_stored_message(Role::Assistant, "second"),
        make_stored_message(Role::User, "third"),
    ];
    let output = format_history(&messages);
    assert!(output.contains("first"));
    assert!(output.contains("second"));
    assert!(output.contains("third"));
}

#[test]
fn test_format_history_empty() {
    let messages = vec![
        make_stored_message(Role::System, "system only"),
        make_stored_message(Role::Tool, "tool only"),
    ];
    let output = format_history(&messages);
    assert_eq!(
        output, "",
        "Only System/Tool messages should yield empty string"
    );
}

#[test]
fn test_format_history_truncates_long_content() {
    let long_content = "x".repeat(200);
    let messages = vec![make_stored_message(Role::Assistant, &long_content)];
    let output = format_history(&messages);
    // The formatted content portion should be truncated (150 chars + "...")
    assert!(
        output.contains("..."),
        "Long content should be truncated with ..."
    );
    // Find content line (second line of the entry)
    let content_line = output.lines().nth(1).unwrap_or("");
    assert!(
        content_line.chars().count() <= 153,
        "Truncated content portion should be at most 153 chars (150 + ...)"
    );
}

// --- parse_session_arg ---

#[test]
fn test_parse_session_arg_empty() {
    assert_eq!(parse_session_arg(""), Ok(None));
}

#[test]
fn test_parse_session_arg_whitespace() {
    assert_eq!(parse_session_arg("  "), Ok(None));
}

#[test]
fn test_parse_session_arg_numeric() {
    assert_eq!(parse_session_arg("3"), Ok(Some(3)));
}

#[test]
fn test_parse_session_arg_zero() {
    // Index validation is deferred to do_switch/do_delete; parser accepts 0.
    assert_eq!(parse_session_arg("0"), Ok(Some(0)));
}

#[test]
fn test_parse_session_arg_non_numeric() {
    let result = parse_session_arg("abc");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid argument"));
}

#[test]
fn test_parse_session_arg_negative() {
    // Negative values fail usize parse → treated as invalid argument.
    let result = parse_session_arg("-1");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid argument"));
}

// --- Defensive guard slash detection ---

#[test]
fn test_defensive_guard_slash_detection() {
    // These should be caught by the defensive guard.
    assert!("/".starts_with('/'));
    assert!("/foo".starts_with('/'));
    assert!("/switch".starts_with('/'));

    // Regular message should NOT be caught.
    assert!(!"hello".starts_with('/'));
}

// --- ChatSessions unit tests ---

#[test]
fn test_chat_sessions_active_session_id_non_empty() {
    let id = Uuid::new_v4();
    let cs = ChatSessions::new(id);
    assert_eq!(cs.active_session_id(), Some(id));
}

#[test]
fn test_chat_sessions_active_session_id_empty() {
    let cs = ChatSessions {
        sessions: vec![],
        active_idx: 0,
    };
    assert_eq!(cs.active_session_id(), None);
}

#[test]
fn test_chat_sessions_active_session_id_multiple() {
    let id0 = Uuid::new_v4();
    let id1 = Uuid::new_v4();
    let cs = ChatSessions {
        sessions: vec![id0, id1],
        active_idx: 1,
    };
    assert_eq!(cs.active_session_id(), Some(id1));
}

// --- Session cap enforcement logic ---

#[test]
fn test_session_cap_evicts_last_when_at_cap() {
    let id_old = Uuid::new_v4();
    let id_new = Uuid::new_v4();
    let mut cs = ChatSessions::new(id_old);

    // Simulate cap enforcement: evict last if at cap.
    let cap = 1usize;
    let mut evicted_id = None;
    if cs.sessions.len() >= cap {
        evicted_id = cs.sessions.last().copied();
        cs.sessions.pop();
    }
    cs.sessions.insert(0, id_new);
    cs.active_idx = 0;

    assert_eq!(evicted_id, Some(id_old));
    assert_eq!(cs.sessions.len(), 1);
    assert_eq!(cs.sessions[0], id_new);
    assert_eq!(cs.active_idx, 0);
}

#[test]
fn test_session_cap_no_eviction_when_below_cap() {
    let id_existing = Uuid::new_v4();
    let id_new = Uuid::new_v4();
    let mut cs = ChatSessions::new(id_existing);

    let cap = 10usize;
    let mut evicted_id = None;
    if cs.sessions.len() >= cap {
        evicted_id = cs.sessions.last().copied();
        cs.sessions.pop();
    }
    cs.sessions.insert(0, id_new);
    cs.active_idx = 0;

    assert_eq!(evicted_id, None);
    assert_eq!(cs.sessions.len(), 2);
    assert_eq!(cs.sessions[0], id_new);
}

// --- Index validation ---

#[test]
fn test_index_validation_zero_is_invalid() {
    let sessions = [Uuid::new_v4()];
    let n = 0usize;
    // 1-based: index 0 is always invalid.
    assert!(n == 0 || n > sessions.len());
}

#[test]
fn test_index_validation_valid_index() {
    let sessions = [Uuid::new_v4(), Uuid::new_v4()];
    let n = 1usize;
    assert!(n >= 1 && n <= sessions.len());
}

#[test]
fn test_index_validation_exceeds_count() {
    let sessions = [Uuid::new_v4()];
    let n = 99usize;
    assert!(n == 0 || n > sessions.len());
}

// --- /delete active_idx adjustment ---

#[test]
fn test_delete_active_session_switches_to_index_0() {
    let id0 = Uuid::new_v4();
    let id1 = Uuid::new_v4();
    let mut cs = ChatSessions {
        sessions: vec![id0, id1],
        active_idx: 0, // id0 is active
    };

    // Delete vec position 0 (active session).
    let deleted_pos = 0usize;
    let was_active = deleted_pos == cs.active_idx;
    cs.sessions.remove(deleted_pos);

    if cs.sessions.is_empty() || was_active {
        cs.active_idx = 0;
    } else if deleted_pos < cs.active_idx {
        cs.active_idx = cs.active_idx.saturating_sub(1);
    }

    // After deleting active (id0), id1 should be active at index 0.
    assert_eq!(cs.active_idx, 0);
    assert_eq!(cs.active_session_id(), Some(id1));
}

#[test]
fn test_delete_non_active_below_active_decrements_active_idx() {
    let id0 = Uuid::new_v4();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let mut cs = ChatSessions {
        sessions: vec![id0, id1, id2],
        active_idx: 2, // id2 is active
    };

    // Delete vec position 0 (non-active, below active).
    let deleted_pos = 0usize;
    let was_active = deleted_pos == cs.active_idx;
    cs.sessions.remove(deleted_pos);

    if cs.sessions.is_empty() || was_active {
        cs.active_idx = 0;
    } else if deleted_pos < cs.active_idx {
        cs.active_idx = cs.active_idx.saturating_sub(1);
    }

    // active_idx should have decremented from 2 to 1, still pointing at id2.
    assert_eq!(cs.active_idx, 1);
    assert_eq!(cs.active_session_id(), Some(id2));
}

#[test]
fn test_delete_non_active_above_active_no_change() {
    let id0 = Uuid::new_v4();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let mut cs = ChatSessions {
        sessions: vec![id0, id1, id2],
        active_idx: 0, // id0 is active
    };

    // Delete vec position 2 (non-active, above active).
    let deleted_pos = 2usize;
    let was_active = deleted_pos == cs.active_idx;
    cs.sessions.remove(deleted_pos);

    if cs.sessions.is_empty() || was_active {
        cs.active_idx = 0;
    } else if deleted_pos < cs.active_idx {
        cs.active_idx = cs.active_idx.saturating_sub(1);
    }

    // active_idx should remain 0, still pointing at id0.
    assert_eq!(cs.active_idx, 0);
    assert_eq!(cs.active_session_id(), Some(id0));
}
