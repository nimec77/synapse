---
description: "Bump workspace version, update CHANGELOG, commit, and tag a release"
argument-hint: "<patch|minor|major>"
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
model: sonnet
---

You are executing a structured release workflow for the Synapse workspace. Follow these steps exactly in order. Do not skip steps. Abort with a clear error message if any step fails.

## EXECUTION CONTRACT

Execute all steps 1–12 sequentially. The workflow is complete when you print the Step 12 summary. Do not stop early.

---

## Step 1: Parse and validate argument

Read the argument passed to this skill: `$ARGUMENTS`

- Must be exactly one of: `patch`, `minor`, `major`
- If missing or invalid: print `Error: argument must be patch, minor, or major` and stop.

---

## Step 2: Read current version

Read the file `/Users/comrade77/RustroverProjects/synapse/Cargo.toml` and extract the `version` field from the `[workspace.package]` section.

Expected format: `version = "X.Y.Z"`

If not found, print `Error: could not find version in [workspace.package]` and stop.

---

## Step 3: Compute new version

Apply the bump to the current version:
- `major`: increment X, reset Y and Z to 0 → `(X+1).0.0`
- `minor`: increment Y, reset Z to 0 → `X.(Y+1).0`
- `patch`: increment Z → `X.Y.(Z+1)`

Store both `CURRENT_VERSION` and `NEW_VERSION` for use in later steps.

---

## Step 4: Pre-release checks

Run in the workspace root (`/Users/comrade77/RustroverProjects/synapse`):

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

If any command fails, print `Error: pre-release checks failed — fix issues before releasing` and stop.

---

## Step 5: Check git clean

Run:
```bash
git -C /Users/comrade77/RustroverProjects/synapse status --porcelain
```

If output is non-empty, print `Error: working tree is dirty — commit or stash changes before releasing` and stop.

---

## Step 6: Check CHANGELOG has content

Read `/Users/comrade77/RustroverProjects/synapse/CHANGELOG.md`.

Find the `## [Unreleased]` section. There must be at least one non-empty, non-heading line between `## [Unreleased]` and the next `## [` heading (or end of file).

If the section is empty, print `Error: [Unreleased] section in CHANGELOG.md is empty — add entries before releasing` and stop.

---

## Step 7: Update version in root Cargo.toml

Edit `/Users/comrade77/RustroverProjects/synapse/Cargo.toml`:
- Replace `version = "CURRENT_VERSION"` with `version = "NEW_VERSION"` in the `[workspace.package]` section.

---

## Step 8: Regenerate Cargo.lock

Run:
```bash
cargo check --manifest-path /Users/comrade77/RustroverProjects/synapse/Cargo.toml
```

This updates `Cargo.lock` to reflect the new version.

---

## Step 9: Update CHANGELOG.md

Read the current CHANGELOG.md content. Get today's date in `YYYY-MM-DD` format by running:
```bash
date +%Y-%m-%d
```

Transform the CHANGELOG:
1. Find the `## [Unreleased]` header and the content that follows it (up to but not including the next `## [` heading).
2. Replace the entire `## [Unreleased]` block with:
   - A new empty `## [Unreleased]` section (just the header with a blank line after it)
   - Followed by `## [NEW_VERSION] - YYYY-MM-DD` with the original unreleased content beneath it.

The result should look like:
```
## [Unreleased]

## [NEW_VERSION] - YYYY-MM-DD

<original unreleased content>

## [PREV_VERSION] - ...
```

Write the updated content back to CHANGELOG.md.

---

## Step 10: Commit

Run:
```bash
git -C /Users/comrade77/RustroverProjects/synapse add Cargo.toml Cargo.lock CHANGELOG.md
git -C /Users/comrade77/RustroverProjects/synapse commit -m "chore: release vNEW_VERSION"
```

(Substitute the actual NEW_VERSION value.)

If the commit fails, print the error and stop.

---

## Step 11: Tag

Run:
```bash
git -C /Users/comrade77/RustroverProjects/synapse tag -a "vNEW_VERSION" -m "Release vNEW_VERSION"
```

(Substitute the actual NEW_VERSION value.)

If tagging fails, print the error and stop.

---

## Step 12: Print summary

Print a summary in this format:

```
Release complete!

  CURRENT_VERSION → NEW_VERSION
  Tag:     vNEW_VERSION
  Commit:  <git log --oneline -1 output>
  Files:   Cargo.toml, Cargo.lock, CHANGELOG.md

Next steps:
  git push && git push --tags
```

**WORKFLOW COMPLETE**
