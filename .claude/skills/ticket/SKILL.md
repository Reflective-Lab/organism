---
name: ticket
description: Create a well-defined GitHub issue that a human or agent can execute
disable-model-invocation: true
argument-hint: [description of what needs to be done]
allowed-tools: Bash, Read, Grep, Glob
---

# Create Agent-Ready Ticket

Create a GitHub issue from $ARGUMENTS that is detailed enough for an agent to pick up and execute without asking questions.

## Steps

1. **Understand the request** — explore the codebase if needed to identify:
   - Which files are involved
   - What the current behavior is
   - What needs to change

2. **Determine area and size**
   - Area: backend, web, desktop, wolfgang-core, ui, infra, security, docs
   - Size: small (< 1hr), medium (1-4hr), large (4hr+)

3. **Create the issue** using `gh`:

```bash
gh issue create --title "[AREA]: short description" --label "task" --body "$(cat <<'EOF'
## Context
Why this needs to happen. What problem it solves.

## Requirements
- [ ] Concrete acceptance criterion 1
- [ ] Concrete acceptance criterion 2
- [ ] Concrete acceptance criterion 3

## Key files
- `path/to/file.rs` — what this file does
- `path/to/other.ts` — why it's relevant

## Test plan
- [ ] How to verify criterion 1
- [ ] How to verify criterion 2

## Size
small | medium | large
EOF
)"
```

4. Return the issue URL.

## Rules
- Every requirement must be testable — "improve performance" is bad, "response time under 200ms" is good
- Key files must be actual paths in the repo, not guesses
- Test plan must be concrete commands or steps, not vague "test it works"
- If the task is large, suggest breaking it into smaller tickets
