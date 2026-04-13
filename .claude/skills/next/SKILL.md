---
name: next
description: Show remaining tickets for current milestone — fast, no network calls
disable-model-invocation: true
user-invocable: true
allowed-tools: Read, Grep
---

# What's Next

Read `MILESTONES.md`. List only the unchecked `[ ]` deliverables from the current milestone, with issue numbers if present.

## Output

```
── Next ───────────────────────────────────────────

<milestone name> — <N> days left

1. <deliverable> (#issue)
2. <deliverable> (#issue)
3. ...

────────────────────────────────────────────────────
```

## Rules

- Read MILESTONES.md only. No network calls. No git. No compile checks.
- Number the items so the user can say "let's do 3".
- Do not recommend an order. The user picks.
- Must complete in under 2 seconds.
