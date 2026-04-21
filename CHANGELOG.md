# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- **Formation pattern** — replaces `CommitBoundary`; teams of heterogeneous agents (LLMs, optimizers, schedulers) assembled by Organism and run in Converge Engine instances
- **Pipeline wiring** — full intent → admission → adversarial → simulation → formation → converge flow in `organism-runtime`
- **Outcome simulator** — Monte Carlo sampling over plan annotations (impacts + risks) with configurable thresholds
- **DefaultAdmissionController** — evaluates 4 feasibility dimensions: capability, context, resources, authority
- **Axiom enforcement tests** (trybuild) — compile-time proof that `Fact` cannot be forged from Organism
- **Connector Architecture** decision record — three-tier model (Tool/Port/Provider), API-only infrastructure strategy
- **The Gap** philosophy doc — why Organism exists and how formations fill the intent→convergence gap

### Changed
- Converge deps bumped to `branch = "main"` (v3.7.0) — typed ID newtypes (`FactId`, `ProposalId`, `BackendId`) adopted throughout
- `FactId` and `ProposalId` replace `String` in all domain structs: `TrackedHypothesis`, `HypothesisOutcome`, `SimulationVerdict`, `Seed`, processed-ID sets in DD suggestors
- `CONFIDENCE_STEP_*` constants defined in `organism-pack` (pending re-export once converge main is indexed on crates.io)
- `adjust_confidence(delta)` pattern adopted in debate-loop example — replaces magic `with_confidence(n)` floats
- `organism-learning` tests use real `StopReason` and `BudgetResource` enum variants (no string coercions); `converge-core` added as dev-dep to access these types (API gap to close upstream by exporting through `converge-kernel`)
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
