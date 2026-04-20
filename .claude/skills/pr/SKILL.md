---
name: pr
description: Push current branch and create a pull request.
model: sonnet
user-invocable: true
argument-hint: [title]
allowed-tools: Bash, Read, Grep
---
# Create PR
## Steps
1. Check state: `git status && git log --oneline main..HEAD`
2. If on main, create a feature branch.
3. Push: `git push -u origin HEAD`
4. Create PR with `gh pr create`. Use $ARGUMENTS as title if given, otherwise draft from commits.
5. Return the PR URL.
