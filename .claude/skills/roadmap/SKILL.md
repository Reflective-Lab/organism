---
name: roadmap
description: View and update the product roadmap — milestones, deliveries, and strategic priorities
disable-model-invocation: true
argument-hint: [show|plan|milestone|feedback]
allowed-tools: Bash, Read, Write, Grep
---

# Product Roadmap

Roadmap lives in `MILESTONES.md` at the repo root. Feedback is tracked as GitHub issues labeled "feedback".

## Commands

### Show (default)
Display current roadmap, milestone progress, and upcoming work:

```bash
echo "═══ Milestones ═══"
gh api repos/:owner/:repo/milestones --jq '.[] | "\(.title) — \(.open_issues) open, \(.closed_issues) closed, due: \(.due_on // "no date")"'

echo ""
echo "═══ Milestones ═══"
cat MILESTONES.md 2>/dev/null || echo "No MILESTONES.md found."
```

### Plan
If $ARGUMENTS is "plan":

1. Read current state:
   - Open issues and their labels/sizes
   - Recent commits (what was shipped)
   - Memory files for project context
2. Draft or update `MILESTONES.md` with:

```markdown
# Wolfgang Roadmap

## Current milestone: <name> (due: <date>)
- [ ] deliverable 1
- [ ] deliverable 2

## Next milestone: <name>
- [ ] deliverable 1

## Backlog
Prioritized list of future work.

## Completed
- [x] <milestone> — shipped <date>
```

3. Create/update GitHub milestones to match
4. Ask user to review and confirm

### Milestone
If $ARGUMENTS starts with "milestone":

Create or update a GitHub milestone:
```bash
gh api repos/:owner/:repo/milestones -f title="<name>" -f due_on="<YYYY-MM-DD>" -f description="<description>"
```

Then assign relevant open issues to it:
```bash
gh issue edit <number> --milestone "<name>"
```

### Feedback
If $ARGUMENTS is "feedback":

1. List feedback issues:
```bash
gh issue list --label=feedback --json number,title,body --template '{{range .}}#{{.number}} {{.title}}{{"\n"}}  {{.body | truncate 200}}{{"\n\n"}}{{end}}'
```

2. Summarize themes — group feedback by topic, identify patterns
3. Suggest which roadmap items address which feedback
4. Flag feedback that doesn't map to any planned work

## Analytics integration (future)

When Cloud Run logs and analytics are set up, this skill will also:
- Show usage trends (messages/day, active users)
- Show error rates and latency percentiles
- Correlate feedback with usage patterns
- Recommend priorities based on data

For now, check logs manually:
```bash
gcloud run services logs read wolfgang-backend --region=europe-west1 --project=wolfgang-kb-prod --limit=50
```
