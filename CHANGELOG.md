# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **SY-4: Configuration System** - TOML-based configuration loading:
  - `Config` struct with `provider`, `api_key`, and `model` fields
  - Priority-based config loading: `SYNAPSE_CONFIG` env var > `./config.toml` > `~/.config/synapse/config.toml` > defaults
  - Default values: provider = "deepseek", model = "deepseek-chat"
  - `ConfigError` with `IoError` and `ParseError` variants
  - CLI displays configured provider on startup
  - `config.example.toml` with documented options
  - Dependencies: toml, serde, dirs, thiserror in synapse-core
  - synapse-cli now depends on synapse-core

- **SY-3: Echo CLI** - CLI argument parsing with clap:
  - One-shot mode: `synapse "message"` prints `Echo: message`
  - Stdin mode: `echo "message" | synapse` reads from pipe
  - TTY detection shows help when no input provided
  - `--help` and `--version` flags

- **SY-2: CI/CD Pipeline** - GitHub Actions workflow for automated quality checks:
  - Format check (`cargo fmt --check`)
  - Linting with warnings as errors (`cargo clippy -- -D warnings`)
  - Test execution (`cargo test`)
  - Security audit via `rustsec/audit-check`
  - Dependency caching with `Swatinem/rust-cache`
  - Triggers on push to `master`/`feature/*` and PRs to `master`
  - `rust-toolchain.toml` for consistent nightly toolchain

- **SY-1: Project Foundation** - Established Rust workspace with three crates:
  - `synapse-core`: Core library for agent logic, providers, storage, and MCP
  - `synapse-cli`: CLI binary (executable: `synapse`)
  - `synapse-telegram`: Telegram bot binary
  - Configured for Rust Edition 2024 with resolver version 3
