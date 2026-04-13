---
tags: [philosophy]
---
# Relationship to Converge

Organism is a **client** of Converge. Converge does not know Organism exists.

```
┌──────────────────────────────────────┐
│  Organism (Layer 2)                  │
│  reason · plan · debate · simulate   │
└──────────────────┬───────────────────┘
                   │ submits observations/proposals
┌──────────────────▼───────────────────┐
│  Converge (Layer 1)                  │
│  axioms · authority · commit         │
└──────────────────────────────────────┘
```

## What Converge Owns (not Organism)

- The 9 convergence axioms
- The commit boundary and authorization barrier
- Policy and authority primitives
- Traceability and audit
- Packs and business domain truths
- SaaS product UX

## What Organism Owns

- Intent interpretation and decomposition
- Multi-model collaborative planning (huddle loop)
- Adversarial governance (assumption breakers, skeptics)
- Simulation swarm (outcome, cost, policy, causal, operational)
- Organizational learning (calibrating priors from outcomes)

## The Authority Rule

**Authority is never inherited from reasoning.** It is recomputed at the Converge commit boundary. Producing a plan does not grant the right to execute it.

## Use Converge Types Directly

Organism uses `converge-pack`, `converge-kernel`, and `converge-model` directly. The Rust type system enforces the axioms — a Suggestor cannot forge a Fact, ProposedFact is not Fact, and the promotion gate is compiler-enforced. No wrapper layers.

| Mode | Crate | Why |
|---|---|---|
| Embedded | `converge-kernel` | In-process: Engine, Context, Suggestor directly |
| Authoring | `converge-pack` | Suggestor trait, ProposedFact, Invariant |
| Reading | `converge-model` | Governed semantic types (Fact, Proposal, PromotionRecord) |
| Remote | `converge-client` | gRPC wire protocol to deployed Converge |

Do NOT depend on `converge-core`, `converge-runtime`, or other internal crates.
Full contract: `~/dev/work/converge/kb/Architecture/API Surfaces.md`.

See also: [[Philosophy/Key Invariants]], [[Architecture/Converge Contract]]
