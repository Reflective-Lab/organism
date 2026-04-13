---
name: focus
description: Session opener — milestone, days left, open deliverables. TRIGGER at the start of every conversation.
user-invocable: true
allowed-tools: Read, Grep, Bash
---
# Focus
Read `MILESTONES.md`. Show the current milestone, which epic it advances (`~/dev/work/EPIC.md`), deadline, and unchecked deliverables.
## Output
```
── Focus ──────────────────────────────────────────
Milestone:   <name>
Epic:        <id and name>
Deadline:    <date> (<N> days left)
Progress:    <done>/<total>
Remaining:
- <deliverable 1>
- ...
────────────────────────────────────────────────────
```
## Rules
- If deadline < 7 days: warn.
- If deadline passed: flag blocker.
- Don't suggest work. Show state. User picks.
