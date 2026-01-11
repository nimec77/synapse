# Research: SY-2 - Add CI/CD

## Overview

This document captures the technical research for ticket SY-2, which adds a CI/CD pipeline using GitHub Actions for the Synapse project.

---

## 1. Project Structure

### Workspace Layout

The Synapse project is a Rust workspace with three crates:

```
synapse/
├── Cargo.toml                     # Workspace manifest (resolver = "3")
├── rust-toolchain.toml            # NOT PRESENT - needs to be created
├── .github/
│   └── workflows/                 # NOT PRESENT - needs to be created
│       └── ci.yml
├── synapse-core/                  # Core library crate
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── synapse-cli/                   # CLI binary crate
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── synapse-telegram/              # Telegram bot crate
    ├── Cargo.toml
    └── src/
        └── main.rs
```

### Workspace Cargo.toml

Location: `/Users/comrade77/RustroverProjects/synapse/Cargo.toml`

```toml
[workspace]
resolver = "3"
members = [
    "synapse-core",
    "synapse-cli",
    "synapse-telegram",
]

[workspace.package]
edition = "2024"
rust-version = "1.85"
authors = ["Synapse Contributors"]
license = "MIT"
```

Key observations:
- **resolver = "3"**: Latest Cargo resolver, requires Rust 1.85+
- **edition = "2024"**: Requires nightly toolchain
- **rust-version = "1.85"**: Minimum supported Rust version

### Crate Binaries

| Crate | Binary Name | Type |
|-------|-------------|------|
| synapse-core | N/A | Library |
| synapse-cli | `synapse` | Binary |
| synapse-telegram | `synapse-telegram` | Binary |

---

## 2. Toolchain Configuration

### Current State

**No `rust-toolchain.toml` exists.** This file needs to be created for CI reproducibility.

### Required Configuration

Per PRD and `docs/vision.md`:
- **Rust Edition**: 2024
- **Toolchain**: Nightly (required for Edition 2024)
- **Minimum Rust Version**: 1.85

### Recommended rust-toolchain.toml

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy"]
```

This configuration:
- Ensures consistent toolchain across all developers and CI
- Includes rustfmt and clippy components needed for CI checks
- Uses "nightly" channel (not pinned to specific date for now)

### Risk: Nightly Stability

Nightly toolchain may have breaking changes. Mitigation options:
1. Pin to specific nightly date (e.g., `channel = "nightly-2025-01-10"`)
2. Use `rust-toolchain.toml` which GitHub Actions respects automatically

---

## 3. Build and Test Commands

### Pre-commit Checks (from docs/vision.md and docs/conventions.md)

```bash
# Format check
cargo fmt --check

# Lint with warnings as errors
cargo clippy -- -D warnings

# Run all tests
cargo test

# Security audit
cargo audit
```

### Full Build Commands

```bash
# Build entire workspace
cargo build

# Build in release mode
cargo build --release

# Run specific crate
cargo run -p synapse-cli
```

### Current Test State

The project is in early development (Phase 1 complete). Current crates contain:
- `synapse-core/src/lib.rs`: Placeholder module with a `hello()` function
- `synapse-cli/src/main.rs`: Prints "Synapse CLI"
- `synapse-telegram/src/main.rs`: Prints "Synapse Telegram Bot"

No tests exist yet. Running `cargo test` will succeed (0 tests).

---

## 4. Existing CI/CD Setup

**No CI/CD configuration exists.**

The `.github/workflows/` directory does not exist. This ticket will create:
1. `.github/workflows/` directory
2. CI workflow file (likely `ci.yml`)

---

## 5. Dependencies Analysis

### Current Dependencies

All three crates have **no dependencies** in Phase 1:

**synapse-core/Cargo.toml:**
```toml
[dependencies]
# No dependencies for Phase 1
```

**synapse-cli/Cargo.toml:**
```toml
[dependencies]
# No dependencies for Phase 1
```

**synapse-telegram/Cargo.toml:**
```toml
[dependencies]
# No dependencies for Phase 1
```

### CI Implications

1. **No external dependencies** = fast builds
2. **No `sqlx`** = no compile-time database checks needed
3. **No `openssl`** = no native library requirements
4. **No async runtime** = no tokio features to configure

### Future Dependencies (from docs/vision.md)

When these are added, CI may need updates:

| Dependency | CI Impact |
|------------|-----------|
| `sqlx` | May need `DATABASE_URL` for compile-time checks (or `sqlx-offline`) |
| `reqwest` | May need native TLS, usually works on ubuntu-latest |
| `tokio` | No CI impact |
| `teloxide` | May need Telegram bot token for integration tests |
| `rmcp` | No CI impact expected |

---

## 6. Reference Workflow Analysis

### telegram-connector CI (Reference)

From PRD:
```yaml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt, clippy
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
```

### Adaptations Needed for Synapse

1. **Toolchain**: Already uses nightly, matches our needs
2. **Workspace**: Commands work on workspaces automatically
3. **Security audit**: Need to add `cargo audit` step
4. **Caching**: Recommended for faster builds

---

## 7. Recommended CI Configuration

### Job Structure

Based on PRD and project requirements:

```yaml
name: CI

on:
  push:
    branches: [master, feature/*]
  pull_request:
    branches: [master]

jobs:
  fmt:
    name: Format Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt
      - run: cargo fmt --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy -- -D warnings

  test:
    name: Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - uses: Swatinem/rust-cache@v2
      - run: cargo test

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

### Alternative: Single Job (Faster for Small Projects)

For current project size, a single job may be more efficient:

```yaml
name: CI

on: [push, pull_request]

jobs:
  ci:
    name: Build & Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 8. GitHub Actions Components

### Actions to Use

| Action | Purpose | Version |
|--------|---------|---------|
| `actions/checkout@v4` | Checkout repository | v4 (latest) |
| `dtolnay/rust-toolchain@nightly` | Install Rust nightly | Uses rust-toolchain.toml if present |
| `Swatinem/rust-cache@v2` | Cache cargo dependencies | v2 (latest) |
| `rustsec/audit-check@v2` | Run cargo audit | v2 (latest) |

### Caching Strategy

`Swatinem/rust-cache@v2` caches:
- `~/.cargo/registry`
- `~/.cargo/git`
- `target/` directory

Cache key is based on:
- Cargo.lock (if present)
- Cargo.toml files
- rust-toolchain.toml

**Note**: `.gitignore` excludes `Cargo.lock`. For reproducible CI builds, consider removing `Cargo.lock` from `.gitignore` for library crates, or keeping it for binary crates.

---

## 9. Patterns from Project Conventions

### From docs/conventions.md

**Pre-commit (required):**
```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

**Additional from docs/vision.md:**
```bash
cargo audit
```

### CI Should Mirror Local Development

The CI pipeline should run the same checks developers run locally. This ensures:
1. No surprises in CI
2. Developers can run checks before pushing
3. Consistent quality gates

---

## 10. Technical Considerations

### Nightly Toolchain in CI

`dtolnay/rust-toolchain@nightly` will:
1. Check for `rust-toolchain.toml` and use it if present
2. Otherwise use latest nightly

Recommendation: Create `rust-toolchain.toml` for consistency.

### Platform Matrix

Per PRD decision: **Linux only (ubuntu-latest)**

No macOS or Windows builds needed. This keeps CI simple and cost-effective.

### Branch Triggers

Per PRD scenarios:
- Push to `feature/*` branches: Full CI
- Push to `master`: Full CI
- Pull requests to `master`: Full CI

### Secrets

No secrets needed for basic CI:
- Tests don't require API keys (mocked providers)
- `GITHUB_TOKEN` is automatically provided for `rustsec/audit-check`

---

## 11. Files to Create

| File | Purpose |
|------|---------|
| `.github/workflows/ci.yml` | CI workflow definition |
| `rust-toolchain.toml` | Toolchain pinning (recommended) |

### Directory Structure After Implementation

```
synapse/
├── .github/
│   └── workflows/
│       └── ci.yml
├── rust-toolchain.toml     # NEW
├── Cargo.toml
├── synapse-core/
├── synapse-cli/
└── synapse-telegram/
```

---

## 12. Performance Targets

From PRD:

| Metric | Target |
|--------|--------|
| CI pipeline total duration | < 10 minutes |
| Format + clippy check duration | < 2 minutes |
| Test suite duration | < 5 minutes |

Current expectation: With no dependencies, CI should complete in < 2 minutes total.

---

## 13. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Nightly toolchain breaks | Medium | High | Pin nightly in rust-toolchain.toml if issues arise |
| cargo audit fails on new vulnerabilities | Low | Low | Can be temporarily ignored for non-production deps |
| GitHub Actions rate limits | Low | Low | Cache dependencies to reduce API calls |
| Future dependencies break CI | Medium | Medium | Add CI checks when adding new dependencies |

---

## 14. Open Technical Questions

### Resolved by PRD Decisions

1. **Q: Use single job or multiple jobs?**
   A: PRD shows reference workflow with single job. Recommend starting with single job for simplicity, can split later.

2. **Q: Which platforms to support?**
   A: Linux only (ubuntu-latest) per PRD decision.

3. **Q: Pin nightly version?**
   A: PRD recommends pinning via rust-toolchain.toml. Start with `channel = "nightly"`, pin to specific date if instability occurs.

4. **Q: Include coverage reporting?**
   A: No, per PRD decision.

5. **Q: Include release automation?**
   A: No, per PRD decision. Manual releases only.

### Remaining Questions

1. **Cargo.lock handling**: Currently `.gitignore` excludes `Cargo.lock`. For reproducible CI builds, should it be committed?
   - Recommendation: Keep excluded for now (no dependencies), revisit when dependencies are added.

2. **Audit failures**: Should audit failures block PRs?
   - Recommendation: Yes, but can add `continue-on-error: true` if needed for legacy dependencies.

---

## 15. Recommendations

1. **Create rust-toolchain.toml first** - This ensures consistent toolchain in CI and local development.

2. **Start with simple single-job CI** - Current project is small; split jobs when build time increases.

3. **Use Swatinem/rust-cache** - Even with no dependencies, caches Rust toolchain artifacts.

4. **Keep audit as separate job** - Allows it to run in parallel and fail independently.

5. **Mirror pre-commit checks exactly** - CI should run same commands as local development.

6. **Test locally before pushing** - Run `cargo fmt --check && cargo clippy -- -D warnings && cargo test` before creating workflow.

---

## 16. Implementation Checklist

- [ ] Create `.github/workflows/` directory
- [ ] Create `rust-toolchain.toml` with nightly channel
- [ ] Create `.github/workflows/ci.yml` with:
  - [ ] Format check (cargo fmt --check)
  - [ ] Clippy (cargo clippy -- -D warnings)
  - [ ] Tests (cargo test)
  - [ ] Security audit (cargo audit)
- [ ] Verify workflow syntax with actionlint or GitHub UI
- [ ] Push to feature branch and verify CI runs
- [ ] Verify all checks pass
