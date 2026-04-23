---
tags: [planning, milestone, formations, compiler, vendor-selection]
---
# Organism Formation Compiler Milestone

Organism's next milestone is a reusable formation compiler, validated through
vendor selection as the first proof wedge.

The milestone is broader than procurement. Vendor selection is the first named
decision product because it is reconciliation-heavy, policy-rich, measurable,
and available as live hackathon discovery material.

## Claim

Organism can turn a business intent into an executable, governed Converge
formation plan, run that plan with correlation and audit intact, and capture a
decision outcome that improves future formation choice.

Converge remains the governed execution kernel. Organism owns the formation
choice, compilation strategy, priors, tournaments, and business-domain decision
products.

## Why This Milestone

Enterprise value is not "agents do more work." The value is a governed decision
layer above fragmented systems of record.

For vendor selection, the current process spans procurement, finance, legal,
security, privacy, operations, spreadsheets, RFP portals, email, meetings, and
contract systems. Humans simplify the process because the full diligence loop is
too expensive to coordinate manually.

The compiler should prove that Organism can restore rigor without restoring the
handoff tax:

- wider evidence collection before shortlist collapse
- typed scoring criteria instead of spreadsheet drift
- compliance checks as first-class gates
- per-role provider routing instead of one generic agent
- decision records with evidence, rationale, policy state, and approvals
- outcome capture for future recall and learning

## Scope

- Formation templates, suggestor descriptors, and provider descriptors are
  separate catalogs with typed join points.
- The compiler assembles complementary teams by coverage, not by requiring every
  member to satisfy every capability.
- Provider routing is role-scoped and can carry `BackendRequirements` for cost,
  latency, sovereignty, compliance, replay, and offline constraints.
- Planning evidence stays separate from business execution evidence.
- Correlation IDs join planning run, execution run, candidate plan, and outcome.
- Vendor selection ships as the first typed decision lifecycle.

## Downstream Consumers

`axiom` and hackathon applications should consume Organism as the decision layer:

- depend on `organism-runtime` for compiler, lifecycle, execution, and outcome
  surfaces
- register concrete executable suggestor factories in the app layer
- keep app-specific UI, artifact ingestion, and writeback outside Organism
- do not depend on local Converge paths or Converge internals
- use Converge only through Organism unless the app has a separate low-level
  kernel embedding need

## Non-Goals

- Generic tournament optimization across every business archetype.
- Autonomous procurement approval.
- Cross-tenant learning.
- Replacing ERP, CRM, legal, or procurement systems of record.
- Full OpenClaw exploration semantics.
- Moving formation genome semantics into Converge.

## Converge Handoff To Use

Organism should consume the public Converge substrate:

- Published Converge crates at `3.7.3`; sibling path dependencies should be
  avoided unless Organism discovers a true upstream contract gap.
- `converge-kernel` for engine, HITL, experience, and formation re-exports.
- `converge-kernel::formation::ProfileSnapshot` for role, output keys, cost,
  latency, capability, and confidence metadata.
- `converge-kernel::formation::SuggestorRole` and
  `SuggestorCapability` for compiler-level matching.
- `converge-provider-api::BackendRequirements` for role-level provider needs.
- `GateDecisionRecorded` as the input stream for future HITL-to-Cedar learning.
- `FormationDecision.correlation_id` and `FormationOutcome.correlation_id` as
  the join keys for planning, execution, tournament candidates, and outcomes.

Sharp edge: Converge's `StoreObserver` may still append envelopes without
tenant or correlation metadata. Organism should use its own observer wrapper
around formation runs so experience events are appended with the formation
run's tenant and correlation IDs.

## Vendor Selection Proof Wedge

The vendor-selection lifecycle is four flows:

| Flow | Business Job | Governed Output |
|---|---|---|
| F1 Frame | define need, constraints, and scoring rubric | `ScoringRubric` + `ShortlistSeed` |
| F2 Source | issue RFP, manage fairness, ingest responses | `NormalizedVendorResponse` + `QALedger` + `EvidenceGapReport` |
| F3 Decide | diligence, evaluation, synthesis, approval | `VendorSelectionDecisionRecord` + `AuditEntry[]` |
| F4 Operate | contract reconciliation and monitoring | `ObligationLedger` + `AuditSnapshot` + `RenewalRecommendation` |

See [[Concepts/Vendor Selection Lifecycle]] for the domain view.

## Validation Gates

This milestone is done only when these are demonstrated locally:

1. Catalogs are real. Formation templates, suggestor descriptors, and provider
   descriptors exist as separate Organism-owned registries.
2. Compilation is coverage-based. A vendor-selection intent compiles into a
   complementary team where different members cover different roles and
   capabilities.
3. Provider routing is per role. Research, extraction, policy, evaluation, and
   synthesis can each carry different backend requirements.
4. Vendor selection is typed. Intake, rubric, evidence sources, compliance
   checks, approval actors, and handoff artifacts are explicit.
5. Decision records are audit-grade. Recommendation, evidence, policy state,
   approval state, and rationale are captured together.
6. Outcomes are learning-ready. The record includes template, roster, provider
   assignments, stop reason, gate triggers, quality signal, tenant, and
   correlation ID fields.
7. HITL is first-class. Human decisions are recorded in a form suitable for
   later Cedar shadow-mode comparison.

## Current Implementation State

The first scaffold exists in `organism-runtime`:

- `FormationCompiler` compiles from formation templates, suggestor descriptors,
  and provider descriptors.
- `vendor_selection_formation_catalog()` exposes F1 Frame, F2 Source, F3
  Decide, and F4 Operate as typed templates.
- `FormationOutcomeRecord` captures the learning-ready run context:
  correlation, tenant, template, roster, provider assignments, stop reason,
  gate triggers, quality signal, and writeback target.
- `FormationExperienceObserver` wraps raw Converge `ExperienceEvent`s into
  envelopes with Organism-owned tenant and correlation metadata.
- `Formation::run_with_event_observer()` lets runtime execution use that
  observer without changing Converge.
- `Runtime::compile_formation()` admits an `IntentPacket` and compiles a
  formation plan from catalogs before any Converge business execution starts.
- `ExecutableSuggestorCatalog` maps compiled `suggestor_id` values to concrete
  suggestor factories. Missing executable mappings fail as typed instantiation
  errors instead of being hidden behind the compiler.
- `Runtime::compile_and_instantiate_formation()` admits, compiles, and builds a
  runnable `Formation` when the executable catalog covers the compiled roster.
- `Runtime::compile_and_run_formation()` runs a single compiled candidate and
  returns a `FormationExecutionRecord` with the compiled plan, Converge result,
  and learning-ready outcome record.
- `examples/formation-compiler` compiles the vendor-selection F3 plan,
  instantiates it through fixture suggestor factories, launches the governed
  Converge run, and emits a draft outcome record.

This is still scaffolding. The compiler can now instantiate and run a single
formation candidate, but the vendor-selection descriptors still point to fixture
suggestors rather than buyer-domain implementations backed by live evidence,
policy, scoring, and writeback integrations. `Runtime::handle` still runs
already-instantiated formations; the next production step is replacing fixture
factories with real buyer-domain suggestors and adding a decision-product runner
that owns writeback and downstream outcome capture.

## Parallel Work Plan

These workstreams can proceed with auto-approval because they are pure
Organism-side scaffolding:

- compiler/catalog types and deterministic unit tests
- vendor-selection lifecycle fixture from the hackathon docs
- per-role provider requirement mapping
- outcome record and correlation metadata structs
- explicit compiled-plan-to-formation instantiation boundary
- single-candidate compile/run wiring over fixture suggestors
- KB alignment and implementation notes

The next human input is needed for domain truth, not compiler scaffolding:

- first buyer-side scenario to model
- exact decision actors and approval authority
- real compliance checks and disqualifiers
- scoring criteria and weights
- systems of record or artifacts for writeback
- downstream outcome signal that proves the decision was good
