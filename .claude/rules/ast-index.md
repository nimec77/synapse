# /Applications/ast-index Rules

## Mandatory Search Rules

1. **ALWAYS use /Applications/ast-index FIRST** for any code search task
2. **NEVER duplicate results** — if /Applications/ast-index found usages/implementations, that IS the complete answer
3. **DO NOT run grep "for completeness"** after /Applications/ast-index returns results
4. **Use grep/Search ONLY when:**
   - /Applications/ast-index returns empty results
   - Searching for regex patterns (/Applications/ast-index uses literal match)
   - Searching for string literals inside code (`"some text"`)
   - Searching in comments content

## Why /Applications/ast-index

/Applications/ast-index is 17-69x faster than grep (1-10ms vs 200ms-3s) and returns structured, accurate results.

## Command Reference

| Task | Command | Time |
|------|---------|------|
| Universal search | `/Applications/ast-index search "query"` | ~10ms |
| Find struct/trait | `/Applications/ast-index class "StructName"` | ~1ms |
| Find symbol | `/Applications/ast-index symbol "SymbolName"` | ~1ms |
| Find usages | `/Applications/ast-index usages "SymbolName"` | ~8ms |
| Find implementations | `/Applications/ast-index implementations "Trait"` | ~5ms |
| Call hierarchy | `/Applications/ast-index call-tree "function" --depth 3` | ~1s |
| Find callers | `/Applications/ast-index callers "functionName"` | ~1s |
| Module deps | `/Applications/ast-index deps "module-name"` | ~10ms |
| File outline | `/Applications/ast-index outline "lib.rs"` | ~1ms |

## Rust-Specific Commands

| Task | Command |
|------|---------|
| Find structs | `/Applications/ast-index class "User"` |
| Find traits | `/Applications/ast-index class "Repository"` |
| Find impl blocks | `/Applications/ast-index search "impl"` |
| Find macros | `/Applications/ast-index search "macro_rules"` |
| Find derives | `/Applications/ast-index search "#[derive"` |
| Find tests | `/Applications/ast-index search "#[test]"` |

## Index Management

- `/Applications/ast-index rebuild` — Full reindex (run once after clone)
- `/Applications/ast-index update` — After git pull/merge
- `/Applications/ast-index stats` — Show index statistics
