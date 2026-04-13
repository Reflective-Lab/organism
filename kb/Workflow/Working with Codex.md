---
tags: [workflow, codex]
---
# Working with Codex

Start from `CODEX.md`. Codex uses `AGENTS.md`, the knowledgebase, and `just` recipes.

## Shared Automation
```bash
just focus     # Session opener
just sync      # Team sync
just status    # Build health
```

## Workflow Equivalents

| Claude workflow | Use with Codex |
|---|---|
| `/focus` | "Run the focus workflow" or `just focus` |
| `/fix 42` | "Fix issue 42: read issue, implement, run just check/test/lint, prepare PR" |
| `/checkpoint` | "Write a session checkpoint" |

See also: [[Workflow/Daily Journey]]
