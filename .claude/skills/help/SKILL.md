---
name: help
description: Show available skills — the daily workflow cheat sheet.
model: haiku
user-invocable: true
allowed-tools: Read
---
# Skills

```
Morning:    /focus → /sync → /next
Work:       /fix, /check, /pr
Evening:    /done
Monday:     /audit

── Developer ──────────────────────────────────────
/dev            Start local dev environment
/check          Lint + test. Am I clean?
/fix <issue>    Fix GitHub issue → branch → PR
/pr [title]     Push and create PR
/wip            Save WIP, push, switch devices
/test [crate]   Expand test coverage

── Git ────────────────────────────────────────────
/branch <type/slug>    Start topic branch + worktree
/merge-cleanup <branch> Post-merge: delete branch + worktree

── Product Owner ──────────────────────────────────
/focus          Session opener. Where are we?
/next           Pick from milestone
/ticket <desc>  File a GitHub issue
/done           End session. Progress + observations
/experiment     Hypothesis-driven development

── VP Engineering ─────────────────────────────────
/audit          Weekly: security, compliance, drift
/review <pr>    Review a pull request

── DevOps ─────────────────────────────────────────
/sync           Pull, PRs, issues, build health
/deploy [target] Deploy to production
```

Justfile equivalents: `just git-hygiene`, `just worktree <branch>`, `just worktree-rm <branch>`, `just worktrees`

For the full reference: `~/dev/work/kb/Workflow/Cheat Sheet.md`
