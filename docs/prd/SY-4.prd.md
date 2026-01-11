# SY-4: Configuration System

Status: PRD_READY

## Context / Idea

Phase 3 of the Synapse project introduces a configuration system that allows loading settings from a TOML file. This is a foundational feature that enables the application to support multiple LLM providers, store API keys securely, and allow users to customize the agent's behavior without modifying code.

The configuration system aligns with the project's core functionality requirements outlined in the idea document:
- **Multi-provider LLM support**: Configuration enables specifying which provider to use
- **TOML-based configuration file**: Stores API keys, default provider, and user preferences
- **Customizable system prompts**: Foundation for future personalization

This phase builds on the completed Echo CLI (Phase 2) and prepares the application for the LLM provider integration (Phase 4).

### Phase 3 Scope

From `docs/phase/phase-3.md`:
- Goal: Load settings from TOML file
- Tasks:
  - 3.1 Create `synapse-core/src/config.rs` with `Config` struct
  - 3.2 Add `toml` + `serde` dependencies, implement TOML parsing
  - 3.3 Create `config.example.toml` in repo root
  - 3.4 Load config in CLI, print loaded provider name

## Goals

1. **Primary Goal**: Enable the Synapse application to read configuration from a TOML file, allowing users to specify their preferred LLM provider, API keys, and model settings.

2. **Secondary Goals**:
   - Establish a clean, extensible configuration module in `synapse-core`
   - Support multiple configuration file locations with sensible defaults
   - Provide a fallback mechanism when no configuration file exists
   - Create clear documentation via an example configuration file

## User Stories

### US-1: Basic Configuration Loading
**As a** developer using Synapse,
**I want to** configure my LLM provider settings in a TOML file,
**So that** I can easily switch between providers and manage API keys without modifying code.

### US-2: Default Configuration Path
**As a** user,
**I want to** place my configuration file in a standard location (`~/.config/synapse/config.toml`),
**So that** the application finds it automatically without specifying a path.

### US-3: Local Configuration Override
**As a** developer working on multiple projects,
**I want to** use a local `config.toml` in the current directory,
**So that** I can have project-specific settings that override the global configuration.

### US-4: Graceful Defaults
**As a** new user,
**I want to** run Synapse without creating a configuration file,
**So that** I can try the application with minimal setup (using defaults).

### US-5: Configuration Example
**As a** user setting up Synapse for the first time,
**I want to** reference an example configuration file,
**So that** I understand what options are available and their format.

## Main Scenarios

### Scenario 1: Load Configuration from Default Path
**Given** a configuration file exists at `~/.config/synapse/config.toml`
**When** the user runs `synapse "test"`
**Then** the application loads settings from that file and displays the configured provider name

### Scenario 2: Load Configuration from Local Path
**Given** a `config.toml` exists in the current working directory
**And** a configuration file also exists at `~/.config/synapse/config.toml`
**When** the user runs `synapse "test"`
**Then** the application uses the local `config.toml` (priority over global)

### Scenario 3: No Configuration File (Defaults)
**Given** no configuration file exists at any standard location
**When** the user runs `synapse "test"`
**Then** the application uses default values (e.g., provider = "anthropic")

### Scenario 4: Verify Configuration Loading
**Given** a `config.toml` with `provider = "anthropic"`
**When** the user runs `synapse "test"`
**Then** the output includes "Provider: anthropic"

## Success / Metrics

### Acceptance Criteria
1. Running `synapse "test"` with a config file containing `provider = "anthropic"` outputs text including "Provider: anthropic"
2. The `Config` struct is accessible from `synapse-core` public API
3. The `config.example.toml` file documents all available configuration options
4. Configuration loading works on macOS, Linux, and Windows

### Quality Metrics
- All new code passes `cargo clippy` without warnings
- All new code is formatted with `cargo fmt`
- Unit tests cover configuration parsing scenarios
- Configuration loading completes in under 10ms (per vision document targets)

## Constraints and Assumptions

### Constraints
1. **Rust Module Convention**: Must use the new Rust module system (no `mod.rs` files) as specified in `CLAUDE.md`
2. **Dependencies**: Use `toml` crate for parsing and `serde` for deserialization as specified in the vision document
3. **Location Priority**: Local config (`./config.toml`) takes precedence over global (`~/.config/synapse/config.toml`)
4. **Minimum Fields**: The initial `Config` struct must include at least: `provider`, `api_key`, `model`

### Assumptions
1. Phase 2 (Echo CLI) is complete and the basic CLI structure is in place
2. Users have write access to `~/.config/synapse/` or can create the directory
3. The configuration file uses valid TOML syntax
4. API keys will be stored in plain text (with a security warning in documentation)

## Risks

### R-1: Configuration File Permissions (Medium)
**Risk**: Users may set insecure permissions on config files containing API keys
**Mitigation**: Document proper permissions (chmod 600) in the example file; consider adding a runtime warning for open permissions (as outlined in vision document)

### R-2: Path Resolution on Windows (Low)
**Risk**: Different path conventions on Windows may cause issues
**Mitigation**: Use platform-appropriate path resolution (e.g., `dirs` crate or standard Rust path handling)

### R-3: Invalid TOML Syntax (Low)
**Risk**: Users may create malformed configuration files
**Mitigation**: Provide clear error messages indicating the parsing error and line number

### R-4: Missing Required Fields (Low)
**Risk**: Configuration file exists but lacks required fields
**Mitigation**: Use sensible defaults for all fields; only `api_key` should eventually be required for actual LLM usage (not in this phase)

## Open Questions

1. **Environment Variable Override**: Should we implement `SYNAPSE_CONFIG` environment variable support in this phase, or defer to a later phase? (Vision document mentions this)

2. **Validation Scope**: Should the configuration module validate provider names against a known list, or accept any string?

3. **Default Provider**: What should be the default provider when no config exists? The phase document suggests "anthropic" but this should be confirmed.

4. **API Key Handling**: Should the `api_key` field be `Option<String>` (optional for this phase since we are not yet calling LLM APIs), or should it be required?

5. **Additional Fields**: Should we include additional fields from the vision document's Configuration entity (e.g., `system_prompt`, `session_db_path`) in this phase, or keep the scope minimal?
