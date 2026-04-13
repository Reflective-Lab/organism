---
name: status
description: Check the health of all Wolfgang services and infrastructure
disable-model-invocation: true
allowed-tools: Bash
---

# Service Status Check

Check all Wolfgang services and report status.

## Steps

1. **Cloud Run backend**
```bash
gcloud run services describe wolfgang-backend --region=europe-west1 --project=wolfgang-kb-prod --format="table(status.conditions.type,status.conditions.status,status.conditions.message)"
```

2. **Backend health** — curl the endpoint
```bash
curl -s -o /dev/null -w "%{http_code}" https://api.wolfgang.bot/v1/chat/stream -X POST -H "Content-Type: application/json" -d '{}' --max-time 5
```

3. **Load balancer IP**
```bash
cd infra/environments/prod/wolfgang-bot && terraform output -raw load_balancer_ip
```

4. **Infrastructure drift**
```bash
cd infra/environments/prod/wolfgang-bot && terraform plan -detailed-exitcode 2>&1 | tail -5
```

5. **Recent Cloud Run logs** (last 10 lines)
```bash
gcloud run services logs read wolfgang-backend --region=europe-west1 --project=wolfgang-kb-prod --limit=10
```

6. **Firebase Hosting**
```bash
firebase hosting:channel:list --project=wolfgang-kb-prod 2>&1 || echo "Check https://wolfgang.bot manually"
```

Summarize findings in a clear status table: service, status (healthy/degraded/down), details.
