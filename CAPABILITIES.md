# Organism — Capability Catalog

> This file is the menu. Apps built on Organism read this to discover what's available.
>
> If you need a capability that isn't here, file a request:
> `gh issue create --repo Reflective-Labs/organism.zone --label capability-request --title "Request: <capability>"`
>
> Before building something fundamental, check what **Converge** already provides:
> `../converge/CAPABILITIES.md` — optimization solvers, knowledge base, policy engine,
> analytics/ML, 14 LLM providers, tool integration, experience store, object storage.

---

## Try it now

Three end-to-end examples show the full organism planning loop running on converge-kernel:

```bash
cargo run -p example-vendor-selection    # swarm evaluation → consensus → converge
cargo run -p example-expense-approval    # intent → admission → adversarial → budget simulation
cargo run -p example-loan-application    # parallel eval → 5 skepticism kinds → 5D simulation → learning
```

Each example wires organism types (`IntentPacket`, `Challenge`, `SkepticismKind`, `DimensionResult`, `SimulationRecommendation`, `LearningEpisode`) as real `Suggestor` implementations on the Converge engine.

---

## How to depend

```toml
[dependencies]
# Planning contract — one import, full pipeline semantics
organism-pack = { path = "../organism/crates/pack" }

# Embedded runtime API — resolution, readiness, runtime wiring
organism-runtime = { path = "../organism/crates/runtime" }

# Capabilities
organism-notes = { path = "../organism/crates/notes", features = ["cleanup", "sources-web"] }
organism-intelligence = { path = "../organism/crates/intelligence", features = ["ocr", "vision"] }

# Domain pack library and blueprints
organism-domain = { path = "../organism/crates/domain" }

# Converge integration (run the engine)
converge-kernel = { path = "../converge/crates/kernel" }
converge-pack = { path = "../converge/crates/pack" }
```

Recommended downstream shape mirrors Converge:
- `organism-pack` for the planning contract
- `organism-runtime` for in-process embedding, resolution, and readiness
- `organism-intelligence`, `organism-notes`, and `organism-domain` as optional libraries

`organism-pack` gives you the full planning contract: `IntentPacket`, `Challenge`, `SimulationResult`, `LearningEpisode`, and the resolution types. App code should usually not depend directly on `organism-intent`, `organism-planning`, `organism-adversarial`, `organism-simulation`, or `organism-learning` unless you are extending Organism itself.

---

## Planning Loop

Organism's unique value. Plans get argued over, stress-tested, and defended before reaching Converge.

```
IntentPacket → Admission (4 dimensions) → Huddle → Adversarial Review (5 skepticism kinds)
  → Simulation Swarm (5 dimensions) → Decision → Learning Episode → Converge commit
```

| Crate | What it does | Key types |
|---|---|---|
| `organism-intent` | Intent admission and decomposition | `IntentPacket`, `AdmissionResult`, `FeasibilityDimension` (Capability/Context/Resources/Authority), `IntentNode` tree, `Reversibility`, `ExpiryAction` |
| `organism-planning` | Multi-model collaborative planning | `Plan`, `PlanAnnotation` (impact/cost/risk), `Reasoner` trait, `Huddle`, `ReasoningSystem` (6 kinds), `PlanBundle` |
| `organism-adversarial` | Institutionalized disagreement | `Challenge`, `SkepticismKind` (AssumptionBreaking/ConstraintChecking/CausalSkepticism/EconomicSkepticism/OperationalSkepticism), `Severity`, `AdversarialSignal`, `Skeptic` trait |
| `organism-simulation` | Parallel stress-testing | `SimulationResult`, `DimensionResult`, `SimulationDimension` (Outcome/Cost/Policy/Causal/Operational), `SimulationRecommendation` (Proceed/ProceedWithCaution/DoNotProceed), `SimulationRunner` trait |
| `organism-learning` | Calibrate priors from outcomes | `LearningEpisode`, `PredictionError`, `ErrorDimension`, `Lesson`, `PriorCalibration`, `LearningSignal` |
| `organism-runtime` | Embedded runtime surface | `Runtime`, `CommitBoundary`, `Registry`, `StructuralResolver`, `check_readiness`, built-in probes |

The five phase crates above are the building blocks behind `organism-pack`. App code should normally import the re-exported types from `organism-pack` and the embedding helpers from `organism-runtime`.

**How apps use this:** Build `Suggestor` implementations that use `organism-pack` types for structured reasoning, and use `organism-runtime` for registry/resolution/readiness concerns. See `examples/expense-approval` and `examples/resolution-showcase` for the intended pattern.

---

## Capability Crates

### organism-notes

Vault-native note lifecycle. CRUD, ingestion, cleanup, enrichment.

| Module | Feature flag | Status | What it does |
|---|---|---|---|
| `vault` | *(always)* | **Live** | ObsidianVault: create/read/save/move notes, tree listing, markdown import, pipeline stages, frontmatter freshness |
| `sources::markdown` | *(always)* | Stub | Import a markdown directory into the vault |
| `sources::apple_notes` | `sources-apple-notes` | **Live** | macOS Apple Notes ingestion via AppleScript — scan, batch export, reuse detection, inline image extraction |
| `sources::web` | `sources-web` | **Live** | URL capture → raw snapshot + vault note, uses organism-intelligence web provider |
| `cleanup` | `cleanup` | **Live** | Exact duplicate detection, Jaccard similarity candidates, merge suggestions |
| `enrichment` | `enrichment` | Partial | Freshness/value analysis live. More derived passes planned: structure extraction, entity extraction, OCR hookup |
| `indexing` | — | Planned | Backlinks, chunks, embeddings, attachment fingerprints, provenance |

**Deps:** `chrono`, `serde`, `thiserror`. Optional: `base64`, `html2md`, `organism-intelligence`.

---

### organism-intelligence

Provider-shaped data acquisition. API adapters that produce observations with provenance.

| Module | Feature flag | Status | What it does |
|---|---|---|---|
| `provenance` | *(always)* | **Live** | `Observation<T>` wrapper with correlation ID, latency, cost, vendor/model tracking |
| `secret` | *(always)* | **Live** | `SecretString` — redacts on Debug |
| `ocr::cloud` | `ocr` | **Live** | Mistral OCR, DeepSeek, LightOn — cloud document understanding |
| `ocr::local` | `ocr` | **Live** | Tesseract, Apple Vision — local OCR backends |
| `ocr::receipt` | `ocr` | **Live** | Receipt-specific extraction (TesseractCli, Ollama) |
| `ocr::photos` | `ocr` | **Live** | Photo ingestion with OCR |
| `ocr::screenshots` | `ocr` | **Live** | Screenshot ingestion with UI chrome detection |
| `pdf` | `pdf` | **Live** | Text-native PDF extraction, chunking, metadata capture |
| `vision` | `vision` | **Live** | Scene understanding — Claude, GPT-4o, Gemini, Pixtral |
| `web` | `web` | **Live** | URL capture, metadata extraction, HTML parsing |
| `social` | `social` | **Live** | Normalized social profile extraction (LinkedIn, X, Instagram, Facebook) |
| `linkedin` | `linkedin` | **Live** | Professional network research provider trait + stub |
| `patent` | `patent` | **Live** | IP landscape search (USPTO, EPO, WIPO, Google Patents, Lens) trait + stub |
| `billing` | `billing` | Partial | Stripe ACP types (checkout, payments, metering). Client/ledger/webhook pending |

**Deps:** `serde`, `thiserror`. Optional: `reqwest`, `base64`, `sha2`, `url`.

---

### organism-domain

Organizational workflow packs. Agents, invariants, and blueprints for autonomous organizations.

| Pack | Status | Lifecycle |
|---|---|---|
| `knowledge` | **Live** | Signal → Hypothesis → Experiment → Decision → Canonical |
| `customers` | **Live** | Lead → Enrich → Score → Route → Propose → Close → Handoff |
| `people` | **Live** | Hire → Identity → Access → Onboard → Pay → Offboard |
| `legal` | **Live** | Contract → Review → Sign → Execute |
| `performance` | **Live** | Review → Goals → Feedback → Calibration → Compensation |
| `autonomous_org` | **Live** | Policy → Enforce → Approve → Budget → Delegate |
| `growth_marketing` | **Live** | Campaign → Channel → Budget → Experiment → Attribution |
| `product_engineering` | **Live** | Roadmap → Feature → Task → Release → Incident → Postmortem |
| `ops_support` | **Live** | Ticket → Triage → Route → SLA → Escalate → Resolve |
| `procurement` | **Live** | Request → Approve → Order → Asset → Subscription → Renewal |
| `partnerships` | **Live** | Source → Assess → Negotiate → Integrate → Review |
| `virtual_teams` | **Live** | Team → Persona → Content → Review → Publish |
| `linkedin_research` | **Live** | Signal → Evidence → Dossier → Path → Approval |
| `reskilling` | **Live** | Assess → Validate → Plan → Track → Credential |

| Blueprint | Packs composed |
|---|---|
| `lead_to_cash` | Customers → Delivery → Legal → Money |
| `hire_to_retire` | Legal → People → Trust → Money |
| `procure_to_pay` | Procurement → Legal → Money |
| `issue_to_resolution` | Ops Support → Knowledge |
| `idea_to_launch` | Product Engineering → Delivery |
| `campaign_to_revenue` | Growth Marketing → Customers → Money |
| `partner_to_value` | Partnerships → Legal → Delivery |
| `patent_research` | Knowledge → Legal → IP pipeline |

When wired to Converge, pack agents implement `converge_pack::Suggestor`. Blueprints compose organism-domain packs with converge-domain foundational packs (trust, money, delivery, data_metrics).

---

## Examples

| Example | What it demonstrates | Run |
|---|---|---|
| `vendor-selection` | Swarm evaluation, multi-criteria scoring, consensus, domain pack metadata | `cargo run -p example-vendor-selection` |
| `expense-approval` | Full pipeline: intent admission (4D) → policy planning → adversarial review → budget simulation (3D) | `cargo run -p example-expense-approval` |
| `loan-application` | Parallel eval (4 agents) → all 5 skepticism kinds → 5D simulation → learning episode capture | `cargo run -p example-loan-application` |

Each example uses `converge-kernel::Engine` with organism types as real `Suggestor` implementations. Copy the pattern for your own domain.

Current reference downstream for the curated API shape: Monterro consumes Organism as `organism-pack` + `organism-runtime` and Converge as `converge-pack` + `converge-kernel`.

---

## Requesting a capability

If your app needs something organism doesn't have:

1. Check this file — it might exist under a feature flag you haven't enabled
2. Check `../converge/CAPABILITIES.md` — it might be infrastructure Converge already provides
3. Check `_legacy/` — it might have patterns to revitalize
4. File a request: `gh issue create --repo Reflective-Labs/organism.zone --label capability-request`

Include: what your app needs, what contract it expects (trait? types? function?), and urgency.

---

## What does NOT belong here

- Tauri commands, Svelte routes, app-specific UX → your app
- Converge kernel internals (axioms, authority, commit boundary) → converge
- Foundational state machines (trust, money, delivery, data_metrics) → converge-domain
- Optimization solvers, LLM providers, policy engine, object storage → converge
