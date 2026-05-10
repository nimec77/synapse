---
name: rust-refactor-helper
description: "Use when refactoring Rust code — renaming symbols, moving functions, or extracting code — and a safe LSP-driven impact analysis is needed before applying changes"
argument-hint: "<action> <target> [--dry-run]"
allowed-tools: LSP, Read, Glob, Grep, Edit, Bash
---

# Rust Refactor Helper

> **Version:** 2.0.0 | **Last Updated:** 2026-05-10

Performs LSP-driven refactors on Rust code. Always runs an impact analysis first; applies the change only when the user confirms (or `--dry-run` is omitted on the second pass).

## Usage

```
/rust-refactor-helper <action> <target> [--dry-run]
```

| Action | Purpose |
|---|---|
| `rename <old> <new>` | Rename a symbol everywhere it's referenced. |
| `extract-fn <file>:<start>-<end>` | Lift the selected lines into a new function. |
| `inline <fn>` | Replace every call to `<fn>` with its body, then delete the function. |
| `move <symbol> <dest>` | Move a struct/fn/impl to another module path. |

Examples:
- `/rust-refactor-helper rename parse_config load_config`
- `/rust-refactor-helper extract-fn src/main.rs:20-35`
- `/rust-refactor-helper move UserService src/services/`

## Required tooling

- The `rust-analyzer` LSP server must be running for the workspace. If `LSP` calls fail with "no workspace initialized" or "server not running", instruct the user to open the project in their IDE (or start `rust-analyzer` headless) and retry — this skill cannot operate without it.
- `Bash` is allowed for git status / `cargo check` smoke tests after applying a change.

## LSP operations

```
LSP(operation: "goToDefinition", filePath: "...", line: N, character: N)
LSP(operation: "findReferences", filePath: "...", line: N, character: N)
LSP(operation: "hover", filePath: "...", line: N, character: N)
LSP(operation: "incomingCalls", filePath: "...", line: N, character: N)
```

## Workflows

### 1. Rename symbol

```
1. goToDefinition → confirm the target symbol's canonical location.
2. findReferences → enumerate every callsite, import, and re-export.
3. Categorize hits: definition / call / import / re-export / test / doc-comment / macro-generated.
4. Conflict check: does the new name already exist in any of the touched scopes?
5. Print impact table (file · line · context · change-kind).
6. If --dry-run, stop. Otherwise apply via Edit (one Edit per file).
7. After applying, run `cargo check` to catch anything LSP missed (e.g. macro expansions).
```

Impact table format:

```
## Rename: parse_config → load_config

**Definition:** src/config.rs:25
**References found:** 8

| File | Line | Context | Change |
|------|------|---------|--------|
| src/config.rs | 25 | `pub fn parse_config(` | Definition |
| src/config.rs | 45 | `parse_config(path)?` | Call |
| src/lib.rs | 8 | `pub use config::parse_config` | Re-export (public API change) |
| ... |

**Potential issues**
⚠️ src/lib.rs:8 — re-export means this is a public API change.
⚠️ docs/api.md:42 — documentation reference; LSP cannot fix.
```

### 2. Extract function

```
1. Read the selected line range.
2. Variable analysis:
   - Inputs: read-but-not-defined inside the block.
   - Outputs: defined-and-used-after the block.
   - Locals: defined-and-used only inside.
3. Determine the function signature (inputs as params, outputs as return tuple/struct).
4. Inspect for early returns, `?`, loops, breaks — these constrain extraction.
5. Generate the function and the replacing call site.
6. If --dry-run, stop. Otherwise apply via Edit and run `cargo check`.
```

### 3. Move symbol

```
1. findReferences on the symbol to enumerate every importer.
2. Identify dependencies (other symbols defined alongside the target that must move with it).
3. Compute the new module path; check for circular deps via the import graph.
4. Generate the import-rewrite plan: every old `use ...::Target` → `use new::path::Target`.
5. Print the plan + new file structure.
6. If --dry-run, stop. Otherwise apply (create destination file, move code, rewrite imports), then `cargo check`.
```

### 4. Inline function

```
1. findReferences on <fn>.
2. For each callsite, fetch the function body and substitute (rename locals to avoid shadowing).
3. Delete the function definition.
4. If --dry-run, stop. Otherwise apply.
```

## Safety checks

| Check | What it catches |
|---|---|
| Reference completeness | LSP missed a hit (macro-generated, `#[cfg]`-gated, dead code) |
| Name conflicts | New name shadows or collides with an existing symbol |
| Visibility changes | `pub`/`pub(crate)`/private scope shifts unexpectedly |
| Macro-generated code | LSP doesn't see inside macros — flag and warn |
| Documentation references | Doc comments mentioning the symbol need manual update |
| Test coverage | Tests touching the symbol (rename or move them too) |

## Dry-run is the default mental model

For destructive refactors, run with `--dry-run` first. The impact table tells you what will happen; only re-run without `--dry-run` once you've reviewed it.

## Failure modes

| Symptom | Cause | Action |
|---|---|---|
| `LSP server not running` | rust-analyzer not started | Tell the user to open the project in an IDE or start `rust-analyzer` headless. |
| `findReferences` returns very few hits | Stale index | Trigger a save in any `.rs` file or restart the LSP, then retry. |
| Rename conflicts after apply | Macro-expanded usage missed | The follow-up `cargo check` flags it; fix manually with `Edit`. |
| Circular dependency on move | Target depends on the destination module | Refactor the dependency first, then re-run move. |
