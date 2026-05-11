---
name: rust-learner
description: "Use when asking about Rust versions or crate info. Keywords: latest version, what's new, changelog, Rust 1.x, stable, nightly, crate info, crates.io, lib.rs, docs.rs, API documentation, crate features, dependencies, which crate, what version, Rust edition, edition 2021, edition 2024, cargo add, cargo update"
allowed-tools: WebFetch, mcp__Ref__ref_search_documentation, mcp__Ref__ref_read_url, Bash, Read
---

# Rust Learner

> **Version:** 3.0.0 | **Last Updated:** 2026-05-10

Fetches authoritative Rust and crate information instead of guessing. Use when the user asks for the latest crate version, an unfamiliar API, what changed in a Rust release, or which lint applies.

## Tool priority

1. **`mcp__Ref__ref_search_documentation`** + **`mcp__Ref__ref_read_url`** — primary. The Ref MCP server is configured for this project (see project's `.mcp.json` / system MCP instructions) and returns structured docs lookups for Rust and crates.
2. **`WebFetch`** — fallback when Ref doesn't have the page or returns empty. Construct URLs from the table below.
3. **`Bash`** — for `cargo` commands when the user wants info about the local project (`cargo tree`, `cargo metadata`, `cargo --version`).

Do NOT use `WebSearch` for crate info — results are unstructured and frequently outdated.

## Query routing

| Question | Approach |
|---|---|
| "What's the latest version of <crate>?" | Ref search for `<crate> crates.io`, then `WebFetch https://lib.rs/crates/<crate>`. |
| "What's new in Rust 1.<X>?" | `WebFetch https://releases.rs/docs/1.<X>.0/` |
| "How do I use std::<path>::<Name>?" | `WebFetch` the canonical URL (table below). |
| "Tokio API for X" / docs for any crate | Ref search for `<crate> <symbol>` first, fall back to `WebFetch https://docs.rs/<crate>/latest/<crate>/<path>`. |
| "Clippy lint <name>?" | `WebFetch https://rust-lang.github.io/rust-clippy/master/index.html#<lint>` |
| "Which version is in our project?" | `Bash`: `cargo tree -p <crate> --depth 0` or read `Cargo.lock`. |

## URL templates

| Resource | URL |
|---|---|
| crate page (lib.rs) | `https://lib.rs/crates/<crate>` |
| crate docs (latest) | `https://docs.rs/<crate>/latest/<crate>/` |
| crate docs (specific version) | `https://docs.rs/<crate>/<version>/<crate>/` |
| std trait | `https://doc.rust-lang.org/std/<module>/trait.<Name>.html` |
| std struct | `https://doc.rust-lang.org/std/<module>/struct.<Name>.html` |
| std module index | `https://doc.rust-lang.org/std/<module>/index.html` |
| Rust release notes | `https://releases.rs/docs/1.<X>.0/` |
| Clippy lints | `https://rust-lang.github.io/rust-clippy/master/index.html#<lint>` |

Common std paths: `Send`/`Sync`/`Copy`/`Clone` → `std/marker/trait.<Name>.html` · `Arc`/`Mutex`/`RwLock` → `std/sync/struct.<Name>.html` · `Rc` → `std/rc/struct.Rc.html` · `Box` → `std/boxed/struct.Box.html` · `Vec` → `std/vec/struct.Vec.html` · `String` → `std/string/struct.String.html`.

## Output format

Keep answers tight; the user almost always wants the version + a one-line summary plus the source link.

**Crate version:**
```
<crate> <version> (released <date if visible>) — <one-line description>
docs.rs/<crate>/latest · lib.rs/crates/<crate>
```

**Rust release:**
```
Rust 1.<X> (<date>)
- <feature>
- <feature>
Source: releases.rs/docs/1.<X>.0/
```

**API lookup:**
```
<crate>::<path>::<Name>

<signature>

<one-paragraph description>

Source: docs.rs/<crate>/latest/<crate>/<path>
```

## Failure modes

| Symptom | Likely cause | Action |
|---|---|---|
| Ref returns no results | crate not indexed by Ref, or query too specific | Fall back to `WebFetch`. |
| `WebFetch` returns empty / 404 | Wrong URL pattern (e.g. `_` vs `-` in crate name) | Try the alternative; crate names can be `serde_json` (URL) vs `serde-json` — check crates.io. |
| Multiple matching crates | Ambiguous query | Show top 3 results from Ref or lib.rs and ask the user which one. |
| Local crate version differs from latest | Project pins an older version | Report both ("project uses X, latest is Y"). |
