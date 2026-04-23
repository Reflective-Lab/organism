---
tags: [architecture]
---
# Converge Contract

How Organism uses Converge. See [[Philosophy/Relationship to Converge]] for why.

Shared stack guidance: `~/dev/work/converge/kb/Architecture/Golden Path Matrix.md`.

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
| `converge-provider-api` | Backend identity and role-level `BackendRequirements` |
| `converge-client` | gRPC SDK — only for remote (out-of-process) deployment |

Use published Converge crates for normal development. Local path dependencies
are only for coordinated Converge changes, and those should be exceptional now
that the formation compiler substrate is released.

## Forbidden Dependencies

- `converge-core` — internal engine implementation
- `converge-runtime` — HTTP/gRPC server internals
- `converge-provider` — provider adapter internals
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

## Allowed

- Implement any Organism agent as `Suggestor`
- Mix LLM, optimization, policy, analytics, knowledge, adversarial, and simulation agents in one formation
- Stage initial inputs through `ContextState`
- Read promoted facts from `ConvergeResult`
- Use `register_suggestor()` and `register_suggestor_in_pack()` as the registration surface

## Not Allowed

- Depend on `converge-core`
- Construct `Fact` directly
- Use removed or stale names such as `ContextView`, `Context::new()`, or `register_in_pack(...)`
- Bypass `Engine.run()` for governed fact creation
- Build wrapper types that pretend to replace the Converge surface
- Rely on suggestor name ordering for sequencing; use dependency keys instead

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

See also: [[Architecture/Crate Map]], [[Philosophy/Key Invariants]]
