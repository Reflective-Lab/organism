---
tags: [architecture]
---
# API Surfaces

Organism should feel familiar next to Converge. The intended downstream API is
therefore split into a few curated crates instead of asking applications to
assemble their own imports from internal building blocks.

If a downstream application can avoid depending on a lower-level phase crate,
it should.

## Public Contracts

Organism exposes five intended downstream-facing Rust surfaces:

1. Planning contract
2. Embedded runtime
3. Provider-shaped intelligence capabilities
4. Note lifecycle capabilities
5. Organizational pack library

These contracts are intentionally separate. A pack consumer should not need to
depend on each individual planning phase crate. A notes consumer should not
need runtime wiring. A runtime embedder should not need to reach into private
module paths to get registry and readiness helpers.

## Supported Rust Crates

### `organism-pack`

Purpose:
- expose the curated planning-loop contract

What downstream code should use it for:
- `IntentPacket`
- `AdmissionResult`
- `Plan`
- `Challenge`
- `SimulationResult`
- `LearningEpisode`
- `CollaborationCharter`
- `TeamFormation`
- `CollaborationRole`
- `ConsensusRule`
- `TurnCadence`
- shared intent and resolution vocabulary re-exported from the internal phase crates

Status:
- canonical Organism planning contract
- preferred downstream entrypoint for planning semantics

### `organism-runtime`

Purpose:
- embed Organism in-process and connect it to Converge

What downstream code should use it for:
- `Runtime`
- `CommitBoundary`
- `Registry`
- `StructuralResolver`
- `DeclarativeBinding`
- `IntentBinding`
- `check_readiness`
- `CredentialProbe`
- `PackProbe`
- `BudgetProbe`

Status:
- canonical embedded runtime surface
- preferred entrypoint for resolution and readiness

### `organism-intelligence`

Purpose:
- provide provider-shaped data acquisition capabilities

What downstream code should use it for:
- OCR, vision, web, social, linkedin, patent, and billing adapters
- provenance and secret helpers

Status:
- optional capability library

### `organism-notes`

Purpose:
- provide reusable note and vault lifecycle capabilities

What downstream code should use it for:
- vault CRUD
- note ingestion
- cleanup and enrichment passes

Status:
- optional capability library

### `organism-domain`

Purpose:
- provide built-in organizational packs and blueprints

What downstream code should use it for:
- built-in pack metadata and organizational workflow library

Status:
- optional pack library
- commonly paired with `organism-runtime::Registry::with_standard_packs()`

## Building-Block Crates

These crates are real and important, but most downstream application code
should not depend on them directly:

- `organism-intent`
- `organism-planning`
- `organism-adversarial`
- `organism-simulation`
- `organism-learning`

They are the internal building blocks re-exported by `organism-pack` and, for
resolution/readiness concerns, selectively surfaced by `organism-runtime`.
Direct dependencies are reasonable when extending Organism itself, adding new
examples inside this repo, or working on the planning loop implementation.

## Who Uses What

| Consumer | Preferred Dependencies |
|---|---|
| App using Organism planning | `organism-pack`, `organism-runtime` |
| App using built-in packs | `organism-pack`, `organism-runtime`, `organism-domain` |
| App using world-facing capabilities | `organism-intelligence` and/or `organism-notes` |
| Axiom / Helm / operator-facing apps using Organism | `organism-pack`, `organism-runtime`, then optional `organism-domain` / `organism-intelligence` / `organism-notes` |
| Organism contributors extending a phase | specific phase crate(s) plus `organism-pack` as needed |

Current reference downstream:
- Monterro consumes Organism as `organism-pack` + `organism-runtime`
- Monterro consumes Converge as `converge-pack` + `converge-kernel`
- Apps above Organism should follow the same shape rather than importing Organism phase crates directly

## Converge Alignment

The intended mental model matches Converge:

| Concern | Converge | Organism |
|---|---|---|
| Curated semantic contract | `converge-pack` / `converge-model` | `organism-pack` |
| Curated in-process embedding | `converge-kernel` | `organism-runtime` |
| Optional adapters/capabilities | `converge-provider` and others | `organism-intelligence`, `organism-notes`, `organism-domain` |

This symmetry is deliberate. A developer moving between Converge and Organism
should see the same pattern: curated top-level surfaces first, internal crates
only when working on internals.
