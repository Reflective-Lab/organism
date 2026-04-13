---
name: pr
description: Create a pull request from current changes
disable-model-invocation: true
argument-hint: [title]
allowed-tools: Bash, Read, Grep
---

# Create Pull Request

Create a PR from the current branch's changes.

## Steps

1. **Check current state**
```bash
git status
git log --oneline main..HEAD
git diff --stat main..HEAD
```

2. **Ensure branch is not main** — if on main, create a feature branch from the changes:
```bash
git checkout -b feature/<descriptive-name>
```

3. **Push to remote**
```bash
git push -u origin HEAD
```

4. **Create PR** using `gh`. Use $ARGUMENTS as title if provided, otherwise draft from commit history.
```bash
gh pr create --title "..." --body "$(cat <<'EOF'
## Summary
- ...

## Test plan
- [ ] ...

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

5. Return the PR URL.
