# Development Conventions

Concise rules for Synapse development. Keep it simple.

---

## Code Style

- **Formatter**: `cargo fmt` (rustfmt) - run before every commit
- **Line length**: 100 characters max
- **Naming**:
  - `snake_case`: functions, variables, modules
  - `PascalCase`: types, traits, enums
  - `SCREAMING_SNAKE_CASE`: constants
- **Imports**: Group in order: `std` → external crates → internal modules, separated by blank lines
- **Module system**: New style only (no `mod.rs` files)
- **Comments**: Explain *why*, not *what*. Code should be self-documenting.

---

## Architecture Adherence

- **Hexagonal architecture**: Core depends on traits (ports), not implementations (adapters)
- **Dependency flow**: `interfaces → core → traits ← adapters`
- **Dependency injection**: Pass trait objects or generics, never construct adapters inside core
- **Module communication**: Via public trait methods only, no internal struct access
- **Forbidden patterns**:
  - No `unwrap()` or `expect()` in library code (use `?` operator)
  - No `mod.rs` files
  - No direct HTTP calls in core (use `LlmProvider` trait)
  - No blocking I/O in async contexts

---

## Testing Requirements

- **Unit test targets**: Config parsing, message serialization, session logic, error conversions
- **Mock strategy**: Use `mockall` for trait mocking; create `MockProvider` for LLM tests
- **Coverage threshold**: 80% minimum for `synapse-core`
- **Test naming**: `test_<function_name>_<scenario>` (e.g., `test_parse_config_missing_api_key`)
- **Test location**: Unit tests in same file (`#[cfg(test)]`), integration tests in `tests/`

---

## Dependency Management

- **Version format**: Caret `^x.y.z` (SemVer-compatible updates)
- **Updates**: Run `cargo update` weekly; review changelogs for minor versions
- **Security**: Run `cargo audit` before releases and in CI
- **Deprecated crates**: Replace within 2 weeks of deprecation notice
- **New dependencies**: Prefer well-maintained crates with >100 GitHub stars

---

## Documentation

- **Doc comments**: Required for all `pub` items (`///` with description)
- **Module docs**: Each module file starts with `//!` describing purpose
- **API format**: Include example in doc comment for complex functions
- **ADRs**: Document major decisions in `doc/adr/NNN-title.md` (when needed)
- **README**: Keep `README.md` updated with build/run instructions

---

## Performance & Security

- **Performance targets**:
  - First token streaming: < 500ms
  - Session load: < 50ms
  - Config load: < 10ms
- **Security rules**:
  - Never log API keys, tokens, or user message content at INFO or above
  - Config files must be `chmod 600` (warn if not)
  - HTTPS only for all external APIs
  - Validate all user input at system boundaries
- **Logging levels**: Use `tracing` macros; secrets only at TRACE level in dev

---

## Error Handling

- **Library errors**: `thiserror` with typed enums in `synapse-core`
- **Application errors**: `anyhow` in CLI/Telegram for ergonomic error chains
- **User-facing messages**: Clear, actionable (e.g., "API key not found. Set ANTHROPIC_API_KEY or add to config.toml")
- **Internal logging**: Log errors with context (session ID, provider, operation)
- **Recovery**: Implement retry with exponential backoff for transient network errors

---

## Git & Version Control

- **Commit format**:
  ```
  <type>: <short description>

  <optional body>
  ```
  Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

- **Branch naming**:
  - Features: `feature/<name>`
  - Fixes: `fix/<issue-or-description>`
  - Main branch: `master`

- **PR checklist**:
  - [ ] `cargo fmt --check` passes
  - [ ] `cargo clippy -- -D warnings` passes
  - [ ] `cargo test` passes
  - [ ] New public items have doc comments
  - [ ] No `unwrap()` in library code

- **Merge strategy**: Squash merge to keep history clean

---

## Pre-commit Checklist

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo audit
```

All must pass before pushing.
