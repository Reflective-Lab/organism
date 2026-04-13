---
name: done
description: End session — progress, changelog, observations.
user-invocable: true
allowed-tools: Read, Edit, Bash
---
# Done
End the session with accountability.
## Steps
1. Read `MILESTONES.md` — current milestone.
2. Review session work: `git diff --stat HEAD && git log --oneline -5`
3. Check off completed deliverables in `MILESTONES.md` with today's date.
4. Update `CHANGELOG.md` under `## [Unreleased]`. Skip trivial edits.
5. Ask: **"Anything surprising or worth remembering?"** If yes, append to `~/dev/work/kb/Observations.md` under today's date.
6. Output:
```
── Done ───────────────────────────────────────────
Moved:
- <what was accomplished>
Remaining for <milestone> (<N> days left):
- <open deliverables>
Risks:
- <threats to deadline, or "None">
Observations: <N captured, or "None">
────────────────────────────────────────────────────
```
## Rules
- Be honest. If nothing moved, say so.
- Partial work → don't check off, note progress.
- Work outside current milestone → flag as scope drift.
- If milestone completed → check ~/dev/work/EPIC.md for signal updates.
- Observations = surprises and gotchas only, not routine work.
