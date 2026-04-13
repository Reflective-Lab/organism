---
name: fix
description: Fix a GitHub issue — read, branch, implement, check, PR.
user-invocable: true
argument-hint: [issue-number]
allowed-tools: Bash, Read, Edit, Write, Grep, Glob
---
# Fix #$ARGUMENTS
## Steps
1. Read the issue: `gh issue view $ARGUMENTS`
2. Branch: `git checkout -b fix/$ARGUMENTS`
3. Explore relevant code.
4. Implement the minimum fix. Follow existing patterns.
5. Verify: `just lint` or `bun run check`
6. Commit: `Fix #$ARGUMENTS: <description>`
7. Push and PR: `git push -u origin HEAD && gh pr create --title "Fix #$ARGUMENTS: <description>" --body "Closes #$ARGUMENTS"`
8. Return the PR URL.
