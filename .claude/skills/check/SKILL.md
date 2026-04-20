---
name: check
description: Lint + type check + test. Am I clean?
model: sonnet
user-invocable: true
allowed-tools: Bash, Read
---
# Check
Run the full quality gate for this project.
## Steps
1. `just lint` or `bun run check` (whichever applies)
2. `just test` or equivalent (if tests exist)
3. Report warnings and errors with file paths and line numbers.
## Rules
- Fix auto-fixable issues (formatting, simple clippy).
- Report remaining issues clearly.
- If everything passes, just say "Clean."
