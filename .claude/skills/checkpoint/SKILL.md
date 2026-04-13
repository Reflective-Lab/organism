---
name: checkpoint
description: End-of-session checkpoint — capture what moved, update MILESTONES.md, flag date risks
disable-model-invocation: true
user-invocable: true
allowed-tools: Read, Edit, Bash
---

# Session Checkpoint

End the session with accountability.

## Steps

1. **Read `MILESTONES.md`** — check the current milestone.

2. **Review session work** — look at git diff and any files changed this session.
   ```bash
   git diff --stat HEAD 2>/dev/null
   git log --oneline -5 2>/dev/null
   ```

3. **Check off deliverables** — if any deliverables were completed this session, mark them `[x]` in `MILESTONES.md` with today's date as a comment.

4. **Update `CHANGELOG.md`** — add notable changes under `## [Unreleased]` (new features, breaking changes, fixes). Skip trivial edits.

5. **Output the checkpoint:**

```
── Checkpoint ─────────────────────────────────────

Moved:
- <what was accomplished this session>

Remaining for <milestone name> (<N> days left):
- <unchecked deliverables still open>

Risks:
- <anything that threatens the deadline, or "None">

────────────────────────────────────────────────────
```

## Rules

- Be honest. If nothing meaningful moved, say so.
- If a deliverable is partially done, don't check it off — note progress.
- If work happened outside the current milestone, flag it under Risks as scope drift.
- If a milestone completed, check whether the parent epic in `~/dev/work/EPIC.md` has signals to update.
- Keep it to 10 lines max. This is a log entry, not a report.
