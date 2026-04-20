---
name: sync
description: Pull latest, show PRs, issues, milestone progress, service health.
model: sonnet
user-invocable: true
allowed-tools: Bash, Read, Grep
---
# Sync
Morning briefing — catch up on everything.
## Steps
1. Pull latest: `git pull --rebase origin main`
2. Open PRs: `gh pr list`
3. Recently merged: `gh pr list --state=merged --limit=5`
4. Open issues: `gh issue list --limit=10`
5. Milestone progress from `MILESTONES.md`
6. Compile check: `just check 2>&1 | tail -3` or `bun run check 2>&1 | tail -3`
## Output
```
── Sync ───────────────────────────────────────────
PRs:       <N> open
Merged:    <N> since last sync
Issues:    <N> open
Milestone: <done>/<total>
Build:     <green|red>
────────────────────────────────────────────────────
```
## Rules
- Under 2 minutes. Brevity over completeness.
