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
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
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
            },
            ApiMessage {
                role: "user".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
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
        }],
        max_tokens: 1024,
        stream: true,
        tools: None,
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
        }],
        max_tokens: 1024,
        tools: Some(tools),
        tool_choice: Some("auto".to_string()),
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
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
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
        }],
        max_tokens: 1024,
        tools: None,
        tool_choice: None,
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
