---
name: help
description: Show available skills — the daily workflow cheat sheet.
model: haiku
user-invocable: true
allowed-tools: Read
---
# Skills

Single-developer workflow. `main` plus one in-flight branch (`next` or `release/<version>`). No worktrees. No per-task topic branches. Full policy: `~/dev/CLAUDE.md` § "Git Workflow".

```
Morning:    /focus → /sync → /next
Work:       /fix, /check
Ship:       /pr → /merge-cleanup
Evening:    /done
Monday:     /audit

── Developer ──────────────────────────────────────
/dev            Start local dev environment
/check          Lint + test. Am I clean?
/fix <issue>    Fix GitHub issue on current branch
/wip            Save WIP, push, switch devices (current branch)
/test [crate]   Expand test coverage

── Git ────────────────────────────────────────────
/branch [release/<version>]   Switch to (or create) the in-flight branch.
                              Defaults to `next`. No worktrees.
/pr [title]                   PR the in-flight branch → main
/merge-cleanup [branch]       After merge: delete spent branch, rotate `next`

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

For the full reference: `~/dev/reflective/stack/bedrock-platform/kb/Workflow/Cheat Sheet.md`
