---
name: jj
description: Jujutsu (jj) version control — colocated with git. Use for branching, parallel work, and change management.
disable-model-invocation: true
argument-hint: [init|status|new|split|squash|push|help]
allowed-tools: Bash, Read
---

# Jujutsu (jj) — colocated with git

This repo uses jj colocated with git. Both work on the same repo. Use jj for the daily workflow, git for pushing/PRs (GitHub doesn't speak jj yet).

## Commands

### First-time setup
If $ARGUMENTS is "init":
```bash
cd /Users/kpernyer/repo/wolfgang-app && jj git init --colocate
```
This creates `.jj/` alongside `.git/`. Both see the same history.

### Status
If $ARGUMENTS is "status" or empty:
```bash
jj status
jj log --limit 10
```

### New change (like git branch + checkout)
If $ARGUMENTS is "new":
```bash
jj new -m "description"
```
No branch names needed. Every change gets an auto-generated ID. jj automatically rebases children.

### Split a change
If $ARGUMENTS is "split":
```bash
jj split
```
Interactively split the current change into two. Useful when you've done too much in one change.

### Squash into parent
If $ARGUMENTS is "squash":
```bash
jj squash
```
Folds current change into its parent.

### Push to GitHub
If $ARGUMENTS is "push":
```bash
# Create a git branch from current jj change and push
jj git push --change @
```
This creates a branch named `push-<change-id>` and pushes it. Then create a PR with `gh pr create`.

### Help / cheat sheet
If $ARGUMENTS is "help":

```
jj new                  # start a new change (like git checkout -b)
jj new -m "desc"        # start with description
jj status               # what's modified
jj log                  # history (graph view)
jj diff                 # show current change
jj describe -m "msg"    # set/update change description
jj squash               # fold current into parent
jj split                # split current change in two
jj edit <change>        # jump to a change (like checkout)
jj abandon              # discard current change
jj git push --change @  # push current change to GitHub
jj git fetch            # pull from remote
jj rebase -d main       # rebase current onto main
```

## Key differences from git

| git | jj | Notes |
|-----|------|-------|
| `git add` + `git commit` | Automatic — jj tracks all changes | No staging area |
| `git branch` | Not needed — changes are the unit | Branches auto-created on push |
| `git stash` | Not needed — just `jj new` | Old change stays as-is |
| `git rebase -i` | `jj squash` / `jj split` | No interactive mode needed |
| `git merge` conflicts | `jj resolve` | Conflicts are first-class |

## Colocated rules
- Use `jj` for daily work (new, edit, describe, squash, split)
- Use `git`/`gh` for GitHub operations (push, PR, issues)
- `jj git fetch` to sync from remote (instead of `git pull`)
- `.jj/` is in `.gitignore` — invisible to git
