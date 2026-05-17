---
tags: [architecture, formations, mosaic, load-bearing]
source: llm
---
# Specialist Bench Formations

Organism's formation engine must understand both sides of the stack:

1. Converge fixed-point loops are the execution model.
2. Mosaic extensions are the specialist bench.

A Formation is not a workflow recipe. It is a hypothesis about which Suggestors
and capability handles can make shared context converge under budget, policy,
authority, and replay constraints.

## The Fixed-Point Question

Before selecting a roster, ask:

- What context keys must become stable?
- Which Suggestors can propose candidate facts?
- Which Suggestors can challenge, invalidate, enrich, retrieve, score,
  optimize, authorize, or explain those proposals?
- Which read/write relationships create useful feedback in the loop?
- Which specialists should run in parallel so Converge can compare competing
  proposals rather than accept a single answer?
- Which budgets, HITL gates, replay needs, and authority constraints bound the
  run?

Formation selection is wrong if it only maps an intent to a static template.
Selection must explain how the chosen roster helps Converge reach stability or
honest budget exhaustion.

## Mosaic Specialist Bench

Organism should prefer Mosaic's professional implementations over local
algorithmic cores. Product layers may provide data, credentials, host policy,
and writeback. Organism selects and composes specialists.

| Bench | Use in a Formation |
|---|---|
| Arbiter | Policy checks, Cedar authorization, delegation checks, approval requirements, policy counter-signals. |
| Manifold | Generic LLM/search/fetch/feed/tool/storage/vector/provider adapters and runtime capability handles. |
| Embassy | Source-specific evidence with provenance when the external system identity is part of the contract. |
| Mnemos | Knowledge retrieval, recall, memory, prior episodes, evidence seeding, feedback learning. |
| Prism | Regression, fuzzy inference, ranking, forecasting, anomaly detection, classification, feature extraction, ML critique. |
| Ferrox | Optimization, scheduling, routing, allocation, feasibility, solver-backed constraint satisfaction. |

If a business problem calls for regression, fuzzy logic, ranking, forecasting,
anomaly detection, or ML critique, the Formation should look to Prism. If it
calls for constrained optimization, routing, scheduling, allocation, or
feasibility proof, it should look to Ferrox. Similar rules apply to Arbiter,
Manifold, Embassy, and Mnemos.

## Product-Layer Boundary

Helm and applications may:

- register executable factories and capability handles
- provide tenant policy, credentials, data, thresholds, and prompts
- decide host policy such as cost caps, tournament size, HITL placement, and
  writeback behavior
- render admission receipts, traces, outcomes, and projections

They must not implement reusable specialist cores such as regression engines,
fuzzy-logic engines, optimizers, policy engines, vector recall systems,
provider adapters, memory layers, or source connectors.

Thin product-specific glue is allowed when it translates product context into a
Mosaic contract and translates the result back into the product projection.

## Selection Trace Requirement

For every non-trivial Formation, the trace should show:

- which Mosaic bench families were considered
- which specialists were selected
- each specialist's loop contribution: propose, retrieve, validate, challenge,
  score, optimize, authorize, observe, or synthesize
- why omitted bench families were not needed
- which host-policy limits shaped the roster

An omitted specialist is acceptable. A silent omission is not.

## Exception Rule

If no Mosaic implementation exists yet, record the gap in the correct upstream
backlog before adding local glue. Local code must stay temporary and must not
become a public product API. If the capability proves reusable, promote it to
the canonical Mosaic owner.
