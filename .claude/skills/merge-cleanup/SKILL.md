---
name: merge-cleanup
description: Post-merge cleanup — delete local branch, remove worktree, delete remote branch.
model: haiku
user-invocable: true
argument-hint: <branch>  (e.g. feat/formation-tournament)
allowed-tools: Bash
---
# Merge Cleanup
After a PR merges, clean up the branch and worktree completely.

## Steps

If `$ARGUMENTS` is provided, use it as the branch name. Otherwise ask.

1. Confirm the branch is merged:
   ```bash
   gh pr list --state=merged --head=<branch> --limit=1
   ```
   If no merged PR found, warn and ask before proceeding.

2. Switch root checkout to main and pull:
   ```bash
   git switch main
   git pull --ff-only origin main
   ```

3. Remove worktree (if it exists):
   ```bash
   just git-worktree-rm <branch>
   ```
   If `../organism-<branch>` does not exist, skip silently.

4. Delete local branch:
   ```bash
   git branch -d <branch>
   ```
   If unmerged commits remain, report and stop — do not force delete.

5. Delete remote branch:
   ```bash
   git push origin --delete <branch>
   ```
   If remote branch does not exist, skip silently.

6. Confirm with:
   ```bash
   just git-hygiene
   ```

## Rules
- Never force-delete a local branch (`-D`) without explicit user confirmation.
- If the remote branch is already gone, that is fine — continue.
- Always end with `just git-hygiene` so the user can see the cleaned state.
