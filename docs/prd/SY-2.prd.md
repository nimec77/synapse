# SY-2: Add CI/CD

Status: PRD_READY

## Context / Idea

The Synapse project needs a CI/CD pipeline to automate quality checks and ensure code integrity on every push and pull request. The pipeline should be implemented as a GitHub Actions workflow.

The idea is to create a CI/CD configuration similar to the one used in the `telegram-connector` project (https://github.com/nimec77/telegram-connector). This ensures consistency across the maintainer's projects.

**Project specifics:**
- Rust workspace with 3 crates: `synapse-core`, `synapse-cli`, `synapse-telegram`
- Uses Rust Edition 2024 with nightly toolchain (rust-version = "1.85")
- Workspace resolver version 3
- Pre-commit checks defined in `docs/vision.md`: fmt, clippy, test, audit

## Goals

1. **Automate quality gates**: Run formatting, linting, and tests automatically on every push and PR
2. **Catch issues early**: Prevent broken code from being merged to master
3. **Enforce code standards**: Ensure consistent code style across the codebase
4. **Security scanning**: Include dependency vulnerability checks
5. **Cross-platform validation**: Build and test on relevant platforms (Linux primary, optionally macOS)

## User Stories

1. **As a developer**, I want automated checks to run when I push code, so I can catch issues before they are merged.

2. **As a developer**, I want the CI to fail fast on formatting issues, so I do not waste time waiting for longer tests to complete.

3. **As a maintainer**, I want pull requests to require passing CI checks, so code quality is maintained.

4. **As a developer**, I want clear feedback on what failed in CI, so I can fix issues quickly.

5. **As a maintainer**, I want security vulnerability scanning, so I am alerted to dependency issues.

## Main Scenarios

### Scenario 1: Push to feature branch
1. Developer pushes commits to `feature/*` branch
2. CI triggers automatically
3. Format check runs first (fast fail)
4. Clippy runs with warnings as errors
5. Tests run for all workspace crates
6. Security audit runs
7. Developer receives pass/fail notification

### Scenario 2: Pull Request to master
1. Developer opens PR from feature branch to master
2. All CI checks run
3. PR status shows check results
4. Merge is blocked until all checks pass
5. After passing, PR can be merged

### Scenario 3: Direct push to master
1. Maintainer pushes directly to master (emergency fix)
2. CI runs all checks
3. Failure is visible in commit status
4. Alerts maintainer to fix issues

## Success / Metrics

| Metric | Target |
|--------|--------|
| CI pipeline total duration | < 10 minutes |
| Format + clippy check duration | < 2 minutes |
| Test suite duration | < 5 minutes |
| False positive rate | 0% (no flaky tests) |
| CI availability | > 99% (GitHub Actions SLA) |

## Constraints and Assumptions

### Constraints
- Must use GitHub Actions (project hosted on GitHub)
- Must support Rust nightly toolchain (Edition 2024 requirement)
- Must work with workspace structure (multiple crates)
- Should minimize CI minutes usage (cost consideration)

### Assumptions
- Tests do not require external services (mocked providers)
- Nightly Rust toolchain is available in GitHub Actions
- No secrets are needed for basic CI (API keys only for integration tests, which can be skipped in CI)

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Nightly toolchain instability | Medium | High | Pin to specific nightly date or use `rust-toolchain.toml` |
| Long CI times | Low | Medium | Use caching for cargo dependencies |
| Flaky tests | Low | Medium | Ensure tests are deterministic, no real API calls |
| GitHub Actions rate limits | Low | Low | Use appropriate triggers (not on every commit) |

## Decisions

1. **Reference workflow**: Use `telegram-connector` CI as base, adapted for this project's workspace structure
2. **Platform matrix**: Linux only (ubuntu-latest)
3. **Coverage reporting**: No coverage reporting
4. **Release automation**: No release automation (manual releases)
5. **Nightly pinning**: Yes, pin nightly version via `rust-toolchain.toml` for reproducibility

## Technical Notes

### Reference Workflow (telegram-connector)

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

### Recommended CI Jobs

Based on `docs/vision.md` pre-commit checks and reference workflow:

```yaml
jobs:
  fmt:
    - cargo fmt --check

  clippy:
    - cargo clippy -- -D warnings

  test:
    - cargo test

  audit:
    - cargo audit
```

### Toolchain Configuration

The project uses:
- Rust Edition 2024
- rust-version = "1.85"
- Workspace resolver = "3"

This requires nightly toolchain. A `rust-toolchain.toml` file may be beneficial:

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy"]
```

### Caching Strategy

Use `actions/cache` or `Swatinem/rust-cache` to cache:
- `~/.cargo/registry`
- `~/.cargo/git`
- `target/` directory

This can reduce CI time significantly on subsequent runs.
