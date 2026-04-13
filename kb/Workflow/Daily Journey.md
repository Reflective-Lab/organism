---
tags: [workflow]
---
# Daily Journey

## Morning
```
/focus              Orient — kb, build health, team activity
/sync               What did the team do? PRs? Issues?
```

## Working
```
/ticket <desc>      Create an agent-ready issue
/fix <issue#>       Branch, fix, test, PR
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
/merge <pr#>        Squash-merge, sync, clean up
/pr [title]         Create a PR
```

## End of Day
```
/checkpoint         What moved? What's left?
/wip                Save and push
```

See also: [[Workflow/Working with Claude]], [[Workflow/Working with Codex]], [[Workflow/Working with Gemini]]
