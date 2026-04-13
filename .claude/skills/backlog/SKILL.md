---
name: backlog
description: View, prioritize, and manage the backlog — issues, PRs, and project board
disable-model-invocation: true
argument-hint: [list|prioritize|sprint|triage]
allowed-tools: Bash, Read
---

# Backlog Management

## Commands

### List (default)
Show the current state of work:

```bash
echo "═══ Open Issues ═══"
gh issue list --limit=20 --json number,title,labels,assignees --template '{{range .}}#{{.number}} [{{range .labels}}{{.name}} {{end}}] {{.title}}{{"\n"}}{{end}}'

echo ""
echo "═══ Open PRs ═══"
gh pr list --json number,title,author,reviewDecision --template '{{range .}}#{{.number}} ({{.reviewDecision}}) {{.title}} — {{.author.login}}{{"\n"}}{{end}}'

echo ""
echo "═══ Recently Closed ═══"
gh issue list --state=closed --limit=5 --json number,title,closedAt --template '{{range .}}#{{.number}} {{.title}} ({{.closedAt}}){{"\n"}}{{end}}'
```

### Prioritize
If $ARGUMENTS is "prioritize":

1. List all open issues
2. Group by area (backend, web, desktop, infra, etc.)
3. Suggest priority order based on:
   - Dependencies (what blocks what)
   - Size (quick wins first)
   - Impact (security > features > cosmetic)
4. Present a recommended sprint plan

### Sprint
If $ARGUMENTS is "sprint":

1. List issues labeled "sprint" or assigned to current milestone
2. Show progress (open vs closed)
3. Flag blockers or overdue items

### Triage
If $ARGUMENTS is "triage":

1. Find issues with no labels, no assignees, or missing required fields
2. For each untriaged issue, suggest:
   - Labels (area, size, priority)
   - Whether it's agent-executable as written, or needs more detail
3. Offer to update the issues with the suggestions
