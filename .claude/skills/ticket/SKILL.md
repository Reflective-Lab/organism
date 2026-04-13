---
name: ticket
description: Create a GitHub issue — detailed enough for an agent to execute.
user-invocable: true
argument-hint: [description]
allowed-tools: Bash, Read, Grep, Glob
---
# Create Ticket
Create a GitHub issue from $ARGUMENTS with concrete requirements, key files, and test plan.
## Steps
1. Explore codebase to identify relevant files.
2. Determine area and size (small/medium/large).
3. Create issue with `gh issue create` including: Context, Requirements (checkboxes), Key files, Test plan, Size.
4. Return the issue URL.
## Rules
- Every requirement must be testable.
- Key files must be real paths.
- If large, suggest splitting.
