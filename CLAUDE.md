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

## CI/CD

The project uses GitHub Actions for continuous integration. CI runs automatically on:
- Push to `master` and `feature/*` branches
- Pull requests targeting `master`

**CI Jobs:**
- `check`: Format check, Clippy lint, tests
- `audit`: Security vulnerability scanning (cargo-audit)

**Local pre-commit checks:**
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Code Conventions

### Module System
Use the **new Rust module system** (Rust 2018+). Do NOT use `mod.rs` files.

```
# Correct (new style)
src/
├── provider.rs        # declares: mod anthropic; mod openai;
└── provider/
    ├── anthropic.rs
    └── openai.rs

# Incorrect (old style) - DO NOT USE
src/
└── provider/
    ├── mod.rs         # ❌ Never use mod.rs
    ├── anthropic.rs
    └── openai.rs
```

## Key Technology Decisions

- **Rust**: Nightly, Edition 2024
- **LLM Providers**: Custom implementation (no rig/genai/async-openai) for learning depth
- **Async Runtime**: Tokio
- **Error Handling**: `thiserror` for library errors, `anyhow` for application errors
- **Configuration**: `toml` + `serde` for TOML parsing, `dirs` for cross-platform paths
- **Session Storage**: SQLite via `sqlx` (supports switching to PostgreSQL/MySQL)
- **Streaming**: SSE streaming via `eventsource-stream` + `async-stream`
- **MCP**: Model Context Protocol support via `rmcp`
- **CLI**: `clap` for args, `ratatui` for REPL UI

## Workspace Crates

| Crate | Purpose |
|-------|---------|
| `synapse-core` | Core library: config, agent, providers, storage, MCP |
| `synapse-cli` | CLI binary: REPL and one-shot modes |
| `synapse-telegram` | Telegram bot interface |

## Project Status

This project is in early development.

## Documentation

| Document | Purpose |
|----------|---------|
| `docs/idea.md` | Project concept and goals |
| `docs/vision.md` | Technical architecture and design decisions |
| `docs/conventions.md` | Code rules: DO and DON'T |
| `docs/tasklist.md` | Development plan with progress tracking |
| `docs/workflow.md` | Step-by-step collaboration process |
| `docs/phase/phase-*.md` | Phase-specific task breakdowns |
| `config.example.toml` | Example configuration file with documentation |
| `docs/prd/` | PRD documents for each ticket (e.g., `SY-1.prd.md`) |
| `docs/prd.template.md` | Template for creating new PRDs |
| `docs/research/` | Research documents for each ticket |
| `docs/plan/` | Implementation plans for each ticket |
| `docs/tasklist/` | Task breakdowns for each ticket |
| `docs/summary/` | Completion summaries for each ticket |
| `docs/.active_ticket` | Current active ticket identifier |
| `reports/qa/` | QA reports for each ticket |
| `CHANGELOG.md` | Project changelog |

## Workflow

**Starting a new feature (full automated workflow):**
```
/feature-development SY-<N> "Title" @docs/<description>.md
```
This runs the complete workflow: PRD → Research → Plan → Tasks → Implementation → Review → QA → Docs.

**Starting a new feature (manual):**
```
/analysis SY-<N> "Title" @docs/<description>.md
```
This creates a PRD in `docs/prd/SY-<N>.prd.md` and sets `docs/.active_ticket`.

**Before starting any task**, read these in order:
1. `docs/.active_ticket` — current ticket being worked on
2. `docs/prd/<ticket>.prd.md` — PRD for the current feature
3. `docs/tasklist.md` — find current phase and next task
4. `docs/vision.md` — understand relevant architecture
5. `docs/conventions.md` — rules to follow

**Follow `docs/workflow.md` strictly:**
1. **Propose** solution with code snippets → wait for approval
2. **Implement** → verify with `cargo check/test/clippy`
3. **Commit** → update `tasklist.md` → wait for confirmation
4. **Next task** → ask before proceeding

**Three mandatory checkpoints — never skip:**
- "Proceed with this approach?"
- "Ready to commit?"
- "Continue to next task?"

## Completed Tickets

| Ticket | Description | Summary |
|--------|-------------|---------|
| SY-1 | Project Foundation | Workspace structure with 3 crates |
| SY-2 | CI/CD Pipeline | GitHub Actions with check + audit jobs |
| SY-3 | Echo CLI | CLI with clap, one-shot and stdin input modes |
| SY-4 | Configuration | TOML config loading with multi-location priority |
| SY-5 | Provider Abstraction | LlmProvider trait, Message/Role types, MockProvider |
| SY-6 | Anthropic Provider | AnthropicProvider with Claude API, async CLI with tokio |
| SY-7 | DeepSeek Provider | DeepSeekProvider with OpenAI-compatible API, provider factory pattern |
| SY-8 | Streaming Responses | Token-by-token streaming via SSE, DeepSeekProvider streaming, Ctrl+C handling |
