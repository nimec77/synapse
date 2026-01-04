# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Synapse is a Rust-based AI agent that serves as a unified interface to interact with multiple LLM providers (Anthropic Claude, OpenAI, etc.). The project targets multiple interfaces: CLI (primary), Telegram bot, and backend service.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│           Interface Layer (CLI/Telegram)            │
├─────────────────────────────────────────────────────┤
│         Core Agent & Shared Library                 │
├─────────────────────────────────────────────────────┤
│  LLM Provider Abstraction (Claude, OpenAI, etc.)    │
├─────────────────────────────────────────────────────┤
│  Session Memory & Configuration Management          │
└─────────────────────────────────────────────────────┘
```

**Key design principles:**
- Shared core library with common agent logic
- Separate interface implementations that consume the core library
- Provider-agnostic LLM abstraction layer
- TOML-based configuration for API keys, provider selection, and system prompts
- Session-based conversation memory for multi-turn dialogues

## Build Commands

```bash
# Build the entire workspace
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Run tests for a specific crate
cargo test -p crate_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Project Status

This project is in early development. See `doc/idea.md` for the full project specification and goals.
