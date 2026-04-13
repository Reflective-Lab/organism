---
tags: [workflow]
---
# Daily Journey

## Morning
```
/focus              Orient — milestone, build health, team activity
/sync               What did the team do? PRs? Issues?
/next               Pick a task from the backlog
```

## Working
```
/fix <issue#>       Branch, fix, test, PR
/check              Run lint, check, tests
/pr [title]         Create a pull request
/ticket <desc>      Create an agent-ready issue
```

### Build loop
```bash
just build          Build all crates
just test           Run tests
just lint           Clippy — must be clean
```

## Reviewing
```
/review <pr#>       Security, correctness, style
```

## End of Day
```
/done               Update milestones, record what moved
/wip                Save and push work-in-progress
```

## Weekly
```
/audit              Monday — security, dependency, compliance, drift
```

## Anytime
```
/help               Show available workflows
```

See also: [[Workflow/Working with Claude]], [[Workflow/Working with Codex]], [[Workflow/Working with Gemini]]
