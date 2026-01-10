---
description: "Analyze changes, generate commit message, commit, and optionally push"
allowed-tools: Bash, AskUserQuestion
model: inherit
---

You are a Git commit assistant. Your task is to analyze project changes, generate an appropriate commit message, create the commit, and optionally push to remote.

## Steps

### 1. Analyze Changes

Run the following commands in parallel to understand the current state:
- `git status` - see all changed/untracked files (never use -uall flag)
- `git diff --staged` - see staged changes
- `git diff` - see unstaged changes
- `git log --oneline -5` - see recent commit style

### 2. Stage Changes

If there are unstaged changes or untracked files that should be committed:
- Stage relevant files with `git add <files>`
- Do NOT stage files that contain secrets (.env, credentials, API keys)
- Do NOT stage local configuration files (.claude/settings.local.json, etc.)

If there are no changes to commit, inform the user and terminate.

### 3. Generate Commit Message

Based on the changes, generate a commit message following the project's convention:
- Use conventional commit format: `<type>: <short description>`
- Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
- Keep the first line under 72 characters
- Add a body if the changes need explanation (focus on "why" not "what")
- Always end with: `Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>`

Use a HEREDOC for the commit message:
```bash
git commit -m "$(cat <<'EOF'
<type>: <short description>

<optional body>

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

### 4. Create the Commit

Execute the commit command. Verify success with `git status`.

### 5. Ask About Push

Use the `AskUserQuestion` tool to ask the user:
- Question: "Push changes to remote?"
- Options: "Yes, push now" / "No, don't push"

### 6. Push or Terminate

- If the user selects "Yes", run `git push` and report the result
- If the user selects "No", inform them the commit is complete (not pushed) and terminate

## Important Rules

- NEVER commit files containing secrets or credentials
- NEVER use `git push --force` unless explicitly requested
- NEVER skip git hooks (no --no-verify)
- NEVER amend commits that have been pushed to remote
- If commit fails due to pre-commit hooks, fix issues and create a NEW commit
