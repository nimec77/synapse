# PROJECT: Synapse

## Primary Goal

Develop an AI agent in Rust that receives messages from users and responds to them. The project architecture consists of:

- **Shared code**: Core agent logic and common functions
- **Interface projects**: Separate implementations for different interaction methods
  - CLI application (initial implementation)
  - Telegram bot

## Core Functionality

- **Multi-provider LLM support**: Support for various LLM providers (DeepSeek, Anthropic Claude, OpenAI, etc.) configured via the config file, where the user specifies the provider's API endpoint and API key
- **Session-based conversation memory**: Maintain conversation context within a session for coherent multi-turn dialogues
- **Text-based I/O**: Simple text input and output for straightforward interaction
- **Configuration system**: TOML-based configuration file for storing API keys, default provider, and user preferences
- **Customizable system prompts**: Allow users to define the agent's personality and behavior through configurable system prompts
- **File-based logging**: Configurable log rotation for production deployments (Telegram bot)

## Target Platform(s)

- Backend Service
- CLI Client
- Telegram Bot

## Target Users

Personal use as a daily AI chat assistant. The primary user is a developer who wants a convenient way to interact with various LLM providers through a unified interface, either via command line or Telegram.

## Success Criteria

- **Learning Rust**: Successfully apply Rust concepts (ownership, lifetimes, async, error handling) in a practical project
- **Daily usability**: Create a tool that is genuinely useful for everyday interactions with AI assistants
- **Clean architecture**: Maintain a well-structured codebase with clear separation between core logic and interface implementations

## Constraints

- No specific constraints. Free to use appropriate libraries and tools as needed.
