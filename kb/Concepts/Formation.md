---
tags: [concepts, architecture, load-bearing]
source: mixed
date: 2026-05-05
---
# Formation

A **Formation** is the unit of work Organism hands to Converge. It is a
composition of Suggestors plus the operational capabilities they collectively
need plus the invariants that must hold during promotion.

> Read Converge's [Plug Boundary](../../../converge/kb/Architecture/Plug%20Boundary.md)
> first. This page is Organism's stance on top of that doctrine — not a
> redefinition. Where the two disagree, Plug Boundary wins.

## The Type

```rust
struct Formation {
    suggestors: Vec<SuggestorId>,
    capabilities: Vec<CapabilityRequirement>,
    invariants: Vec<InvariantId>,
}
```

Three fields, three concerns:

- **`suggestors`** — who contributes. Stable identifiers that name the
  purposeful agents the Formation composes. Never adapter names. Never vendor
  names.
- **`capabilities`** — what the Suggestors collectively need from operational
  infrastructure. Declarative requirements (`LLM with JSON output`,
  `vector recall with EU sovereignty`, `embedding model with 1536 dims`) — not
  Backend type imports. Converge resolves these against the registered Backend
  pool at activation and hands `&dyn Capability` handles to each Suggestor.
- **`invariants`** — what must hold during promotion. References to
  Converge-side `Invariant` definitions. The Formation does not author them; it
  cites them.

A Formation never sees an SDK type. A Suggestor never sees a vendor name unless
it explicitly demands one through a `CapabilityRequirement`.

## PackProfile vs CapabilityRequirement

These live at different layers and must not collapse.

| | `PackProfile` | `CapabilityRequirement` |
|---|---|---|
| **What it answers** | "Which Suggestors speak this kind of organizational concept?" | "What operational infrastructure does this Suggestor need?" |
| **Who reads it** | `FormationGuru` (Stage 3) — selecting Suggestor IDs from the catalog | Converge Engine — resolving Backend handles at activation |
| **Where declared** | On the Suggestor (purpose) | On the Suggestor (operational requirement) |
| **Where consumed** | Organism (selection input) | Converge (Backend pool resolution) |
| **Field on `Formation`?** | No — input to building one | Yes — `capabilities` |

`PackProfile` belongs to **Suggestor selection**. `CapabilityRequirement`
belongs to **Backend selection**. The `FormationGuru` uses PackProfile keywords
plus problem-class facts to pick which Suggestor IDs go into a Formation; the
Engine uses CapabilityRequirements to pick which Backends fulfill the
Formation's operational needs. Two different selection problems, two different
mechanisms.

## Organism Is a Consumer of Ports, Never a Publisher of Providers

Per Plug Boundary, the foundation owns the trait definitions (Ports). Extension
repos (`mnemos`, `prism`, `manifold`) own the implementations (Providers).
Organism is **neither**. Organism is the formation/planning crate that
consumes capabilities through the registry mechanism the foundation provides.

Concrete rule:

> **Organism imports neither `converge-provider` adapters nor extension-repo
> crates.** Capability handles only. A Suggestor that imports an adapter type
> is a bug — declare the requirement, let the runtime resolve the handle.

This is the sharpened version of the older "provider vs port" rule from the
Organism architecture memos. The 3.8 declaration makes it enforceable: contract
surfaces have real names (`converge-provider-api`, transitionally), adapter
crates carry adapter qualifiers, and the registry mechanism does the resolution.

## Targeting the 3.8 Contract

When building Formations, target the coming 3.8 surface, not current 3.7
idioms:

- Admission flows through `converge_kernel::admission::admit_observation`
- Admission inputs and outputs are typed values from `converge-model`
- Suggestors emit through `AgentEffect::builder()…build()`
- No `kernel-authority` Cargo feature
- No raw `Fact` construction; consumers construct `ProposedFact` only

## Cross-References

- Converge: [Plug Boundary](../../../converge/kb/Architecture/Plug%20Boundary.md) — the canonical doctrine
- Converge: [Ports](../../../converge/kb/Architecture/Ports.md) — Suggestor + Backend trait definitions
- Converge: [Providers](../../../converge/kb/Architecture/Providers.md) — extension-repo populated entries
- Converge: ADR-006 (admission boundary), ADR-007 (provider/tool naming), ADR-008
- Organism: [Intent Pipeline](Intent%20Pipeline.md) — where Formations sit in the larger flow
- Organism: [Intent Resolution](Intent%20Resolution.md) — how Truths become Formation inputs
