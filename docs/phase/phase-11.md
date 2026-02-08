# Phase 11: MCP Integration

**Goal:** Tool calling via Model Context Protocol.

## Tasks

- [x] 11.1 Add `rmcp` dependency to core
- [x] 11.2 Create `synapse-core/src/mcp.rs` with `McpClient` struct
- [x] 11.3 Load MCP server configs from `mcp_servers.json`
- [x] 11.4 Implement tool discovery and registration
- [x] 11.5 Handle tool calls in agent loop: detect → execute → return result

## Acceptance Criteria

**Test:** Configure a simple MCP server, ask the LLM to use it.

## Dependencies

- Phase 10 complete (OpenAI Provider)

## Implementation Notes

