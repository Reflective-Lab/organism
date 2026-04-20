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
│  form · reason · debate · simulate       │
└──────────────────┬───────────────────────┘
                   │ assembles `Formation`
┌──────────────────▼───────────────────────┐
│  Converge                                │
│  engine · authority · promotion          │
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
- The authority / promotion boundary
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

**Authority is never inherited from reasoning.** It is recomputed at Converge's authority boundary. Producing a plan does not grant the right to execute it.

## What Changed in Converge

These changes matter directly to Organism:

1. `Suggestor` is now the one universal in-loop contract.
   LLMs, optimizers, policy gates, analytics, knowledge retrieval, adversaries,
   and simulators all enter Converge the same way.
2. `Formation` is therefore the right Organism abstraction.
   Organism does not hand Converge a result to commit. It hands Converge a
   team of `Suggestor`s and seeds, then lets the engine find the fixed point.
3. Context naming is now unambiguous.
   `converge_kernel::Context` is the trait. `ContextState` is the concrete
   struct embedders create. There is no `ContextView`.
4. The fact boundary is enforced harder.
   Organism may freely construct `ProposedFact`. It may not construct
   authoritative `Fact`.
5. Downstreams use `converge-kernel`, `converge-pack`, and optionally
   `converge-model` / `converge-client`. `converge-core` is still internal.

## Why Formation Matters

`Formation` is Organism's unit of handoff to Converge:

- Organism chooses the team.
- Organism chooses the seeds.
- Organism may run multiple formations and compare them.
- Converge owns only the governed convergence run for each formation.

This keeps the boundary clean. Organism owns assembly. Converge owns
promotion, invariants, stop reasons, and integrity proof.

## Use Converge Types Directly

Organism uses `converge-pack`, `converge-kernel`, and `converge-model` directly. The Rust type system enforces the axioms — a Suggestor cannot forge a Fact, ProposedFact is not Fact, and the promotion gate is compiler-enforced. No wrapper layers.

| Mode | Crate | Why |
|---|---|---|
| Embedded | `converge-kernel` | In-process: Engine, `ContextState`, `ConvergeResult`, re-exported `Suggestor` contract |
| Authoring | `converge-pack` | Suggestor trait, ProposedFact, Invariant |
| Reading | `converge-model` | Governed semantic types (Fact, Proposal, PromotionRecord) |
| Remote | `converge-client` | gRPC wire protocol to deployed Converge |

Do NOT depend on `converge-core`, `converge-runtime`, or other internal crates.
Full contract: `~/dev/work/converge/kb/Architecture/API Surfaces.md`.

## Allowed and Not Allowed

Allowed:
- Implement `Suggestor` for any Organism agent type.
- Assemble heterogeneous teams with `Formation::new().agent(...)`.
- Seed `ContextState` through the public input/proposal path.
- Read governed facts from `ConvergeResult`.
- Compose Converge foundational packs with Organism packs.

Not allowed:
- Construct `Fact` directly.
- Depend on `converge-core` or other internal Converge crates.
- Treat Converge as a dumb "submit then commit" API.
- Invent side-car in-loop traits that bypass `Suggestor`.
- Sequence agents by name or rely on removed APIs like `ContextView`, `Context::new()`, or `register_in_pack(...)`.

## Pack Relationship

Converge owns foundational packs such as `trust`, `money`, `delivery`, and
`data_metrics`. Converge's policy, optimization, analytics, and knowledge
crates are still loop participants, but they participate as `Suggestor`s, not
as separate pipeline subsystems.

Organism owns organizational packs and blueprints on top of those foundations.

Organism mirrors that split on its own side:
- `organism-pack` is the planning contract
- `organism-runtime` is the curated embedding surface
- `organism-intelligence`, `organism-notes`, and `organism-domain` are opt-in libraries

Prefer those surfaces over direct dependencies on phase subcrates in app code.

Above Organism, Axiom and Helm should still consume Organism through those curated
surfaces rather than importing `organism-intent`, `organism-planning`,
`organism-adversarial`, `organism-simulation`, or `organism-learning` directly.

See also: [[Philosophy/Key Invariants]], [[Architecture/Converge Contract]], [[Architecture/Remember-Organism-When]]
