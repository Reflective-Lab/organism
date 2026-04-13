---
name: wip
description: Save work-in-progress and push — use before switching devices
disable-model-invocation: true
allowed-tools: Bash
---

# Save WIP and Push

Quick save before switching devices. Ensures all work is pushed so you can pick up anywhere.

## Steps

1. **Check for running background agents**
   If any agents are still running, warn the user. They'll finish and push their own PRs.

2. **Show current state**
```bash
jj status 2>/dev/null || git status
```

3. **Save current work**

If using jj:
```bash
jj describe -m "WIP: $(jj log -r @ --no-graph -T 'description')" 2>/dev/null
jj git push --change @ 2>/dev/null
```

If using git (fallback):
```bash
git add -A
git stash || true
git checkout -b wip/$(date +%Y%m%d-%H%M%S) 2>/dev/null || true
git stash pop 2>/dev/null || true
git add -A
git commit -m "WIP: work in progress — $(date +%Y-%m-%d)"
git push -u origin HEAD
```

4. **Push all local branches that have unpushed changes**
```bash
jj git push --all 2>/dev/null || git push --all
```

5. **Summary** — tell the user:
   - What was saved and pushed
   - Any branches/changes that are in flight
   - How to resume: `jj git fetch && jj edit <change-id>` or `git pull`
