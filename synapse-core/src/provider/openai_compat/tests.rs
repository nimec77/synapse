use super::types::*;
use super::*;
use crate::message::ToolCallData;

// -- ApiRequest serialisation --

#[test]
fn test_api_request_serialization() {
    let request = ApiRequest {
        model: "test-model".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["model"], "test-model");
    assert_eq!(json["max_tokens"], 1024);
    assert_eq!(json["messages"][0]["role"], "user");
    assert_eq!(json["messages"][0]["content"], "Hello");
    assert!(json.get("tools").is_none());
}

#[test]
fn test_api_request_with_system_message() {
    let request = ApiRequest {
        model: "test-model".to_string(),
        messages: vec![
            ApiMessage {
                role: "system".to_string(),
                content: Some("You are a helpful assistant.".to_string()),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
            ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            },
        ],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["messages"][0]["role"], "system");
    assert_eq!(
        json["messages"][0]["content"],
        "You are a helpful assistant."
    );
    assert_eq!(json["messages"][1]["role"], "user");
    assert_eq!(json["messages"][1]["content"], "Hello");
}

#[test]
fn test_streaming_request_serialization() {
    let request = StreamingApiRequest {
        model: "test-model".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 1024,
        stream: true,
        tools: None,
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["model"], "test-model");
    assert_eq!(json["stream"], true);
    assert_eq!(json["max_tokens"], 1024);
}

// -- ApiResponse deserialisation --

#[test]
fn test_api_response_parsing() {
    let json = r#"{
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you today?"
                },
                "finish_reason": "stop"
            }
        ]
    }"#;

    let response: ApiResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.choices.len(), 1);
    assert_eq!(
        response.choices[0].message.content,
        Some("Hello! How can I help you today?".to_string())
    );
}

#[test]
fn test_api_error_parsing() {
    let json = r#"{
        "error": {
            "message": "Incorrect API key provided",
            "type": "invalid_request_error",
            "code": "invalid_api_key"
        }
    }"#;

    let error: ApiError = serde_json::from_str(json).unwrap();
    assert_eq!(error.error.message, "Incorrect API key provided");
}

// -- SSE chunk deserialisation --

#[test]
fn test_parse_sse_text_delta() {
    let json = r#"{
        "id": "chatcmpl-123",
        "choices": [
            {
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }
        ]
    }"#;

    let chunk: StreamChunk = serde_json::from_str(json).unwrap();
    assert_eq!(chunk.choices.len(), 1);
    assert_eq!(chunk.choices[0].delta.content, Some("Hello".to_string()));
    assert!(chunk.choices[0].finish_reason.is_none());
}

#[test]
fn test_parse_sse_done() {
    // The [DONE] marker is checked as a string before JSON parsing.
    assert_eq!(SSE_DONE_MARKER, "[DONE]");

    let json = r#"{
        "id": "chatcmpl-123",
        "choices": [
            {
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }
        ]
    }"#;

    let chunk: StreamChunk = serde_json::from_str(json).unwrap();
    assert!(chunk.choices[0].delta.content.is_none());
    assert_eq!(chunk.choices[0].finish_reason, Some("stop".to_string()));
}

#[test]
fn test_parse_sse_empty_content() {
    let json = r#"{
        "id": "chatcmpl-123",
        "choices": [
            {
                "index": 0,
                "delta": {"content": ""},
                "finish_reason": null
            }
        ]
    }"#;

    let chunk: StreamChunk = serde_json::from_str(json).unwrap();
    let content = chunk.choices[0].delta.content.as_deref().unwrap_or("");
    assert!(content.is_empty());
}

#[test]
fn test_parse_sse_with_role() {
    // First SSE event often has role but no content.
    let json = r#"{
        "id": "chatcmpl-123",
        "choices": [
            {
                "index": 0,
                "delta": {"role": "assistant"},
                "finish_reason": null
            }
        ]
    }"#;

    let chunk: StreamChunk = serde_json::from_str(json).unwrap();
    assert!(chunk.choices[0].delta.content.is_none());
}

// -- Tool-related serialisation --

#[test]
fn test_complete_with_tools_serialization() {
    let tools = vec![OaiTool {
        tool_type: "function".to_string(),
        function: OaiFunction {
            name: "get_weather".to_string(),
            description: Some("Get weather".to_string()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {"location": {"type": "string"}}
            }),
        },
    }];

    let request = ApiRequest {
        model: "test-model".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("What's the weather?".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 1024,
        tools: Some(tools),
        tool_choice: Some("auto".to_string()),
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert!(json.get("tools").is_some());
    assert_eq!(json["tools"][0]["type"], "function");
    assert_eq!(json["tools"][0]["function"]["name"], "get_weather");
    assert_eq!(json["tool_choice"], "auto");
}

#[test]
fn test_complete_with_tools_no_tools() {
    let request = ApiRequest {
        model: "test-model".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert!(json.get("tools").is_none());
}

#[test]
fn test_api_request_tool_choice_absent_without_tools() {
    let request = ApiRequest {
        model: "test-model".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert!(
        json.get("tool_choice").is_none(),
        "tool_choice must be absent when no tools"
    );
}

#[test]
fn test_tool_call_response_parsing() {
    let json = r#"{
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "arguments": "{\"location\":\"London\"}"
                    }
                }]
            }
        }]
    }"#;

    let response: ApiResponse = serde_json::from_str(json).unwrap();
    let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_1");
    assert_eq!(tool_calls[0].function.name, "get_weather");
}

// -- build_api_messages helper --

#[test]
fn test_tool_role_message_serialization() {
    let messages = vec![Message::tool_result("call_1", "Sunny, 20C")];
    let api_messages = build_api_messages(&messages);

    assert_eq!(api_messages[0].role, "tool");
    assert_eq!(api_messages[0].tool_call_id, Some("call_1".to_string()));
    assert_eq!(api_messages[0].content, Some("Sunny, 20C".to_string()));
}

#[test]
fn test_assistant_tool_call_message_serialization() {
    let mut assistant_msg = Message::new(Role::Assistant, "");
    assistant_msg.tool_calls = Some(vec![ToolCallData {
        id: "call_1".to_string(),
        name: "get_weather".to_string(),
        input: serde_json::json!({"location": "London"}),
    }]);

    let messages = vec![
        Message::new(Role::User, "What's the weather?"),
        assistant_msg,
    ];

    let api_messages = build_api_messages(&messages);
    assert_eq!(api_messages.len(), 2);

    assert_eq!(api_messages[1].role, "assistant");
    let tool_calls = api_messages[1].tool_calls.as_ref().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_1");
    assert_eq!(tool_calls[0].call_type, "function");
    assert_eq!(tool_calls[0].function.name, "get_weather");

    let args: serde_json::Value = serde_json::from_str(&tool_calls[0].function.arguments).unwrap();
    assert_eq!(args["location"], "London");
}

// -- to_oai_tools helper --

#[test]
fn test_to_oai_tools_empty() {
    let result = to_oai_tools(&[]);
    assert!(result.is_none());
}

#[test]
fn test_to_oai_tools_conversion() {
    let tools = vec![ToolDefinition {
        name: "test_tool".to_string(),
        description: Some("A test tool".to_string()),
        input_schema: serde_json::json!({"type": "object"}),
    }];
    let result = to_oai_tools(&tools).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].tool_type, "function");
    assert_eq!(result[0].function.name, "test_tool");
}

// ============================================================================
// SY-22: Reasoning / thinking mode tests
// ============================================================================

/// AC: When `thinking` is set on `ApiRequest`, it serialises correctly as
/// `{"type": "enabled"}` and `reasoning_effort` is present.
#[test]
fn test_request_serializes_thinking_when_reasoning_set() {
    let request = ApiRequest {
        model: "deepseek-v4-pro".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("Think hard".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 4096,
        tools: None,
        tool_choice: None,
        thinking: Some(ThinkingConfig {
            kind: "enabled".to_string(),
        }),
        reasoning_effort: Some("high".to_string()),
    };

    let json = serde_json::to_value(&request).unwrap();
    assert_eq!(json["thinking"]["type"], "enabled");
    assert_eq!(json["reasoning_effort"], "high");
}

/// AC: When `thinking` and `reasoning_effort` are `None` (default), they are
/// completely absent from the serialised JSON — regression guard for OpenAI.
#[test]
fn test_request_omits_thinking_when_reasoning_none() {
    let request = ApiRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ApiMessage {
            role: "user".to_string(),
            content: Some("Hello".to_string()),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
        thinking: None,
        reasoning_effort: None,
    };

    let json = serde_json::to_value(&request).unwrap();
    assert!(
        json.get("thinking").is_none(),
        "thinking must be absent for non-reasoning providers"
    );
    assert!(
        json.get("reasoning_effort").is_none(),
        "reasoning_effort must be absent for non-reasoning providers"
    );
}

/// AC: `ChoiceMessage.reasoning_content` is populated when the API responds with it.
#[test]
fn test_response_parses_reasoning_content() {
    let json = r#"{
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "The answer is 42.",
                "reasoning_content": "Let me think step by step..."
            }
        }]
    }"#;

    let response: ApiResponse = serde_json::from_str(json).unwrap();
    assert_eq!(
        response.choices[0].message.content,
        Some("The answer is 42.".to_string())
    );
    assert_eq!(
        response.choices[0].message.reasoning_content,
        Some("Let me think step by step...".to_string())
    );
}

/// AC: `build_api_messages` includes `reasoning_content` on the outbound
/// `ApiMessage` when the source `Message` has both `tool_calls` and
/// `reasoning_content` (DeepSeek tool-call history rule).
#[test]
fn test_outbound_message_includes_reasoning_for_tool_calls() {
    let assistant_msg =
        Message::new(Role::Assistant, "").with_reasoning("I need to call a tool first".to_string());
    let mut assistant_msg = assistant_msg;
    assistant_msg.tool_calls = Some(vec![ToolCallData {
        id: "call_1".to_string(),
        name: "search".to_string(),
        input: serde_json::json!({"query": "Rust"}),
    }]);

    let messages = vec![Message::new(Role::User, "Search for Rust"), assistant_msg];

    let api_messages = build_api_messages(&messages);
    assert_eq!(api_messages.len(), 2);

    // User message has no reasoning
    assert!(api_messages[0].reasoning_content.is_none());

    // Assistant message with tool_calls MUST include reasoning_content
    assert_eq!(
        api_messages[1].reasoning_content,
        Some("I need to call a tool first".to_string())
    );
}

/// AC: `build_api_messages` drops `reasoning_content` from text-only assistant
/// turns (tool_calls is None) to save tokens.
#[test]
fn test_outbound_message_drops_reasoning_for_text_only() {
    let assistant_msg =
        Message::new(Role::Assistant, "Here is the answer").with_reasoning("My reasoning");
    // No tool_calls — text-only turn.
    assert!(assistant_msg.tool_calls.is_none());

    let messages = vec![Message::new(Role::User, "Question"), assistant_msg];

    let api_messages = build_api_messages(&messages);
    assert_eq!(api_messages.len(), 2);

    // reasoning_content MUST be absent for text-only turns
    assert!(
        api_messages[1].reasoning_content.is_none(),
        "reasoning_content must be dropped for text-only assistant turns"
    );
}

/// AC: The SSE delta parses `reasoning_content` correctly.
///
/// This test verifies the `StreamDelta` struct can deserialise both
/// `reasoning_content` and `content` fields from a DeepSeek SSE chunk.
/// The actual `stream_sse` function's `ReasoningDelta` yield path is
/// covered by an integration-style test in the agent tests.
#[test]
fn test_stream_yields_reasoning_delta_then_text_delta() {
    // Reasoning delta chunk
    let reasoning_json = r#"{
        "id": "chatcmpl-r1",
        "choices": [
            {
                "index": 0,
                "delta": {"reasoning_content": "Step 1: analyse"},
                "finish_reason": null
            }
        ]
    }"#;
    let chunk: StreamChunk = serde_json::from_str(reasoning_json).unwrap();
    assert_eq!(
        chunk.choices[0].delta.reasoning_content,
        Some("Step 1: analyse".to_string())
    );
    assert!(chunk.choices[0].delta.content.is_none());

    // Answer delta chunk
    let text_json = r#"{
        "id": "chatcmpl-r1",
        "choices": [
            {
                "index": 0,
                "delta": {"content": "The answer is 42"},
                "finish_reason": null
            }
        ]
    }"#;
    let chunk2: StreamChunk = serde_json::from_str(text_json).unwrap();
    assert!(chunk2.choices[0].delta.reasoning_content.is_none());
    assert_eq!(
        chunk2.choices[0].delta.content,
        Some("The answer is 42".to_string())
    );
}
