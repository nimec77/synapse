# Implementation Plan: SY-2 - Add CI/CD

**Status: PLAN_APPROVED**

---

## Overview

This plan implements a CI/CD pipeline for the Synapse project using GitHub Actions. The pipeline will automate quality checks (formatting, linting, testing, security audit) on every push and pull request to ensure code integrity and maintain code standards.

The implementation follows the reference workflow from the `telegram-connector` project, adapted for the Synapse workspace structure.

---

## Components

### Files to Create

| File | Purpose |
|------|---------|
| `rust-toolchain.toml` | Pin Rust nightly toolchain for reproducibility |
| `.github/workflows/ci.yml` | CI workflow definition |

### Directory Structure After Implementation

```
synapse/
├── .github/
│   └── workflows/
│       └── ci.yml           # NEW
├── rust-toolchain.toml      # NEW
├── Cargo.toml
├── synapse-core/
├── synapse-cli/
└── synapse-telegram/
```

---

## Implementation Details

### 1. rust-toolchain.toml

Location: `/Users/comrade77/RustroverProjects/synapse/rust-toolchain.toml`

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy"]
```

**Rationale:**
- Project uses Rust Edition 2024, which requires nightly toolchain
- Ensures consistent toolchain across all developers and CI
- Includes rustfmt and clippy components needed for CI checks
- Uses floating "nightly" channel; can pin to specific date (e.g., `nightly-2025-01-10`) if stability issues arise

### 2. .github/workflows/ci.yml

Location: `/Users/comrade77/RustroverProjects/synapse/.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [master, "feature/*"]
  pull_request:
    branches: [master]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Build & Test
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt, clippy

      - name: Cache cargo dependencies
        uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --check

      - name: Run clippy
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Run security audit
        uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

**Design Decisions:**

1. **Two-job structure**: Main checks in one job (fast fail, cached), audit in separate job (runs in parallel).

2. **Triggers**:
   - Push to `master` and `feature/*` branches
   - Pull requests targeting `master`

3. **Steps order in check job**:
   - Format check first (fastest, fails early on style issues)
   - Clippy second (catches lint issues before running tests)
   - Tests last (most time-consuming)

4. **Caching**: `Swatinem/rust-cache@v2` caches cargo registry, git dependencies, and target directory for faster subsequent builds.

5. **Audit job separation**: Security audit runs independently and in parallel. This allows:
   - Main build to complete even if audit has issues
   - Clear visibility into security-specific failures
   - `GITHUB_TOKEN` is automatically provided by GitHub Actions

---

## Actions and Versions

| Action | Version | Purpose |
|--------|---------|---------|
| `actions/checkout` | v4 | Checkout repository code |
| `dtolnay/rust-toolchain` | nightly | Install Rust nightly toolchain |
| `Swatinem/rust-cache` | v2 | Cache cargo dependencies and build artifacts |
| `rustsec/audit-check` | v2 | Run cargo-audit for security vulnerabilities |

---

## CI Commands Mapping

The CI pipeline mirrors the pre-commit checks defined in `docs/conventions.md`:

| Pre-commit Check | CI Step |
|-----------------|---------|
| `cargo fmt --check` | Check formatting |
| `cargo clippy -- -D warnings` | Run clippy |
| `cargo test` | Run tests |
| `cargo audit` (from vision.md) | Security Audit job |

---

## NFR (Non-Functional Requirements)

### Performance

| Metric | Target | Expected |
|--------|--------|----------|
| CI pipeline total duration | < 10 minutes | < 2 minutes (current project size) |
| Format + clippy check duration | < 2 minutes | < 30 seconds |
| Test suite duration | < 5 minutes | < 10 seconds (no tests yet) |

### Maintainability

- Workflow file follows YAML best practices with clear step names
- Steps ordered by execution time (fastest first) for early failure detection
- Caching reduces build times as project grows
- Easy to extend with additional jobs (e.g., coverage, macOS builds) later

### Reliability

- Uses stable, well-maintained GitHub Actions
- `rust-toolchain.toml` ensures consistent toolchain
- No external service dependencies for basic CI
- No secrets required (GITHUB_TOKEN is automatic)

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Nightly toolchain breaks | Medium | High | Pin to specific nightly date in `rust-toolchain.toml` if issues occur |
| Audit finds vulnerability in future deps | Medium | Low | Use `continue-on-error: true` temporarily if needed for non-critical deps |
| Cache invalidation issues | Low | Low | `Swatinem/rust-cache` handles this automatically; can clear cache via GitHub UI |
| GitHub Actions outage | Low | Medium | No mitigation needed; GitHub SLA is > 99% |

---

## Alternatives Considered

### 1. Single Job vs Multi-Job

**Chosen**: Two jobs (check + audit)

**Alternative**: Single job with all steps

**Trade-off**: Two jobs allow parallel execution and independent failures. Single job would be simpler but slower overall and conflates build failures with security issues.

### 2. Platform Matrix (Linux + macOS + Windows)

**Chosen**: Linux only (ubuntu-latest)

**Alternative**: Multi-platform matrix

**Trade-off**: Linux-only is simpler and cheaper. The project has no platform-specific code yet. Can add macOS/Windows later if needed.

### 3. Pinned Nightly vs Floating Nightly

**Chosen**: Floating nightly (`channel = "nightly"`)

**Alternative**: Pinned nightly (`channel = "nightly-2025-01-10"`)

**Trade-off**: Floating nightly gets latest fixes but may break. Starting with floating; will pin if stability issues occur.

---

## Open Questions

None. All decisions have been made in the PRD:
- Platform matrix: Linux only
- Coverage reporting: No
- Release automation: No
- Nightly pinning: Via rust-toolchain.toml

---

## Implementation Checklist

- [ ] Create `rust-toolchain.toml` in project root
- [ ] Create `.github/workflows/` directory
- [ ] Create `.github/workflows/ci.yml` with workflow definition
- [ ] Verify locally: `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- [ ] Push to feature branch and verify CI runs successfully
- [ ] Verify all jobs pass in GitHub Actions UI
- [ ] Create PR to master and verify PR checks work

---

## Verification

After implementation, verify:

1. **Local check**: Run pre-commit commands locally to ensure they pass
2. **Push trigger**: Push to `feature/*` branch triggers CI
3. **PR trigger**: Opening PR to master triggers CI
4. **All jobs green**: Both `check` and `audit` jobs pass
5. **Failure detection**: Intentionally break code (e.g., remove semicolon) and verify CI fails
