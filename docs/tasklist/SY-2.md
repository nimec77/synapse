# Tasklist: SY-2 - Add CI/CD

Status: TASKLIST_READY

## Context

Implement a CI/CD pipeline for the Synapse project using GitHub Actions. The pipeline will automate quality checks (formatting, linting, testing, security audit) on every push and pull request to ensure code integrity and maintain code standards.

The implementation follows the reference workflow from the `telegram-connector` project, adapted for the Synapse workspace structure.

---

## Tasks

### 2.1 Create rust-toolchain.toml

- [x] Create `rust-toolchain.toml` at project root with nightly toolchain and required components

**Acceptance Criteria:**
- File `rust-toolchain.toml` exists at project root
- Contains `[toolchain]` section with `channel = "nightly"` and `components = ["rustfmt", "clippy"]`
- Running `rustup show` in the project directory shows nightly toolchain is selected

---

### 2.2 Create .github/workflows directory

- [x] Create `.github/workflows/` directory structure

**Acceptance Criteria:**
- Directory `.github/workflows/` exists
- Directory is tracked by git (not ignored)

---

### 2.3 Create ci.yml workflow file

- [x] Create `.github/workflows/ci.yml` with the CI workflow definition

**Acceptance Criteria:**
- File `.github/workflows/ci.yml` exists
- Contains `name: CI` at the top
- Triggers on push to `master` and `feature/*` branches
- Triggers on pull requests targeting `master`
- Contains `check` job with: checkout, toolchain install, cache, fmt check, clippy, tests
- Contains `audit` job with: checkout, security audit using `rustsec/audit-check@v2`
- Uses `Swatinem/rust-cache@v2` for dependency caching
- Clippy runs with `-D warnings` to treat warnings as errors

---

### 2.4 Verify CI commands locally

- [x] Run `cargo fmt --check` and confirm it passes
- [x] Run `cargo clippy -- -D warnings` and confirm it passes
- [x] Run `cargo test` and confirm it passes

**Acceptance Criteria:**
- `cargo fmt --check` exits with code 0
- `cargo clippy -- -D warnings` exits with code 0
- `cargo test` exits with code 0
- All commands complete without errors or warnings

---

### 2.5 Verify CI workflow on GitHub

- [x] Push changes to feature branch
- [x] Confirm CI workflow triggers and runs
- [ ] Confirm both `check` and `audit` jobs pass

**Acceptance Criteria:**
- GitHub Actions shows CI workflow triggered on push
- `check` job (Build & Test) shows green checkmark
- `audit` job (Security Audit) shows green checkmark
- All individual steps within jobs complete successfully

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 2.1 | Toolchain configuration | `rust-toolchain.toml` |
| 2.2 | Workflows directory | `.github/workflows/` |
| 2.3 | CI workflow definition | `.github/workflows/ci.yml` |
| 2.4 | Local verification | N/A (verification commands) |
| 2.5 | GitHub verification | N/A (GitHub Actions UI) |
