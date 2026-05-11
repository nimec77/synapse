-- SY-22: Add reasoning_content column to store chain-of-thought from reasoning models.
-- Nullable TEXT with no default — existing rows load as NULL (-> None in Rust).
-- Mirrors the style of 20260208_002_add_tool_columns.sql.
ALTER TABLE messages ADD COLUMN reasoning_content TEXT;
