---
tags: [philosophy]
---
# Relationship to Converge

Organism is a **client** of Converge and the intelligence layer beneath Axiom and Helm. Converge does not know Organism exists.

```
┌──────────────────────────────────────────┐
│  Axiom                                  │
│  truth definitions · projections        │
└──────────────────┬───────────────────────┘
                   │ starts `Engine.run()`
┌──────────────────▼───────────────────────┐
│  Organism                                │
│  reason · plan · debate · simulate       │
└──────────────────┬───────────────────────┘
                   │ submits `ProposedFact` / `AgentEffect`
┌──────────────────▼───────────────────────┐
│  Converge                                │
│  axioms · authority · promotion          │
└──────────────────┬───────────────────────┘
                   │ calls capability adapters
┌──────────────────▼───────────────────────┐
│  Providers                               │
│  ChatBackend · DdLlm · DdSearch          │
└──────────────────────────────────────────┘
```

Helm sits above Axiom as the operator-facing control surface. Organism normally stays architectural rather than product-branded.

## What Converge Owns (not Organism)

- The 9 convergence axioms
- The commit boundary and authorization barrier
- Policy and authority primitives
- Traceability and audit
- Foundational packs and commit-boundary truths

## What Organism Owns

- Translating Axiom-defined intent into a live reasoning plan
- Intent interpretation and decomposition
- Multi-model collaborative planning (huddle loop)
- Adversarial governance (assumption breakers, skeptics)
- Simulation swarm (outcome, cost, policy, causal, operational)
- Organizational learning (calibrating priors from outcomes)
- Organizational workflow packs and blueprints layered on top of Converge

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

Organism mirrors that split on its own side:
- `organism-pack` is the planning contract
- `organism-runtime` is the curated embedding surface
- `organism-intelligence`, `organism-notes`, and `organism-domain` are opt-in libraries

Prefer those surfaces over direct dependencies on phase subcrates in app code.

Above Organism, Axiom and Helm should still consume Organism through those curated
surfaces rather than importing `organism-intent`, `organism-planning`,
`organism-adversarial`, `organism-simulation`, or `organism-learning` directly.

See also: [[Philosophy/Key Invariants]], [[Architecture/Converge Contract]], [[Architecture/Remember-Organism-When]]
