-- Add tool-related columns to messages table for MCP integration.
-- tool_calls: JSON-serialized tool call data (for assistant messages requesting tool use).
-- tool_results: JSON-serialized tool result data (for tool response messages).
ALTER TABLE messages ADD COLUMN tool_calls TEXT;
ALTER TABLE messages ADD COLUMN tool_results TEXT;
