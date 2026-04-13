---
name: parallel
description: Run multiple tasks in parallel using isolated git worktrees — each agent pushes and creates a PR when done
disable-model-invocation: true
argument-hint: [task descriptions separated by |]
---

# Parallel Task Execution

Run multiple independent tasks simultaneously, each in its own git worktree. Agents push branches and create PRs autonomously — you review asynchronously.

## Input format

Tasks are separated by `|` in $ARGUMENTS. Example:
```
/parallel fix the sidebar toggle | add loading spinner to chat | update error messages
```

## Execution

For each task, launch an Agent with `isolation: "worktree"` and `run_in_background: true`.

Each agent MUST:

1. **Work in isolation** — full copy of the repo, no conflicts with other agents
2. **Implement the task** — make the minimum changes needed
3. **Verify** — run `just check` (or at minimum `cargo check` for Rust changes)
4. **Commit** with a clear message describing the change
5. **Push the branch**
   ```bash
   git push -u origin HEAD
   ```
6. **Create a PR** using `gh`:
   ```bash
   gh pr create --title "<short description>" --body "$(cat <<'EOF'
   ## Summary
   <what changed and why>

   ## Launched by
   `/parallel` skill — autonomous agent work

   🤖 Generated with [Claude Code](https://claude.com/claude-code)
   EOF
   )"
   ```

Launch ALL agents in a **single message** with multiple tool calls (truly parallel, not sequential).

## Important

- Each agent runs in the background — you don't wait for them
- Each agent creates its own branch and PR
- If an agent fails, it reports the error — other agents continue independently
- You review PRs later via `/sync` or `gh pr list`

## After completion

Report for each agent:
- PR URL (or error if it failed)
- Summary of changes
- Files modified
