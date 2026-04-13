---
name: ship
description: Full ship cycle — check, build, test, commit, deploy
disable-model-invocation: true
argument-hint: [message]
allowed-tools: Bash, Read, Grep
---

# Ship It

Complete ship cycle: check everything compiles, commit, and deploy.

## Steps

1. **Check everything compiles**
```bash
just check
```

2. **Run security audit** (quick — just cargo audit)
```bash
cd backend && cargo audit 2>&1 || true
cd ../crates/wolfgang-core && cargo audit 2>&1 || true
```

3. **Show what changed**
```bash
git status
git diff --stat
```

4. **Commit** — use $ARGUMENTS as commit message if provided, otherwise draft one from the changes. Ask the user to confirm the message before committing.

5. **Deploy backend**
```bash
just deploy-backend
```

6. **Deploy web**
```bash
just deploy-web
```

7. **Verify deployment**
```bash
gcloud run services describe wolfgang-backend --region=europe-west1 --project=wolfgang-kb-prod --format="value(status.conditions[0].status)"
```

Confirm with the user before each major step (commit, deploy). Stop immediately if any check fails.
