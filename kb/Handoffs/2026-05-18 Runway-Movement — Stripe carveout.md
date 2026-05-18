---
tags: [handoff, cross-repo, boundary]
source: organism
date: 2026-05-18
targets: [runway, movement]
---
# Handoff to Runway and Movement — Stripe carveout

Organism no longer owns Stripe billing code. The removed
`organism_intelligence::billing` module mixed provider transport, SaaS checkout,
metering, credit-ledger, and webhook semantics into Organism's formation
intelligence boundary.

## Ownership boundary

| Concern | Owner | Notes |
|---|---|---|
| Stripe webhook route, signing secret access, deployment config, runtime observability | Runway | Transport and host/runtime infrastructure only. |
| Stripe checkout, subscription, invoice, payment, meter-event, provider object mapping | Movement | Commercial provider adapter behind Movement's commerce contract. |
| Webhook receipt, idempotency, replay, reconciliation, entitlement grants, commercial ledger, payouts | Movement | Stripe IDs must stay provider references, not primary domain IDs. |
| Planning constraints derived from commercial state | Organism consumes typed facts | Organism must not call Stripe or decide commercial state. |

## Code that was removed from Organism

- `crates/intelligence/src/billing/types.rs` — Stripe-shaped checkout,
  customer, payment link, meter event, webhook, and credit ledger types.
- `crates/intelligence/src/billing/client.rs` — direct Stripe HTTP client,
  including checkout, payment link, customer, and meter-event endpoints.
- `crates/intelligence/src/billing/webhook.rs` — Stripe webhook signature
  parsing, dispatch, and credit-grant handling.
- `crates/intelligence/src/billing/ledger.rs` — Firestore-backed usage credit
  ledger.

## Porting notes

- Runway should keep only the ingress contract: authenticated route, raw body,
  `Stripe-Signature` header, signing secret lookup, request observability, and
  handoff to Movement.
- Movement should express provider events through its commerce contracts:
  provider object references, webhook receipts, replay keys, commercial
  commands, subscriptions, entitlement grants, ledger entries, transfer intents,
  and payout obligations.
- Do not copy the Organism webhook verifier as-is. It used ordinary byte
  equality while claiming constant-time comparison, and it exposed too much
  signature material in errors.
- Do not keep an Organism compatibility shim. If Organism needs budget or
  entitlement data, it should receive typed facts or host-provided capability
  constraints from the owning layer.
