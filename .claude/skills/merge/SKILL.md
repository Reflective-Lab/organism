---
name: merge
description: Squash-merge a PR by number, sync main, delete the branch
disable-model-invocation: true
argument-hint: [pr-number]
allowed-tools: Bash
---

# Merge PR #$ARGUMENTS

Squash-merge, sync local main, clean up.

## Steps

1. **Show what's about to be merged**
```bash
gh pr view $ARGUMENTS
```

2. **Confirm CI is green.** If checks are failing, stop and report.
```bash
gh pr checks $ARGUMENTS
```

3. **Squash-merge and delete remote branch.**
```bash
gh pr merge $ARGUMENTS --squash --delete-branch
```

4. **Sync local main.**
```bash
git checkout main
git pull
```

5. **Report the merge.** Show the merged PR title and number.
