---
name: wip
description: Save work-in-progress and push — use before switching devices.
user-invocable: true
allowed-tools: Bash
---
# WIP
Quick save before switching context.
## Steps
1. Show state: `git status`
2. Save:
   ```bash
   git add -A
   git checkout -b wip/$(date +%Y%m%d-%H%M%S) 2>/dev/null || true
   git commit -m "WIP: $(date +%Y-%m-%d)"
   git push -u origin HEAD
   ```
3. Tell user how to resume: `git checkout <branch> && git pull`
