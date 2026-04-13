---
tags: [workflow, codex]
---
# Working with Codex

Start from `CODEX.md`. Keep the same workflow names used in Claude docs. In Codex, name the workflow directly in plain text: `focus`, `run focus`, `check`, `done`, `audit`, `fix issue 42`, `review PR 5`.

## Shared Automation
```bash
just focus     # Session opener
just sync      # Team sync
just lint      # Lint and check
```

## Canonical Workflows

| Workflow | Use with Codex |
|---|---|
| `/focus` | `focus`, `run focus`, or `just focus` |
| `/sync` | `sync`, `run sync`, or `just sync` |
| `/check` | `check` or `run lint and check` |
| `/fix 42` | `fix 42` or `fix issue #42` |
| `/done` | `done` or `update MILESTONES.md and CHANGELOG.md` |

## Gemini Equivalents

Same canonical workflow names, but phrased for Gemini CLI:

| Workflow | Use with Gemini |
|---|---|
| `/focus` | `/focus`, `focus`, or `gemini "read MILESTONES.md, show current milestone"` |
| `/sync` | `/sync`, `sync`, or `gemini "pull, show PRs and issues"` |
| `/check` | `/check`, `check`, or `gemini "run lint and check"` |
| `/fix 42` | `/fix 42`, `fix 42`, or `gemini "fix issue #42"` |
| `/done` | `/done`, `done`, or `gemini "update MILESTONES.md and CHANGELOG.md"` |

`/done` may still be backed by the `checkpoint` workflow internally, and `/check` by `quality`. Keep `/done` and `/check` as the public names in docs and daily use.

See also: [[Workflow/Daily Journey]]
