use std::env::temp_dir;

use super::*;

#[test]
fn test_role_to_string() {
    assert_eq!(SqliteStore::role_to_string(Role::System), "system");
    assert_eq!(SqliteStore::role_to_string(Role::User), "user");
    assert_eq!(SqliteStore::role_to_string(Role::Assistant), "assistant");
    assert_eq!(SqliteStore::role_to_string(Role::Tool), "tool");
}

#[test]
fn test_parse_role() {
    assert!(matches!(
        SqliteStore::parse_role("system"),
        Ok(Role::System)
    ));
    assert!(matches!(SqliteStore::parse_role("user"), Ok(Role::User)));
    assert!(matches!(
        SqliteStore::parse_role("assistant"),
        Ok(Role::Assistant)
    ));
    assert!(matches!(SqliteStore::parse_role("tool"), Ok(Role::Tool)));
    assert!(matches!(
        SqliteStore::parse_role("invalid"),
        Err(StorageError::InvalidData(_))
    ));
}

/// Create a temporary database for testing.
async fn create_test_store() -> SqliteStore {
    let db_path = temp_dir().join(format!("synapse_test_{}.db", Uuid::new_v4()));
    let url = format!("sqlite:{}", db_path.display());
    SqliteStore::new(&url)
        .await
        .expect("failed to create test store")
}

#[tokio::test]
async fn test_create_and_get_session() {
    let store = create_test_store().await;
    let session = Session::new("deepseek", "deepseek-chat");
    let session_id = session.id;

    // Create session
    store.create_session(&session).await.expect("create failed");

    // Get session
    let retrieved = store.get_session(session_id).await.expect("get failed");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, session_id);
    assert_eq!(retrieved.provider, "deepseek");
    assert_eq!(retrieved.model, "deepseek-chat");
}

#[tokio::test]
async fn test_get_session_not_found() {
    let store = create_test_store().await;
    let result = store
        .get_session(Uuid::new_v4())
        .await
        .expect("query failed");
    assert!(result.is_none());
}

#[tokio::test]
async fn test_list_sessions() {
    let store = create_test_store().await;

    // Create two sessions
    let session1 = Session::new("provider1", "model1");
    let session2 = Session::new("provider2", "model2");

    store
        .create_session(&session1)
        .await
        .expect("create failed");
    store
        .create_session(&session2)
        .await
        .expect("create failed");

    // List sessions
    let summaries = store.list_sessions().await.expect("list failed");
    assert_eq!(summaries.len(), 2);
}

#[tokio::test]
async fn test_touch_session() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;
    let original_updated = session.updated_at;

    store.create_session(&session).await.expect("create failed");

    // Wait a bit to ensure timestamp changes
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    store.touch_session(session_id).await.expect("touch failed");

    let updated = store
        .get_session(session_id)
        .await
        .expect("get failed")
        .unwrap();
    assert!(updated.updated_at > original_updated);
}

#[tokio::test]
async fn test_touch_session_not_found() {
    let store = create_test_store().await;
    let result = store.touch_session(Uuid::new_v4()).await;
    assert!(matches!(result, Err(StorageError::NotFound(_))));
}

#[tokio::test]
async fn test_delete_session() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Delete session
    let deleted = store
        .delete_session(session_id)
        .await
        .expect("delete failed");
    assert!(deleted);

    // Verify it's gone
    let retrieved = store.get_session(session_id).await.expect("get failed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_delete_session_not_found() {
    let store = create_test_store().await;
    let deleted = store
        .delete_session(Uuid::new_v4())
        .await
        .expect("delete failed");
    assert!(!deleted);
}

#[tokio::test]
async fn test_add_and_get_messages() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add messages
    let msg1 = StoredMessage::new(session_id, Role::User, "Hello!");
    let msg2 = StoredMessage::new(session_id, Role::Assistant, "Hi there!");

    store.add_message(&msg1).await.expect("add msg1 failed");
    store.add_message(&msg2).await.expect("add msg2 failed");

    // Get messages
    let messages = store.get_messages(session_id).await.expect("get failed");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::User);
    assert_eq!(messages[0].content, "Hello!");
    assert_eq!(messages[1].role, Role::Assistant);
    assert_eq!(messages[1].content, "Hi there!");
}

#[tokio::test]
async fn test_messages_ordered_by_timestamp() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add messages in order
    for i in 0..5 {
        let msg = StoredMessage::new(session_id, Role::User, format!("Message {}", i));
        store.add_message(&msg).await.expect("add failed");
    }

    // Verify order
    let messages = store.get_messages(session_id).await.expect("get failed");
    for (i, msg) in messages.iter().enumerate() {
        assert_eq!(msg.content, format!("Message {}", i));
    }
}

#[tokio::test]
async fn test_list_sessions_with_message_count_and_preview() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add a user message (this will be the preview)
    let msg = StoredMessage::new(session_id, Role::User, "What is the weather today?");
    store.add_message(&msg).await.expect("add failed");

    // List sessions
    let summaries = store.list_sessions().await.expect("list failed");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].message_count, 1);
    assert_eq!(
        summaries[0].preview,
        Some("What is the weather today?".to_string())
    );
}

#[tokio::test]
async fn test_preview_truncated_to_50_chars() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add a long message
    let long_content =
        "This is a very long message that exceeds fifty characters and should be truncated";
    let msg = StoredMessage::new(session_id, Role::User, long_content);
    store.add_message(&msg).await.expect("add failed");

    // List sessions
    let summaries = store.list_sessions().await.expect("list failed");
    let preview = summaries[0].preview.as_ref().unwrap();
    assert_eq!(preview.len(), 50); // 47 chars + "..."
    assert!(preview.ends_with("..."));
}

#[tokio::test]
async fn test_preview_truncated_multibyte_chars() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // 60 Cyrillic characters — each is 2 bytes, so naive byte slicing panics
    let long_content: String = "а".repeat(60);
    let msg = StoredMessage::new(session_id, Role::User, &long_content);
    store.add_message(&msg).await.expect("add failed");

    let summaries = store.list_sessions().await.expect("list failed");
    let preview = summaries[0].preview.as_ref().unwrap();
    assert_eq!(preview.chars().count(), 50); // 47 chars + "..."
    assert!(preview.ends_with("..."));
}

#[tokio::test]
async fn test_cascade_delete() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add messages
    let msg = StoredMessage::new(session_id, Role::User, "Test");
    store.add_message(&msg).await.expect("add failed");

    // Delete session (should cascade to messages)
    store
        .delete_session(session_id)
        .await
        .expect("delete failed");

    // Messages should be gone too (this would fail without CASCADE)
    let messages = store.get_messages(session_id).await.expect("get failed");
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_cleanup_by_max_sessions() {
    let store = create_test_store().await;

    // Create 5 sessions
    for i in 0..5 {
        let session = Session::new("test", format!("model-{}", i));
        store.create_session(&session).await.expect("create failed");
        // Small delay to ensure different timestamps
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    // Verify 5 sessions exist
    let before = store.list_sessions().await.expect("list failed");
    assert_eq!(before.len(), 5);

    // Run cleanup with max_sessions = 3
    let config = SessionConfig {
        database_url: None,
        max_sessions: 3,
        retention_days: 365, // Don't delete by retention
        auto_cleanup: true,
    };

    let result = store.cleanup(&config).await.expect("cleanup failed");
    assert_eq!(result.sessions_deleted, 2);
    assert_eq!(result.by_max_limit, 2);
    assert_eq!(result.by_retention, 0);

    // Verify only 3 sessions remain
    let after = store.list_sessions().await.expect("list failed");
    assert_eq!(after.len(), 3);
}

#[tokio::test]
async fn test_cleanup_no_action_when_under_limit() {
    let store = create_test_store().await;

    // Create 2 sessions
    let session1 = Session::new("test", "model1");
    let session2 = Session::new("test", "model2");
    store
        .create_session(&session1)
        .await
        .expect("create failed");
    store
        .create_session(&session2)
        .await
        .expect("create failed");

    // Run cleanup with max_sessions = 10 (under limit)
    let config = SessionConfig {
        database_url: None,
        max_sessions: 10,
        retention_days: 365,
        auto_cleanup: true,
    };

    let result = store.cleanup(&config).await.expect("cleanup failed");
    assert_eq!(result.sessions_deleted, 0);
    assert_eq!(result.by_max_limit, 0);
    assert_eq!(result.by_retention, 0);

    // All sessions still exist
    let after = store.list_sessions().await.expect("list failed");
    assert_eq!(after.len(), 2);
}

#[tokio::test]
async fn test_cleanup_result_counts() {
    let store = create_test_store().await;

    // Create 3 sessions
    for i in 0..3 {
        let session = Session::new("test", format!("model-{}", i));
        store.create_session(&session).await.expect("create failed");
    }

    // Cleanup with max_sessions = 1
    let config = SessionConfig {
        database_url: None,
        max_sessions: 1,
        retention_days: 365,
        auto_cleanup: true,
    };

    let result = store.cleanup(&config).await.expect("cleanup failed");

    // Should delete 2 sessions (3 - 1 = 2)
    assert_eq!(result.by_max_limit, 2);
    assert_eq!(
        result.sessions_deleted,
        result.by_max_limit + result.by_retention
    );
}

#[tokio::test]
async fn test_sqlite_role_tool_roundtrip() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add a Tool role message
    let msg = StoredMessage::new(session_id, Role::Tool, "tool result content");
    store.add_message(&msg).await.expect("add tool msg failed");

    // Retrieve and verify
    let messages = store.get_messages(session_id).await.expect("get failed");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].role, Role::Tool);
    assert_eq!(messages[0].content, "tool result content");
}

#[tokio::test]
async fn test_sqlite_tool_calls_roundtrip() {
    let store = create_test_store().await;
    let session = Session::new("test", "model");
    let session_id = session.id;

    store.create_session(&session).await.expect("create failed");

    // Add an assistant message with tool_calls JSON
    let tool_calls_json = r#"[{"id":"call_1","name":"get_weather","input":{"city":"London"}}]"#;
    let msg = StoredMessage::new(session_id, Role::Assistant, "").with_tool_calls(tool_calls_json);
    store
        .add_message(&msg)
        .await
        .expect("add tool calls msg failed");

    // Add a tool result message with tool_results JSON
    let tool_results_json = r#"{"tool_call_id":"call_1"}"#;
    let result_msg = StoredMessage::new(session_id, Role::Tool, "sunny, 20C")
        .with_tool_results(tool_results_json);
    store
        .add_message(&result_msg)
        .await
        .expect("add tool result msg failed");

    // Retrieve and verify
    let messages = store.get_messages(session_id).await.expect("get failed");
    assert_eq!(messages.len(), 2);

    // First message: assistant with tool_calls
    assert_eq!(messages[0].role, Role::Assistant);
    assert_eq!(messages[0].tool_calls.as_deref(), Some(tool_calls_json));
    assert!(messages[0].tool_results.is_none());

    // Second message: tool with tool_results
    assert_eq!(messages[1].role, Role::Tool);
    assert_eq!(messages[1].content, "sunny, 20C");
    assert_eq!(messages[1].tool_results.as_deref(), Some(tool_results_json));
    assert!(messages[1].tool_calls.is_none());
}
