---
name: fix
description: Fix a GitHub issue — read, implement on the in-flight branch, check, PR. No per-task topic branches.
model: opus
user-invocable: true
argument-hint: [issue-number]
allowed-tools: Bash, Read, Edit, Write, Grep, Glob
---
# Fix #$ARGUMENTS

## Policy

This is a single-developer workspace. There are no per-task topic branches. The fix lands on the in-flight `next` branch (or `release/<version>` during a release window), like every other piece of work. See `~/dev/CLAUDE.md` § "Git Workflow (single-developer policy)".

## Steps

1. **Read the issue:**
   ```bash
   gh issue view $ARGUMENTS
   ```

2. **Make sure you're on the right branch.** If on `main`, run `/branch` first to land on `next`. Don't auto-cut a `fix/<issue>` branch.
   ```bash
   git status --short --branch
   ```

3. **Explore relevant code** to understand the bug or gap. Read existing patterns before writing new code.

4. **Implement the minimum fix.** Follow existing patterns. Don't expand scope beyond what closes the issue.

5. **Verify:**
   ```bash
   just lint     # or bun run check
   just test     # or cargo test --workspace
   ```

6. **Commit on the current branch:**
   ```bash
   git commit -m "fix #$ARGUMENTS: <description>"
   ```
   The commit message references the issue so `gh` linking works.

7. **Push and (when ready to ship) PR via `/pr`.** A single fix doesn't need a dedicated PR — multiple fixes can ship together when the in-flight branch merges to `main`. If the user wants this fix shipped on its own, run `/pr` after the commit.

## Rules

- **Never `git checkout -b fix/$ARGUMENTS`** or any other per-task branch.
- **Never push directly to `main`.**
- If `git status` shows substantial unrelated work in progress, surface it — the user may want to commit or stash first so the fix lands cleanly.
- Commit messages reference the issue (`fix #N:`) so GitHub closes it on merge.
