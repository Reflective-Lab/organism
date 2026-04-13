---
name: review
description: Review a pull request — security, correctness, style
disable-model-invocation: true
argument-hint: [pr-number]
context: fork
agent: Plan
allowed-tools: Bash, Read, Grep, Glob
---

# Review PR #$ARGUMENTS

## Steps

1. **Read the PR**
```bash
gh pr view $ARGUMENTS
gh pr diff $ARGUMENTS
```

2. **Review for:**

**Security**
- Hardcoded secrets or credentials
- SQL injection, XSS, command injection
- Auth/authz bypasses
- Unsafe deserialization

**Correctness**
- Logic errors
- Missing error handling
- Edge cases not covered
- Breaking changes to public APIs

**Style**
- Follows existing patterns in the codebase
- No unnecessary complexity
- Clear naming

**Operations**
- Will this break the deploy?
- Are there migration steps needed?
- Any new env vars or secrets required?

3. **Summarize** findings as:
- Blockers (must fix before merge)
- Suggestions (nice to have)
- Questions (need clarification)

Do NOT leave PR comments — just report findings to the user.
