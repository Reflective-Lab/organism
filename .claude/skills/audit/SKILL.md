---
name: audit
description: Run security audit — vulnerabilities, drift, secrets, container scan
disable-model-invocation: true
allowed-tools: Bash, Read, Grep
---

# Security Audit

Run the full security audit via the Justfile target and supplement with code review.

## Automated checks
```bash
just security-audit
```

This covers:
- Terraform drift detection
- Public Cloud Run services inventory
- Secrets audit
- `cargo audit` (Rust dependency CVEs)
- Trivy container image scan

## Additional manual checks

1. **Review IAM bindings** — who has access to what
```bash
gcloud projects get-iam-policy wolfgang-kb-prod --format=json | jq '.bindings[] | {role, members}'
```

2. **Check for hardcoded secrets** in the codebase
Search for API keys, tokens, or passwords that shouldn't be committed.

3. **Review Cloud Armor logs** for abuse patterns
```bash
gcloud logging read 'resource.type="http_load_balancer" AND jsonPayload.enforcedSecurityPolicy.outcome="DENY"' --project=wolfgang-kb-prod --limit=20 --format=json
```

4. **Check secret versions** — flag any that haven't been rotated in 90+ days
```bash
for s in wolfgang-anthropic-api-key wolfgang-openai-api-key wolfgang-firebase-api-key; do
  echo -n "$s: "
  gcloud secrets versions list $s --project=wolfgang-kb-prod --limit=1 --format="value(createTime)"
done
```

Summarize findings with severity (critical/high/medium/low) and recommended actions.
