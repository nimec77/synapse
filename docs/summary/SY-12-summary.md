# SY-12 Summary: Phase 11 - MCP Integration

**Status:** COMPLETE
**Date:** 2026-02-08

---

## Overview

SY-12 integrates the Model Context Protocol (MCP) into Synapse, enabling tool calling via external MCP servers. This is the most cross-cutting change in the project's history, touching the data model, all three LLM providers, configuration, storage, and both CLI modes. The implementation adds an MCP client that connects to stdio-based MCP servers via the `rmcp` crate, a provider-agnostic tool schema system, tool-aware extensions to the `LlmProvider` trait, and an `Agent` orchestrator that implements the detect-execute-return tool call loop.

The core design principle is graceful degradation: without `mcp_servers.json`, Synapse behaves identically to pre-MCP. MCP server failures produce warnings but never crash the agent.

---

## What Was Built

### New Components

1. **MCP Error Type** (`synapse-core/src/mcp.rs`)
   - `McpError` enum with `ConfigError`, `ConnectionError`, `ToolError`, `IoError` variants using `thiserror`
   - `load_mcp_config()` function with path resolution: `SYNAPSE_MCP_CONFIG` env var > `~/.config/synapse/mcp_servers.json`
   - Module declarations for `protocol` and `tools` submodules

2. **MCP Configuration** (`synapse-core/src/mcp/protocol.rs`)
   - `McpServerConfig` struct: `command`, `args`, `env` fields
   - `McpConfig` struct with `mcpServers` key (compatible with Claude Desktop / Windsurf standard format)
   - `ToolDefinition` struct: provider-agnostic tool schema with `name`, `description`, `input_schema`

3. **MCP Client** (`synapse-core/src/mcp/tools.rs`)
   - `McpClient` struct managing connections to MCP servers via `rmcp` with `TokioChildProcess` transport
   - Tool discovery via `list_tools()` and a unified tool registry mapping tool names to server names
   - `call_tool(name, input)` for tool execution routed to the correct server
   - `has_tools()`, `tool_definitions()`, `shutdown()` methods
   - Servers that fail to start are logged as warnings but do not prevent initialization

4. **Agent Orchestrator** (`synapse-core/src/agent.rs`)
   - `Agent` struct coordinating `LlmProvider` and optional `McpClient`
   - `AgentError` enum wrapping `ProviderError`, `McpError`, and `MaxIterationsExceeded`
   - `complete()` method: loops up to 10 iterations -- sends messages with tool schemas, executes tool calls via MCP, appends results, re-sends until LLM returns text-only response
   - `stream()` and `stream_owned()` methods: uses non-streaming `complete_with_tools()` for tool call iterations, yields final response as `TextDelta` + `Done`
   - `shutdown()` for graceful MCP client cleanup

5. **Database Migration** (`synapse-core/migrations/20260208_002_add_tool_columns.sql`)
   - Adds `tool_calls TEXT` and `tool_results TEXT` nullable columns to the `messages` table

### Modified Components

1. **Data Model Extensions** (`synapse-core/src/message.rs`)
   - Added `Tool` variant to `Role` enum
   - Created `ToolCallData` struct with `id`, `name`, `input` fields
   - Extended `Message` with `tool_calls: Option<Vec<ToolCallData>>` and `tool_call_id: Option<String>` (backward compatible -- new fields default to `None`)
   - Added `Message::tool_result(tool_call_id, content)` builder method

2. **StoredMessage Extensions** (`synapse-core/src/session.rs`)
   - Added `tool_calls: Option<String>` and `tool_results: Option<String>` fields
   - Added `with_tool_calls()` and `with_tool_results()` builder methods

3. **SqliteStore Updates** (`synapse-core/src/storage/sqlite.rs`)
   - `parse_role()`: added `"tool" => Ok(Role::Tool)`
   - `role_to_string()`: added `Role::Tool => "tool"`
   - `add_message()` and `get_messages()`: bind and read `tool_calls` and `tool_results` columns

4. **LlmProvider Trait Extension** (`synapse-core/src/provider.rs`)
   - Added `complete_with_tools()` with default implementation delegating to `complete()`
   - Added `stream_with_tools()` with default implementation delegating to `stream()`
   - Backward compatible: existing providers work via defaults, `MockProvider` unchanged

5. **Anthropic Provider** (`synapse-core/src/provider/anthropic.rs`)
   - `AnthropicTool` struct for Anthropic-native format (`name`, `description`, `input_schema`)
   - `tools` field on `ApiRequest` with `skip_serializing_if`
   - `ContentBlock` parsing extended for `type: "tool_use"` blocks
   - `complete_with_tools()` override including tool definitions and parsing tool call responses
   - `Role::Tool` translated to `user` role with `tool_result` content blocks
   - Assistant messages with `tool_calls` serialized as `tool_use` content blocks

6. **OpenAI Provider** (`synapse-core/src/provider/openai.rs`)
   - `OpenAiTool`, `OpenAiFunction`, `OpenAiToolCall`, `OpenAiToolCallFunction` types
   - `tools` field on request structs; streaming tool call delta accumulation
   - `complete_with_tools()` and `stream_with_tools()` overrides
   - `Role::Tool` mapped to `"tool"` with `tool_call_id` field
   - `finish_reason: "tool_calls"` handling in streaming

7. **DeepSeek Provider** (`synapse-core/src/provider/deepseek.rs`)
   - Same changes as OpenAI (OpenAI-compatible format) with private API types

8. **Mock Provider** (`synapse-core/src/provider/mock.rs`)
   - `with_tool_call_response()` builder for configuring mock tool call responses
   - `complete_with_tools()` override using same response queue
   - `Role::Tool` handling in match arms

9. **CLI One-Shot Mode** (`synapse-cli/src/main.rs`)
   - `init_mcp_client()` helper for MCP initialization with graceful degradation
   - `Agent` wraps provider and optional MCP client
   - `Role::Tool => "[TOOL]"` display arm for session history

10. **CLI REPL Mode** (`synapse-cli/src/repl.rs`)
    - Accepts optional `McpClient`, creates `Agent` wrapper
    - `Role::Tool => ("[TOOL]", Color::Magenta)` in `build_history_lines()`
    - Uses `agent.stream_owned()` for borrow-checker-friendly event loop

11. **Module Exports** (`synapse-core/src/lib.rs`)
    - `pub mod agent;`, `pub mod mcp;`
    - Exports: `Agent`, `AgentError`, `McpClient`, `McpConfig`, `McpError`, `McpServerConfig`, `ToolDefinition`, `ToolCallData`, `load_mcp_config`

---

## Key Decisions

### 1. Agent Orchestrator Instead of Modifying Provider Trait

**Decision:** Introduce a new `Agent` struct to coordinate the tool call loop rather than embedding tool call logic in the `LlmProvider` trait or individual providers.

**Rationale:** The detect-execute-return loop is a cross-cutting concern that spans the LLM provider, MCP client, and message history. An `Agent` struct cleanly separates this orchestration from the provider's responsibility (making API calls). This follows the hexagonal architecture: the agent is the application service layer coordinating between ports (provider and MCP client).

### 2. Non-Streaming Tool Call Iterations

**Decision:** Use `complete_with_tools()` (non-streaming) for all tool call iterations. Only the final text-only response uses streaming (or is yielded as a single `TextDelta`).

**Rationale:** Intermediate tool call iterations are machine-to-machine communication (LLM response with tool calls, tool execution, result injection). Streaming these adds complexity without user-facing benefit. The final response appears as a single block rather than token-by-token, which is a minor UX trade-off documented as acceptable for Phase 11.

### 3. Provider-Agnostic Tool Schema with Per-Provider Serialization

**Decision:** Define a single `ToolDefinition` struct in `synapse-core/src/mcp/protocol.rs` and have each provider convert it to its native API format.

**Rationale:** Anthropic and OpenAI have different tool definition formats (Anthropic uses `input_schema`, OpenAI wraps in `type: "function"` with `function.parameters`). A provider-agnostic type in core keeps the MCP layer decoupled from provider specifics. Each provider handles its own serialization.

### 4. Optional Fields for Backward Compatibility

**Decision:** Add tool-related fields to `Message` and `StoredMessage` as `Option<T>` types, defaulting to `None`.

**Rationale:** This ensures all existing code continues to compile and work without modification. `Message::new()` retains its existing signature. Database migration adds nullable columns. No data migration needed for existing messages.

### 5. Safety Limit on Tool Call Iterations

**Decision:** Cap the agent's tool call loop at 10 iterations, returning `AgentError::MaxIterationsExceeded` if exceeded.

**Rationale:** Prevents infinite loops where the LLM repeatedly requests tool calls. The limit is generous enough for realistic workflows (most tool call sequences complete in 1-3 iterations) while providing a safety net.

### 6. `rmcp` Crate for MCP Client

**Decision:** Use the `rmcp` crate (version 0.14) with `client`, `transport-child-process`, and `transport-io` features.

**Rationale:** `rmcp` is the official Rust SDK for the Model Context Protocol, providing the client-side `ServiceExt`, `TokioChildProcess` transport, `list_tools()`, and `call_tool()` APIs needed for stdio-based MCP server communication.

---

## Data Flow

### Tool Call Flow (One-Shot Mode)
```
1. User: synapse "List files in /tmp"
2. CLI loads Config, loads MCP config (mcp_servers.json)
3. McpClient::new() spawns MCP server, discovers tools via list_tools()
4. Agent::new(provider, Some(mcp_client))
5. Agent.stream(messages):
   a. provider.complete_with_tools([user_msg], [tool_definitions]) -> tool call response
   b. Agent appends assistant message with tool_calls to messages
   c. Agent calls mcp_client.call_tool("list_directory", {path: "/tmp"})
   d. Agent appends tool result message to messages
   e. provider.complete_with_tools([user, assistant+tc, tool_result], tools) -> text response
   f. Agent yields TextDelta(text), Done
6. CLI prints response, stores messages, shuts down MCP client
```

### Graceful Degradation (No MCP Config)
```
1. No mcp_servers.json exists
2. load_mcp_config() returns Ok(None)
3. Agent created with mcp_client = None
4. Agent.stream() delegates directly to provider.stream() (no tool awareness)
5. Behavior identical to pre-MCP implementation
```

---

## Testing

### New Tests (48 total)

| Category | Count |
|----------|-------|
| Data model (message, session) | 7 |
| MCP config and error | 8 |
| MCP client | 4 |
| Anthropic provider tool calling | 5 |
| OpenAI provider tool calling | 5 |
| DeepSeek provider tool calling | 7 |
| Mock provider extensions | 3 |
| Agent orchestrator | 6 |
| Storage tool round-trip | 2 |
| CLI/REPL integration | 1 |

**Full regression suite: 183 tests total (48 new + 135 pre-existing), 0 failures**

---

## Files Changed

| File | Change Type |
|------|-------------|
| `synapse-core/Cargo.toml` | Modified -- added `rmcp` dependency, `process` feature to `tokio` |
| `synapse-core/src/lib.rs` | Modified -- added `pub mod agent;`, `pub mod mcp;`, new exports |
| `synapse-core/src/message.rs` | Modified -- `Role::Tool`, `ToolCallData`, `Message` tool fields |
| `synapse-core/src/session.rs` | Modified -- `StoredMessage` with `tool_calls`, `tool_results` fields |
| `synapse-core/src/provider.rs` | Modified -- `complete_with_tools()`, `stream_with_tools()` on trait |
| `synapse-core/src/provider/anthropic.rs` | Modified -- tool calling support, `Role::Tool` translation |
| `synapse-core/src/provider/openai.rs` | Modified -- tool calling support, `Role::Tool` handling |
| `synapse-core/src/provider/deepseek.rs` | Modified -- tool calling support, `Role::Tool` handling |
| `synapse-core/src/provider/mock.rs` | Modified -- `with_tool_call_response()`, `complete_with_tools()` |
| `synapse-core/src/storage/sqlite.rs` | Modified -- `Role::Tool` parse/serialize, tool column bindings |
| `synapse-core/src/mcp.rs` | **New** -- `McpError`, `load_mcp_config()`, module declarations |
| `synapse-core/src/mcp/protocol.rs` | **New** -- `McpConfig`, `McpServerConfig`, `ToolDefinition` |
| `synapse-core/src/mcp/tools.rs` | **New** -- `McpClient`, tool registry, tool execution |
| `synapse-core/src/agent.rs` | **New** -- `Agent` orchestrator, `AgentError`, tool call loop |
| `synapse-core/migrations/20260208_002_add_tool_columns.sql` | **New** -- `ALTER TABLE` for tool columns |
| `synapse-cli/src/main.rs` | Modified -- MCP init, Agent creation, `Role::Tool` display |
| `synapse-cli/src/repl.rs` | Modified -- Agent integration, `Role::Tool` display |

---

## Module Structure

```
synapse-core/src/
  agent.rs                # Agent orchestrator, AgentError, tool call loop
  mcp.rs                  # McpError, load_mcp_config(), mod protocol; mod tools;
  mcp/
    protocol.rs           # McpConfig, McpServerConfig, ToolDefinition
    tools.rs              # McpClient, tool registry, tool execution
  message.rs              # Role::Tool, ToolCallData, Message tool fields
  session.rs              # StoredMessage tool_calls/tool_results fields
  provider.rs             # complete_with_tools(), stream_with_tools() on trait
  provider/
    anthropic.rs          # Anthropic tool calling (native tool_use format)
    openai.rs             # OpenAI tool calling (function format)
    deepseek.rs           # DeepSeek tool calling (OpenAI-compatible format)
    mock.rs               # with_tool_call_response() builder
  storage/
    sqlite.rs             # Role::Tool parsing, tool column bindings
```

---

## Known Limitations

1. **No end-to-end integration test with a real MCP server.** The `rmcp` transport layer (TokioChildProcess, stdio connect, list_tools, call_tool) is only exercised in production. All tests use mock/test helpers that bypass the transport.

2. **Intermediate tool call/result messages are not persisted.** When tool calls occur, the agent modifies the messages vector in-place, but intermediate tool call and tool result messages are not stored to the database. Session history shows only user and final assistant messages.

3. **Streaming with tools uses non-streaming fallback.** When tools are available, the final response is yielded as a single `TextDelta` rather than streamed token-by-token. This is a deliberate design decision for Phase 11 simplicity.

4. **`rmcp` version pinned to 0.14.** The crate is actively developed; API changes in future versions could require adaptation.

---

## Future Work

This implementation enables:
- **End-to-end integration tests** with a real MCP server (e.g., `@modelcontextprotocol/server-filesystem`)
- **Intermediate message persistence** for tool call traces in session history
- **True streaming for tool call conversations** -- stream the final response token-by-token instead of yielding a single TextDelta
- **MCP server health monitoring** -- detect crashed servers and optionally restart them
- **Tool call UI feedback** -- show tool call progress indicators in the REPL during tool execution
- **Additional MCP transports** -- HTTP/SSE transport for remote MCP servers (currently stdio only)
