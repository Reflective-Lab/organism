---
name: sync
description: Daily sync — pull, PRs, issues, agent work, service health, what to work on next
disable-model-invocation: true
allowed-tools: Bash, Read, Grep
---

# Daily Sync

Morning briefing — catch up on everything in under 2 minutes.

## Steps

1. **Pull latest**
```bash
jj git fetch 2>/dev/null && jj log --limit 10 2>/dev/null || git pull --rebase origin main
```

2. **Agent work overnight** — PRs created by agents
```bash
echo "═══ PRs awaiting review ═══"
gh pr list --json number,title,author,createdAt,reviewDecision --template '{{range .}}#{{.number}} {{if eq .reviewDecision ""}}⏳{{else}}{{.reviewDecision}}{{end}} {{.title}} ({{.author.login}}, {{.createdAt | timeago}}){{"\n"}}{{end}}'
```

3. **What shipped since last sync**
```bash
echo "═══ Recently merged ═══"
gh pr list --state=merged --limit=5 --json number,title,mergedAt --template '{{range .}}#{{.number}} {{.title}} ({{.mergedAt | timeago}}){{"\n"}}{{end}}'
```

4. **Open issues by priority**
```bash
echo "═══ Open issues ═══"
gh issue list --limit=15 --json number,title,labels --template '{{range .}}#{{.number}} [{{range .labels}}{{.name}} {{end}}] {{.title}}{{"\n"}}{{end}}'
```

5. **Current milestone progress**
Read `MILESTONES.md` at the repo root. Count checked `[x]` vs unchecked `[ ]` deliverables for the current milestone. Calculate days remaining to deadline.

```bash
echo "═══ Milestone ═══"
grep -c '\[x\]' MILESTONES.md 2>/dev/null || echo "0"
grep -c '\[ \]' MILESTONES.md 2>/dev/null || echo "0"
```

6. **Service health**
```bash
echo "═══ Service health ═══"
gcloud run services describe wolfgang-backend --region=europe-west1 --project=wolfgang-kb-prod --format="value(status.conditions[0].type,status.conditions[0].status)" 2>&1
```

7. **Compile check**
```bash
just check 2>&1 | tail -3
```

## Output format

Summarize as a brief morning briefing:

```
Morning Briefing — <date>

Services:    backend ✓  web ✓
PRs:         3 open (2 from agents, 1 from you)
Issues:      12 open, 3 in current sprint
Milestone:   "v1.2 — Web Launch" — 7/10 done
Shipped:     2 PRs merged since last sync
Builds:      all green

Recommended next:
1. Review PR #47 (agent: add input validation)
2. Review PR #48 (agent: update deps)
3. Work on #23 (Firebase Google sign-in) — medium, in sprint
```
