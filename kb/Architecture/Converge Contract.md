---
tags: [architecture]
---
# Converge Contract

How Organism uses Converge. See [[Philosophy/Relationship to Converge]] for why.

## Direct Type Usage

Organism uses Converge types directly. The Rust type system enforces the axioms — a Suggestor cannot forge a Fact, ProposedFact is not Fact, and promotion goes through the gate. No wrapper layers needed.

| Converge Crate | What Organism Uses It For |
|---|---|
| `converge-pack` | Suggestor trait, ProposedFact, Invariant — for authoring packs that run inside Converge |
| `converge-model` | Governed semantic types — for interpreting Converge results |
| `converge-kernel` | Engine, Context — for embedded (in-process) execution |
| `converge-client` | gRPC SDK — only for remote (out-of-process) deployment |

## Forbidden Dependencies

- `converge-core` — internal engine implementation
- `converge-runtime` — HTTP/gRPC server internals
- `converge-provider` — provider adapter internals
- `converge-storage` — storage adapter internals
- Any other internal Converge crate

## Two Deployment Modes

**Embedded:** Organism runs Converge in-process via `converge-kernel`. Use `Engine`, `Context`, and the `Suggestor` trait directly. This is the default for development and single-process deployments.

**Remote:** Organism talks to a deployed Converge instance via `converge-client` (the Converge crate, not a wrapper). Use `SubmitObservationRequest` over gRPC.

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

See also: [[Architecture/Crate Map]], [[Philosophy/Key Invariants]]
