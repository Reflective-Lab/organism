---
tags: [architecture]
---
# Converge Contract

How Organism uses Converge. See [[Philosophy/Relationship to Converge]] for why.

Shared stack guidance: `~/dev/reflective/stack/bedrock-platform/converge/kb/Architecture/Golden Path Matrix.md`.

## Direct Type Usage

Organism uses Converge types directly. The Rust type system enforces the axioms:
- a `Suggestor` cannot forge a `Fact`
- `ProposedFact` is not `Fact`
- promotion goes through the engine

No wrapper layers needed.

The same rule applies one layer up: apps above Organism should use
`organism-pack` and `organism-runtime` as their first imports rather than
assembling Organism from phase crates.

| Converge Crate | What Organism Uses It For |
|---|---|
| `converge-pack` | Suggestor trait, ProposedFact, Invariant — for authoring packs that run inside Converge |
| `converge-model` | Governed semantic types — for interpreting Converge results |
| `converge-kernel` | Engine, `ContextState`, `ConvergeResult`, re-exported `Suggestor` contract — for embedded execution |
| `converge-provider` | Backend identity, role-level `BackendRequirements`, and capability routing |
| `converge-client` | gRPC SDK — only for remote (out-of-process) deployment |

Use published Converge crates for normal development. Local path dependencies
are only for coordinated Converge changes, and those should be exceptional now
that the formation compiler substrate is released.

## Forbidden Dependencies

- `converge-core` — internal engine implementation
- `converge-runtime` — HTTP/gRPC server internals
- `converge-storage` — storage adapter internals
- Any other internal Converge crate

## Two Deployment Modes

**Embedded:** Organism runs Converge in-process via `converge-kernel`. Use `Engine`, `ContextState`, and the `Suggestor` trait directly. This is the primary model for Formation-based execution.

**Remote:** Organism talks to a deployed Converge instance via `converge-client` (the Converge crate, not a wrapper). This is a deployment choice, not a different conceptual contract.

## Formation Pattern

The correct Organism-to-Converge handoff is:

1. Organism assembles a `Formation`
2. every in-loop participant implements `Suggestor`
3. the formation seeds a `ContextState`
4. the formation runs a fresh `Engine`
5. Converge returns `ConvergeResult`

What Formation owns:
- team assembly
- seed selection
- budget selection
- running competing hypotheses

What Converge owns:
- eligibility and execution of suggestors
- proposal promotion
- invariants and typed stop reasons
- integrity proof and governed result

## Fixed-Point Formation Rule

A Formation exists to exploit Converge's fixed-point loop. It is not a prompt
chain, workflow recipe, or static roster. Organism selects Suggestors because
their read/write relationships should make shared context stabilize: some
propose candidate facts, some retrieve evidence, some challenge weak claims,
some score or optimize alternatives, some authorize or block, and some
synthesize the final shape.

Selection traces should explain this loop contribution. If a formation omits a
useful specialist family, the omission should be visible and intentional.

## Allowed

- Implement any Organism agent as `Suggestor`
- Emit typed `FactPayload` values through a canonical `ProvenanceSource` marker
  and override `Suggestor::provenance()` for every fact-emitting suggestor
- Mix LLM, optimization, policy, analytics, knowledge, adversarial, and simulation agents in one formation
- Stage initial inputs through `ContextState`
- Read promoted facts from `ConvergeResult`
- Use `register_suggestor()` and `register_suggestor_in_pack()` as the registration surface

## Not Allowed

- Depend on `converge-core`
- Construct `Fact` directly
- Emit a `ProposedFact` from a suggestor whose `provenance()` is empty
- Use removed or stale names such as `ContextView`, `Context::new()`, or `register_in_pack(...)`
- Bypass `Engine.run()` for governed fact creation
- Build wrapper types that pretend to replace the Converge surface
- Rely on suggestor name ordering for sequencing; use dependency keys instead
- Implement reusable specialist cores in Organism, Helm, or apps when Mosaic
  owns the professional implementation

## Extending Types

Extend through composition, not wrapping:

```rust
/// Organism's enriched plan — carries planning metadata alongside the Converge proposal.
pub struct OrganismPlan {
    pub proposed_fact: ProposedFact,
    pub simulation_report: SimulationReport,
    pub adversarial_findings: Vec<Finding>,
    pub debate_round: u32,
}
```

## The Authority Rule

Organism submits observations/proposals, not facts. Converge decides whether to promote. Organism has no authority in this transaction.

## Pack Placement Rule

Converge foundational packs remain Converge's concern. Organism's packs and
blueprints layer on top. Policy, optimization, analytics, and knowledge are
still valid formation members, but they are `Suggestor`s, not special pipeline
stages.

## Mosaic Specialist Bench

Use Mosaic extension implementations for specialist work:

| Need | Bench |
|---|---|
| Policy, Cedar, authorization, approval gates | Arbiter |
| Generic providers, storage, search, fetch, feed, vector, tools | Manifold |
| Source-specific connectors and provenance | Embassy |
| Knowledge, recall, retrieval, memory | Mnemos |
| Regression, fuzzy inference, ranking, forecasting, anomaly detection, ML | Prism |
| Optimization, scheduling, routing, allocation, feasibility proofs | Ferrox |

Organism may define formation roles, descriptors, input/output contracts, and
thin adapter-agnostic Suggestors. It should not import concrete adapter types
or rebuild the algorithms these extensions own.

See also: [[Architecture/Crate Map]], [[Architecture/Specialist Bench Formations]], [[Philosophy/Key Invariants]]
