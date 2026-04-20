---
name: deploy
description: Deploy to production. Confirms before every destructive step.
model: sonnet
user-invocable: true
argument-hint: [backend|web|all]
allowed-tools: Bash, Read
---
# Deploy
## Steps
1. Run `/check` first. Stop if anything fails.
2. Deploy target ($ARGUMENTS or ask): `just deploy-backend`, `just deploy-web`, or both.
3. Verify health after deploy.
4. Report status.
## Rules
- Confirm with user before each deploy step.
- Backend first, verify, then web.
- If no `just deploy-*` recipe exists, show what manual steps are needed.
