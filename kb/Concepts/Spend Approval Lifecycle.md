---
tags: [concept, spend-approval, autonomous-org, suggestors]
---
# Spend Approval Lifecycle

Spend and expense approval is Organism-domain behavior. It does not belong in
Axiom because Axiom can only validate the truth shape without knowing local
policy, budget envelopes, approver topology, or escalation rules. It does not
belong in Converge foundation either; Converge provides the convergence,
authority, and promotion boundary, while Organism supplies the organizational
workflow semantics.

## Lifecycle

| Stage | Pack Suggestor | Context Movement |
|---|---|---|
| Admission | `SpendAdmissionSuggestor` | `Seeds` -> `Signals` |
| Approval routing | `ApprovalRoutingSuggestor` | `Signals` -> `Strategies` |
| Policy challenge | `ApprovalPolicySkepticSuggestor` | `Strategies` -> `Evaluations` |
| Budget simulation | `BudgetSimulationSuggestor` | `Strategies` + `Evaluations` -> `Proposals` |

The `examples/expense-approval` crate is now a thin consumer of these
Suggestors. The reusable behavior lives in
`organism_domain::packs::autonomous_org`, where concrete apps can compose it
with procurement, people, legal, or finance-specific context.

## Boundary

Upstream examples are still useful as fixtures and teaching material, but the
moment the example needs organizational thresholds, policy versions, budget
state, or approver routing, it should graduate into `organism-domain`.

Concrete apps should provide request seeds and tenant-specific budget/policy
configuration. Helms or other app surfaces should not copy the approval logic;
they should import the pack Suggestors and own presentation, integration, and
trust-transfer flows.
