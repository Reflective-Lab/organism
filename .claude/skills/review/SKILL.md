---
name: review
description: Review a pull request — security, correctness, style, ops.
model: opus
user-invocable: true
argument-hint: [pr-number]
allowed-tools: Bash, Read, Grep, Glob
---
# Review PR #$ARGUMENTS
## Steps
1. Read: `gh pr view $ARGUMENTS && gh pr diff $ARGUMENTS`
2. Review for:
   - **Security** — secrets, injection, auth bypass
   - **Correctness** — logic, edge cases, breaking changes
   - **Style** — follows existing patterns, clear naming
   - **Ops** — will it break deploy? New env vars?
3. Report: Blockers, Suggestions, Questions.
## Rules
- Don't leave PR comments. Report to user.
