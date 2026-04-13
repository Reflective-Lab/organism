---
name: next
description: Show remaining tasks for current milestone.
user-invocable: true
allowed-tools: Read, Grep
---
# Next
Read `MILESTONES.md`. List unchecked `[ ]` deliverables from the current milestone.
## Output
```
── Next ───────────────────────────────────────────
<milestone> — <N> days left
1. <deliverable> (#issue)
2. <deliverable> (#issue)
3. ...
────────────────────────────────────────────────────
```
## Rules
- MILESTONES.md only. No network. No git. No compile.
- Number items so user can say "let's do 3".
- Don't recommend order. User picks.
