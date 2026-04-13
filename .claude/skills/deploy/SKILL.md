---
name: deploy
description: Deploy backend and/or web to production
disable-model-invocation: true
argument-hint: [backend|web|all]
allowed-tools: Bash, Read
---

# Deploy to Production

Deploy the specified target ($ARGUMENTS or "all" if not specified).

## Backend deploy
```bash
just deploy-backend
```
1. Builds Docker image from `deploy/backend/Dockerfile.cloudrun`
2. Pushes to Artifact Registry
3. Updates Cloud Run via Terraform

## Web deploy
```bash
just deploy-web
```
1. Builds SvelteKit app
2. Deploys to Firebase Hosting

## After deploy
- Verify the service is healthy: `just status`
- Check logs: `gcloud run services logs read wolfgang-backend --region=europe-west1 --limit=20`

If deploying "all", deploy backend first, verify it's healthy, then deploy web.
Always confirm before running destructive commands.
