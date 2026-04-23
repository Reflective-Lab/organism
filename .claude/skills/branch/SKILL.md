---
name: branch
description: Start a topic branch + worktree for new work, following the organism git strategy.
model: haiku
user-invocable: true
argument-hint: <type/slug>  (e.g. feat/formation-tournament)
allowed-tools: Bash
---
# Branch
Start a clean topic branch with an isolated worktree.

## Branch types
| Prefix | Use |
|---|---|
| `feat/<slug>` | new capability |
| `fix/<slug>` | bug fix |
| `docs/<slug>` | KB or API docs |
| `ci/<slug>` | workflows, hooks, badges |
| `chore/<slug>` | maintenance |
| `release/<version>` | version bump + tag prep |
| `spike/<slug>` | disposable investigation |

## Steps

If `$ARGUMENTS` is provided, use it as the branch name. Otherwise ask.

1. Verify root checkout is on `main` and clean:
   ```bash
   git status --short --branch
   ```
   If dirty or not on main, warn and stop.

2. Pull latest:
   ```bash
   git pull --ff-only origin main
   ```

3. Create the worktree + branch:
   ```bash
   just git-worktree <branch>
   ```
   This runs: `git worktree add ../organism-<branch> -b <branch>`

4. Tell the user:
   - Worktree path: `../organism-<branch>`
   - How to enter: `cd ../organism-<branch>`
   - How to clean up when done: `just git-worktree-rm <branch>`

## Rules
- One concern per branch. If the task expands, split it.
- Never start from a dirty root checkout.
- Spike branches must not outlive their session without a note in the PR.
