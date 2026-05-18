# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Removed
- Removed the Stripe billing surface from `organism-intelligence`. Billing,
  subscription, entitlement, ledger, and payment-provider reconciliation
  semantics belong to Movement; Stripe webhook transport and secret/runtime
  plumbing belong to Runway.

## [1.9.0] - 2026-05-17

First release on the Converge 3.9 contract, and the first crates.io publish
since 1.5.1. Adds the FuzzyReasoner / MlPredictionReasoner planning surface,
the AnomalySkeptic + GradedAdmissionController adversarial/intent pair, and
per-crate provenance markers that satisfy the Converge 3.9 fact-construction
contract end-to-end.

### Added
- **Planning.** `FuzzyReasoner` and `MlPredictionReasoner` backed by
  `converge-prism-analytics` 2.0 fuzzy inference. Suggestor surface in
  `crates/planning/src/suggestor.rs`, with kb integration notes in
  `kb/Architecture/Prism Analytics Integrations.md`.
- **Adversarial / Intent.** `AnomalySkeptic` agent
  (`crates/adversarial/src/anomaly_skeptic.rs`) and
  `GradedAdmissionController` (in `crates/intent`) form a paired
  admit/challenge loop for graded intent acceptance.
- **Provenance markers.** New `provenance.rs` module in each of
  `adversarial`, `learning`, `planning`, `runtime`, `simulation` crates.
  Each exports a unit struct implementing `converge_pack::ProvenanceSource`
  and a `*_PROVENANCE` constant for `*_PROVENANCE.proposed_fact(...)`
  construction.
- **KB.** `kb/Architecture/Prism Analytics Integrations.md` and
  `kb/Architecture/Specialist Bench Formations.md` document the new
  reasoner surface and formation taxonomy.

### Changed
- **Converge floor: 3.8.1 → 3.9.1.** All fact construction migrates to
  typed `FactPayload` plus `*_PROVENANCE.proposed_fact(...)`; raw-string
  payload construction is gone.
- **Suggestors and adapters across `planning`, `runtime`, `adversarial`,
  `learning`, `simulation`** updated for the typed payload + provenance
  surface.
- **Pack compile-fail tests** updated for the new fact-construction
  contract (`fact_construction_blocked.{rs,stderr}`,
  `fact_no_new.stderr`, `proposed_fact_compiles.rs`).
- **Mosaic extension pins** moved to current crates.io releases:
  arbiter 2.0.1, embassy-pack/linkedin 1.3.0, ferrox 0.7.1,
  manifold 1.1.1, mnemos 1.2.2, prism 2.0.0.
- **Workspace dependency hygiene.** `[patch.crates-io]` block removed
  entirely — every dependency now resolves directly from crates.io,
  no path overrides. Dead `converge-ferrox-server` workspace entry
  removed.
- Workspace bumped 1.8.1 → 1.9.0.

### Notes
Versions 1.6.x – 1.8.x existed as internal/tagged work but were never
published to crates.io. 1.9.0 supersedes them on the registry.

## [1.8.1] - 2026-05-07

Docs-only patch on top of 1.8.0. No API delta. Sharpens the helms
migration handoff so helms picks up the 1.8.x surface eyes-open on the
four nits the reviewer raised against the prior draft.

### Changed
- `kb/Handoffs/2026-05-07 Helms — organism 1.7.0 migration.md`:
  - **Async surface callout.** Pinned that `Runtime::select_formation` /
    `FormationGuru::select` stay sync; only the standalone
    `classify_with_tiebreaker` / `classify_text_with_tiebreaker` helpers
    are async. Helms's `auto_run` either skips the tiebreaker or routes
    via `template_id_for(class)` after pre-classifying — wrapping the
    guru is not an option in 1.8.x.
  - **`RoleStallSuggestor` consumer named.** Diagnostic emission is dead
    telemetry without a consumer; helms's responsibility is to surface
    the fact as a `UserExperienceEvent::UserCorrection` and feed it
    through the audit-trail path. Re-selection on stall is explicitly
    not in 1.8.0.
  - **HITL ADR tradeoffs sketched per shape.** Single-ingress (centralised
    enforcement, but couples HITL to the kernel boundary) vs.
    pre-admission gate at truth-catalog (kernel stays pure, but every
    new admission entry point owns the gate). Helms picks with the cost
    visible.
  - **Per-role descriptor scoring promoted to a planning input.** Helms
    must redesign tournament scoring around whole-template composites
    (`CandidateScore.composite`, `catalog_rank`, `capability_surplus`,
    `cost_hint`) — don't ship a per-role workaround that organism then
    retires in 1.9.0.
  - Step 1 of the migration sequence updated: pin to **1.8.1**, not
    1.8.0, so the doc and the pin agree.
- Workspace bumped 1.8.0 → 1.8.1.

## [1.8.0] - 2026-05-07

Stage 3+ — "Smarter selection". Selection-shaped additions only; no
orchestration, no compilation. The four bullets that pivoted out of
Stage 3's "Automatic tournament" deliverable land here, except per-role
descriptor scoring which is deferred to a future cut (it would need to
modify `FormationCompiler` and `CompiledFormationPlan` — out of scope for
"keep the API stable").

### Added
- **Capability-surplus + cost-aware template scoring in `FormationGuru`.**
  The guru's composite ranking now factors in three signals: catalog match
  rank (still dominant), how many host capabilities the template doesn't
  require (surplus = template "underspending" the host = preferable), and
  the template's organism-side cost class. New types: `templates::CostHint`,
  `templates::cost_hint_for`, `guru::CandidateScore`. `SelectionTrace`
  gains a `scores: Vec<CandidateScore>` field so traces explain the rank.
  Composite formula: `catalog_bonus * 10 + surplus + cost_bonus`.
- **`organism_intent::problem::ClassifierTiebreaker` trait** — Plug Boundary
  for LLM-backed (or otherwise external) tiebreakers. Async; same shape as
  the resolution-ladder Level 3 SemanticMatcher. Paired with new
  `classify_with_tiebreaker(&IntentPacket, &impl ClassifierTiebreaker)` and
  `classify_text_with_tiebreaker(&str, ...)` async helpers. Tiebreaker is
  consulted only when the deterministic keyword pass defaulted; on
  tiebreaker error the result degrades to the deterministic default.
- **`ProblemClassification.tiebroken: bool`** — surfaces whether a
  tiebreaker resolved the ambiguity, so audits and traces can downweight
  classifications that flipped on retry.
- **`organism_runtime::stall::RoleStallSuggestor`** — in-loop observation
  Suggestor that watches one ContextKey and emits a `Diagnostic` fact
  recommending an alternate descriptor when the role bound to that key is
  failing to contribute while convergence is happening elsewhere.
  Idempotent (one stall per role per run). Tunable threshold via
  `with_min_progress`. Observation only — host policy decides whether to
  act on the recommendation.

### Deferred to a future cut (not in 1.8.0)
- Per-role descriptor scoring + per-role decisions in `SelectionTrace`.
  Touches `FormationCompiler` and `CompiledFormationPlan`. Splitting it
  out keeps 1.8.0's surface backwards-compatible.

## [1.7.0] - 2026-05-07

Stage 3 — "Truth-Driven Formation Selection" — plus the contract-boundary
cleanups that completed the axiom split. `1.6.0` is intentionally skipped:
the bridge deletion and the formation-routing work ship as one minor cut.

### Added
- **`organism_intent::problem`** — coarse 7-class taxonomy
  (`ProblemClass::{Decision,Research,Evaluation,Planning,Diligence,Incident,Strategy}`)
  plus a deterministic keyword-based `classify(&IntentPacket)` and the
  underlying `classify_text(&str)`. Tie-breaking favors the most expensive
  misclassification (Incident first, then Diligence). 14 inline tests.
- **`organism_runtime::classifier::ProblemClassifierSuggestor`** — in-loop
  classifier that reads `Seeds`/`Signals` and emits a `Hypotheses` fact
  carrying the problem class. Idempotent; doesn't refire on contexts that
  already carry a `problem-class:` hypothesis. `extract_classification`
  recovers the typed value back from the fact. 5 Formation-driven tests.
- **`organism_runtime::templates`** — five named formation templates
  (`organism-{decision,research,evaluation,planning,diligence}`) wrapping
  `converge_kernel::formation::FormationTemplate`. Each declares roles,
  keywords, entities, and required `SuggestorCapability`s. Plus
  `standard_formation_catalog()` and `template_id_for(ProblemClass)` for
  routing. Incident and Strategy fall back to existing templates. 7 tests.
- **`organism_runtime::guru::FormationGuru`** — picks formation templates
  given an `IntentPacket` and host `SuggestorCapability` inventory.
  Classifies internally, queries the catalog by class-derived keywords,
  post-filters by available capabilities, returns primary + up to 2
  alternates plus a `SelectionTrace` (problem class, matched keywords,
  considered template ids, primary reason). 6 tests.
- **`Runtime::select_formation(&IntentPacket, &FormationCatalog, &[SuggestorCapability])`**
  — auto-mode's selection front half. Manual modes
  (`compile_formation`, `compile_and_run_formation`, `handle`) remain
  unchanged.
- **`scenarios/truth-driven-formation`** in atelier-showcase — three
  Truth fixtures (Decision / Research / Diligence), each parsed by
  `axiom_truth::compile_intent_from_source` and routed by the guru. The
  binary prints the visible selection trace per Truth.

### Removed
- **`organism_intent::bridge` deleted entirely.** `TruthInput`,
  `IntentBlock`, `AuthorityBlock`, `ConstraintBlock`, `EvidenceBlock`,
  `ExceptionBlock`, and `compile_truth` are gone. Truth-shaped types and
  the `TruthDocument → IntentPacket` compiler now live in `axiom-truth 0.8`
  as `axiom_truth::compile_intent`. Callers compile via axiom and pass the
  resulting `IntentPacket` to the runtime.

### Changed
- **`Runtime::resolve_and_admit_truth` → `Runtime::admit_intent(&IntentPacket, ...)`.**
  The runtime no longer compiles Truth; it admits an already-compiled
  `IntentPacket` through Converge's typed admission boundary. The bridge
  step is the caller's responsibility (`axiom_truth::compile_intent`).
- **`TruthAdmissionError` → `IntentAdmissionError`.** The
  `Bridge(BridgeError)` variant is removed; remaining variants (`Rejected`,
  `AdmissionRequest`, `Serialize`, `Converge`) are unchanged.
- **`runtime/tests/truth_admission.rs` → `runtime/tests/intent_admission.rs`.**
  Tests now build `IntentPacket`s directly. Truth-→-IntentPacket coverage
  moved to `axiom-truth/src/intent.rs` (17 inline tests).

### Caller migration
```rust
// Before (1.5.1):
let (intent, receipt) = runtime.resolve_and_admit_truth(&truth_input, actor, src, &mut ctx)?;

// After (1.7.0):
let intent  = axiom_truth::compile_intent(&truth_document)?;  // axiom owns Truth
let receipt = runtime.admit_intent(&intent, actor, src, &mut ctx)?;

// Optional auto-mode selection:
let selection = runtime.select_formation(&intent, &catalog, &capabilities)?;
println!("picked {}: {}", selection.primary.id(), selection.trace.primary_reason);
```

## [1.5.1] - 2026-05-08

Cleanup release that makes the organism crates publishable to crates.io
without external preconditions. No new features; same Phase B contract
surface as 1.5.0.

### Changed
- **Drop `axiom-truth` dependency.** Organism no longer parses `.truths` source
  itself. The bridge now consumes a self-contained `organism_intent::bridge::TruthInput`
  whose fields mirror the canonical Truth Document governance shape;
  upstream tooling (Helms / atelier showcase) parses `.truths` with its own
  toolchain and populates `TruthInput`. `compile_truth_document` /
  `compile_truth_source` are renamed to `compile_truth(&TruthInput)`.
  `Runtime::resolve_and_admit_truth` takes `&TruthInput` instead of
  `&axiom_truth::TruthDocument`.
- **Drop `organism-domain` dependency.** The pack-framework vocabulary
  (`AgentMeta`, `InvariantMeta`, `Pack`, `PackProfile`, `ContextKey`,
  `InvariantClass`) now lives in `organism-pack::pack`. `Registry`'s
  `with_standard_packs` / `register_standard_packs` helpers are removed —
  downstream catalogs (e.g. `organism-domain`) register their own packs into
  a `Registry` directly.

## [1.5.0] - 2026-05-07

Phase B contract closure. Organism is Helms's typed front door: Truth Documents
become governed proposals through `Runtime::resolve_and_admit_truth`, the
resolver ladder traces every binding decision, and `PlanningPriorAgent` learns
from operator approvals, rejections, overrides, corrections, and boundary
adjustments end-to-end. Aligned to Converge 3.8.1 and the new extension
topology (manifold, mnemos, prism, arbiter, atelier-domain, embassy, ferrox).

### Added
- **Truth Document → IntentPacket bridge** —
  `organism_intent::bridge::compile_truth_document` (and `compile_truth_source`
  for raw `.truths` text). Maps `axiom_truth::TruthDocument` into a typed
  `IntentPacket`. 17 unit tests including a real-Truth round-trip.
- **Public admission adapter** —
  `Runtime::resolve_and_admit_truth(truth, actor, source, ctx)` calls
  `converge_kernel::admission::admit_observation` and returns the compiled
  `IntentPacket` plus the `AdmissionReceipt`. Replaces the hand-rolled
  `IntentPacket` construction in `helms/truth-catalog/src/organism.rs`. 4
  integration tests in `crates/runtime/tests/truth_admission.rs`.
- **IntentResolver Levels 3 and 4** in `organism-intent`:
  - `SemanticResolver` + `SemanticMatcher` trait (Level 3, constructor-injected
    matcher per the Plug Boundary doctrine — no vendor adapter imports).
  - `LearnedResolver` + `EpisodeRecall` trait + `EpisodeSummary` projection
    (Level 4, biases pack confidence by historical success).
  - `LadderResolver` composes `Vec<Box<dyn IntentResolver>>` and recomputes
    `ResolutionTrace.completeness_confidence` from levels attempted vs
    contributed.
- **Bidirectional ExperienceStore consumption** — `PlanningPriorAgent`
  consumes all 5 user-event variants from Converge 3.8.1
  (`UserApprovalGranted`, `UserApprovalRejected`, `UserOverrideIssued`,
  `UserCorrection`, `UserBoundaryAdjusted`) through `consult_recall`. Each
  variant produces a typed `RecallCandidate` with the spec'd
  `(source_type, confidence)` mapping. 6 integration tests in
  `crates/learning/tests/bidirectional_variants.rs`.
- **Recall biases synthesis proposals** —
  `crates/runtime/tests/recall_biases_synthesis.rs` pairs `PlanningPriorAgent`
  with `RoundSynthesizer` and asserts the synthesis `ProposedFact` content
  reflects recall avg confidence (or falls back to `no_recall` when the store
  is empty).
- **`dd_complete()` helper** in `organism-planning::dd` — bridges DD
  Suggestor prompts to `converge_provider::DynChatBackend`, mapping `LlmError`
  variants into `DdError` (RateLimited / CreditsExhausted / ProviderUnavailable
  / PromptTooLarge / BadResponse). Single typed boundary for all DD LLM access.
- **Concept docs** — `kb/Concepts/Formation.md` (the
  `SuggestorId + CapabilityRequirement + InvariantId` contract;
  PackProfile-vs-CapabilityRequirement layering),
  `kb/Concepts/Bidirectional ExperienceStore.md` (all 5 user-event variants
  with confidence/source-type table).
- **Audit + handoff docs** — `kb/Audits/2026-05-06 parse_content.md`
  (categorises typed-vs-untyped JSON parses across the workspace);
  `kb/Handoffs/2026-05-06 Converge — Three UserExperienceEvent variants.md`
  (the brief that drove Converge 3.8.1's three new variants).
- **Converge extension declarations** — workspace `Cargo.toml` now declares
  aliased deps for all 7 extensions targeting Converge 3.8.1: `arbiter`,
  `atelier-domain`, `embassy-pack`, `embassy-linkedin`, `ferrox`,
  `ferrox-server`, `manifold`, `mnemos`, `prism`. Aliases match each crate's
  `[lib] name`; package names are the canonical `converge-*` prefix.

### Changed
- **Converge floor bumped to 3.8.1.** Foundation crates resolve from
  crates.io 3.8.1 directly. Foundation `[patch.crates-io]` entries dropped;
  patches retained only for unreleased extension crates and the local axiom
  checkout.
- **`converge-provider-api` → `converge-provider`** — contract crate renamed
  per ADR-007. All Organism imports migrated.
- **`Fact` → `ContextFact`** — Organism's references to `converge_pack::Fact`
  migrated to `ContextFact` (the type Converge 3.8 actually exports).
- **ContextFact field access** — `.content`, `.id`, `.key` are now accessor
  methods, not public fields. Migrated across `learning`, `adversarial`,
  `simulation`, `planning`, `runtime`, `pack` (~30 files, ~80 sites).
- **AgentEffect construction** — every Suggestor's `execute()` returns
  `AgentEffect::builder()…build()` instead of `with_proposal[s]`. Mechanical
  migration across all 32 sites in adversarial / learning / planning /
  simulation / runtime.
- **`ConsensusRule::passes` typed args** — 27 call sites in
  `planning::collaboration` migrated from raw `(yes, total)` to
  `VoteTally::new(...)` + `EligibleVoters::new(...)`. The
  `consensus_with_zero_voters` test was deleted because `EligibleVoters` is
  `NonZeroUsize` — the type system now enforces what the test was checking.
- **`UnitInterval` adoption** — every Organism field with `[0,1]` semantics
  now uses `converge_pack::UnitInterval`:
  `PackRequirement.confidence`, `CapabilityRequirement.confidence`,
  `ResolutionTrace.completeness_confidence`, `Lesson.confidence`,
  `LearningSignal.weight`, `PriorCalibration.{prior,posterior}_confidence`,
  `DimensionResult.confidence`, `SimulationResult.overall_confidence`,
  `SimulationVerdict.confidence`. Wire format unchanged
  (`#[serde(transparent)]`).
- **Typed JSON deserialization at the convergence boundary** — 5 parse sites
  in `runtime::huddle` migrated from `serde_json::from_str::<T>(fact.content())`
  to `fact.parse_json_content::<T>()` (`Disagreement`, `Vote`,
  `ConsensusOutcome`, `DisagreementMap`).
- **Compile-fail proofs** in `crates/pack/tests/compile_fail/` updated for
  the 3.8 surface: `ContextFact` (was `Fact`), no `construct_unchecked`
  constructor reachable, no field-by-field construction of authoritative
  facts.
- **`organism-domain` path** — moved to
  `~/dev/reflective/stack/mosaic-extensions/atelier-showcase/crates/organism-domain` (workspace
  directory rename). Crate name unchanged.

### Removed
- **`pub trait DdLlm`** and **`pub struct FailoverDdLlm`** —
  per the Plug Boundary doctrine, DD Suggestors take
  `Arc<dyn converge_provider::DynChatBackend>` directly. Engagements
  (e.g. `monterro-core`) need to migrate their `DdLlm` impls to
  `DynChatBackend` when picking up 1.5.0.
- **`organism-intelligence::linkedin`** module — extracted to
  `embassy-linkedin` (`~/dev/reflective/stack/mosaic-extensions/embassy-ports/crates/linkedin`).
  Embassy owns source-specific connector ports.
- **LinkedIn readiness probe** — `CredentialProbe::with_standard_checks` no
  longer requires `LINKEDIN_API_KEY`. Consumers that want a LinkedIn probe
  add it explicitly via the embassy crate.
- **Zero `kernel-authority` feature usage** verified across Organism. ADR-006
  drift no longer present.

### Known issues / planned for 1.6.0

These are pre-existing leaks in `organism-runtime` that the audit on
2026-05-07 surfaced. They don't block 1.5.0 (Helms can adopt today) but are
queued for a focused cleanup release.

- **`runtime::vendor_selection`** (~214 lines) is domain-shaped content
  sitting in the runtime crate. Will move to `atelier-showcase` in 1.6.0.
- **`runtime::registry::with_standard_packs`** hardcodes 13 specific business
  pack names from `organism-domain`. The generic `Registry` machinery stays;
  the standard-pack roster will move into `organism-domain` so callers
  register their pack catalog explicitly.
- **`FormationGuru`** (auto-selection from a catalog given a problem class)
  and named templates (`Decision`, `Research`, `Evaluation`, `Diligence`)
  ship in 1.6.0.

## [1.4.1] - 2026-05-05

### Added
- **`HuddleInvocation` envelope** in `organism-planning` — domain-agnostic
  request to convene a huddle (`subject_id`, `kind`, `urgency`, free-form
  `triggers`, `rationale`, `correlation_id`, `reviewer`, optional
  `domain_context: serde_json::Value`). `HuddleInvocationKind` covers
  `Contested | Sensitive | HighRisk | AiAssisted`; `HuddleUrgency` covers
  `Routine | Elevated | Urgent`. Re-exported from `organism-pack`. Domain
  classifiers (e.g. journalism rules over `ClaimRecord`/`ArticleDraft`) stay
  in consuming crates and emit this envelope at the boundary.
- **Runtime huddle suggestors** — `RoundStarter`, `ConsensusEvaluator`,
  `RoundSynthesizer`, and `DisagreementMapper` are available upstream for apps
  that want reusable huddle-loop mechanics without carrying app-domain voice.
- **Canonical result constructors** for intent admission, adversarial review,
  and simulation dimensions/summaries.

### Changed
- Converge dependencies now target published `3.7.6` crates without local
  `[patch.crates-io]` overrides, so release CI resolves the same published
  contract as downstream consumers.
- Vendor-selection and spend-approval Suggestors moved into reusable domain
  packs; the examples now consume pack-level Suggestors instead of owning the
  reusable business mechanics.
- The loan-application example graduated out of Organism into
  `~/dev/reflective/stack/marquee-apps/loan-application`, keeping lending vocabulary and underwriting
  assumptions in the app layer.
- `PlanningPriorAgent` can consult Converge recall through `ExperienceStore`,
  closing the experience-to-planning feedback loop.

## [1.4.0] - 2026-04-23

### Added
- **Formation compiler** — Organism-owned compile step from formation templates,
  suggestor descriptors, and provider descriptors into auditable formation plans.
- **Executable suggestor catalog** — compiled `suggestor_id` values now resolve
  explicitly to concrete suggestor factories before execution.
- **Vendor selection proof wedge** — F1 Frame, F2 Source, F3 Decide, and F4
  Operate templates plus a runnable F3 compiler example.
- **Formation outcome records** — learning-ready run records with template,
  roster, provider assignments, tenant, correlation, stop reason, gates, quality
  signal, and writeback target.
- **Run-scoped experience observer** — wraps Converge events with Organism-owned
  tenant and correlation metadata.
- **Single-candidate compile/run path** — `Runtime::compile_and_run_formation`
  admits, compiles, instantiates, executes, and returns a `FormationExecutionRecord`.
- **Formation pattern** — replaces `CommitBoundary`; teams of heterogeneous agents (LLMs, optimizers, schedulers) assembled by Organism and run in Converge Engine instances
- **Pipeline wiring** — full intent → admission → adversarial → simulation → formation → converge flow in `organism-runtime`
- **Outcome simulator** — Monte Carlo sampling over plan annotations (impacts + risks) with configurable thresholds
- **DefaultAdmissionController** — evaluates 4 feasibility dimensions: capability, context, resources, authority
- **Axiom enforcement tests** (trybuild) — compile-time proof that `Fact` cannot be forged from Organism
- **Connector Architecture** decision record — three-tier model (Tool/Port/Provider), API-only infrastructure strategy
- **The Gap** philosophy doc — why Organism exists and how formations fill the intent→convergence gap

### Changed
- Converge dependencies now use published `3.7.3` crates instead of GitHub or
  sibling path dependencies.
- `FactId` and `ProposalId` replace `String` in all domain structs: `TrackedHypothesis`, `HypothesisOutcome`, `SimulationVerdict`, `Seed`, processed-ID sets in DD suggestors
- `CONFIDENCE_STEP_*` constants defined in `organism-pack` (pending re-export once converge main is indexed on crates.io)
- `adjust_confidence(delta)` pattern adopted in debate-loop example — replaces magic `with_confidence(n)` floats
- `organism-learning` tests use `StopReason` and `BudgetResource` through
  `converge-kernel`; the direct `converge-core` dev dependency is removed.
- `outcome_signal_note` takes `&OutcomeEventView` (was by-value); all clippy pedantic warnings resolved
- Converge deps bumped to rev `a277ab3` (ContextState rename, optimization/policy Suggestors)
- Removed `CommitBoundary` trait — replaced by `Formation` which directly uses Converge's Engine
- `organism-learning` tests updated for `Context` → `ContextState` rename

## [1.3.0] - 2026-04-19

### Added
- **Dynamic charter derivation** — `derive_charter(intent, now)` reads 6 complexity signals from an `IntentPacket` and produces a `CollaborationCharter` with per-field rationale; `derive_charter_with_priors()` integrates historical shape calibration
- **Topology transitions** — data-driven mid-run shape changes with 5 trigger types (`EvidenceClustering`, `ContradictionSpike`, `StabilityReached`, `BudgetPressure`, `ConsensusDeadlock`) and 4 canonical transition rules (Swarm→Huddle, Huddle→Panel, Panel→Synthesis, Any→Tighter)
- **Shape-as-hypothesis** — collaboration shapes compete as hypotheses; `generate_candidates()` produces 2–3 shapes, `score_observation()` evaluates 4 metrics, `calibrate_shape()` converges priors over episodes
- `CollaborationCharter` builder methods: `with_discipline`, `with_topology`, `with_minimum_members`, `with_formation_mode`, `with_expected_roles`
- `CollaborationRunner::transition()` for runtime team re-formation with transition log
- `TransitionRecord` for tracking topology change history
- 7 new collaboration examples: huddle, panel, discussion, self-organizing, charter-from-intent, topology-transition, shape-competition
- `proptest` workspace dependency and property-based tests across planning, learning, and runtime
- Comprehensive negative tests and edge case coverage

### Changed
- Converge deps bumped to v3.4.0 (new `metadata` field on `ExperienceEvent::OutcomeRecorded`)
- Fixed all clippy pedantic warnings across the workspace
- Fixed non-deterministic `HashMap` iteration in `CollaborationRunner` tests

## [0.1.0] - 2026-04-14

### Added
- Debate loop, pack profiles, multi-dimension resolver
- Intent packet types with ForbiddenAction, ExpiryAction, reversibility
- Admission control types (AdmissionController trait, 4 feasibility dimensions)
- Intent decomposition (IntentNode tree with authority narrowing)
- Plan annotations (impact, cost, risk modeling)
- Huddle scaffold (HuddleParticipant/Reasoner traits, 6 reasoning systems)
- Adversarial types (Challenge, 5 SkepticismKinds, Skeptic trait)
- Simulation swarm types (5 dimensions, SimulationRunner trait)
- Learning episode types (prediction error, lesson, prior calibration)
- Knowledge lifecycle pack (Signal → Hypothesis → Experiment → Decision → Canonical)
- 13 organizational packs + 8 blueprints
- Billing module (Stripe ACP types, from converge-runtime)
- Vision module (Claude, GPT-4o, Gemini, Pixtral)
- OCR module (photos, screenshots ingestion)
- PDF text extraction for text-native PDF ingestion
- Notes enrichment pass for freshness and value analysis
- API surface documentation for curated `organism-pack` + `organism-runtime` consumption
- Codex and workflow documentation refresh
- Standard runtime registry bootstrap for built-in domain packs

### Changed
- Restructured as organizational intelligence runtime (9 crates)
- Curated runtime surface now re-exports registry and readiness APIs
- Resolution showcase now uses the public runtime API path
- Monterro-aligned downstream consumption pattern: `organism-pack` + `organism-runtime`

## 2026-04-06

### Added
- Intelligence crate, kb, and workflow
- Initial organism workspace
