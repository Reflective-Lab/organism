---
name: dev
description: Start local development environment.
model: sonnet
user-invocable: true
argument-hint: [backend|web|desktop|all]
allowed-tools: Bash, Read
---
# Dev
Start local dev. Use `just dev` if a Justfile exists, otherwise `bun run dev`.
## Rules
- Check required tools are installed.
- Report missing dependencies clearly.
- For multi-target projects (backend + desktop + web), start what $ARGUMENTS says, or ask.
