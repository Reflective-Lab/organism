# Organism

[![CI](https://github.com/Reflective-Lab/organism/actions/workflows/ci.yml/badge.svg)](https://github.com/Reflective-Lab/organism/actions/workflows/ci.yml)
[![Security](https://github.com/Reflective-Lab/organism/actions/workflows/security.yml/badge.svg)](https://github.com/Reflective-Lab/organism/actions/workflows/security.yml)
![coverage](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/kpernyer/0d02060b27bfee904bf5b805102ea382/raw/organism-coverage.json)
[![docs.rs](https://docs.rs/organism-pack/badge.svg)](https://docs.rs/organism-pack)
[![dependency status](https://deps.rs/repo/github/Reflective-Lab/organism/status.svg?style=flat-square)](https://deps.rs/repo/github/Reflective-Lab/organism)
![MSRV](https://img.shields.io/badge/MSRV-1.94.0-blue)
<img alt="gitleaks badge" src="https://img.shields.io/badge/protected%20by-gitleaks-blue">
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Organizational intelligence runtime.** The reasoning layer between human intent and governed execution.

```
┌─────────────────────────────────────────────┐
│  Helm          Decision frameworks          │
├─────────────────────────────────────────────┤
│  Axiom         Truth validation & codegen   │
├─────────────────────────────────────────────┤
│  Organism      Reasoning, planning, debate  │  ← you are here
├─────────────────────────────────────────────┤
│  Converge      Engine, promotion, integrity │
├─────────────────────────────────────────────┤
│  Providers     LLMs, tools, storage         │
└─────────────────────────────────────────────┘
```

Where [Converge](https://github.com/Reflective-Lab/converge) answers *"what actions are allowed to become governed facts?"*, Organism answers *"which team should work on this, and how should it think?"*

```
Human intent → Organism (form, reason, debate, simulate) → Converge (run, promote, govern) → World
```

## The Intent Pipeline

Every intent flows through a mandatory six-stage sequence. No shortcuts, no "trusted plan" exceptions.

```
IntentPacket → Admission (4 dimensions) → Decomposition (intent tree)
  → Huddle (multi-model planning) → Adversarial Review (5 skepticism kinds)
  → Simulation Swarm (5 dimensions) → Formation in Converge
```

### 1. Intent Admission

The system assesses feasibility across four dimensions before committing resources:

- **Capability** — can we do this?
- **Context** — do we have enough information?
- **Resources** — do we have the budget?
- **Authority** — is this permitted?

Verdict: Feasible, FeasibleWithConstraints, Uncertain, or Infeasible. Infeasible intents are rejected early.

### 2. Decomposition

Complex intents break into an `IntentNode` tree. Authority can only **narrow** during decomposition — a subtask never has more authority than its parent.

### 3. Huddle

Multiple reasoners run in parallel — LLM, constraint solver, ML prediction, causal analysis, cost estimation, domain model. Each produces candidate plans. Failures are dropped, survivors proceed to debate.

Organism also models **how a team collaborates**, not just that it collaborates.

#### Static presets

- `CollaborationCharter::huddle()` — strict turn-taking, synthesis, dissent map, done gate
- `CollaborationCharter::discussion_group()` — moderated discussion with lighter decision pressure
- `CollaborationCharter::panel()` — curated expert panel with explicit roles and a demanding done gate
- `CollaborationCharter::self_organizing()` — loose self-organizing "figure it out" mode

#### Dynamic collaboration

Collaboration shapes are not fixed — they are **derived, adaptive, and self-discovering**:

1. **Charter derivation** — `derive_charter(intent, now)` reads 6 complexity signals (reversibility, authority breadth, constraint pressure, forbidden density, time pressure, escalation) and produces a charter with transparent rationale. Irreversible acquisition → Panel/Enforced/Unanimous. Low-stakes exploration → SelfOrganizing/Loose/Advisory.

2. **Topology transitions** — mid-run shape changes driven by convergence signals. Rules fire when evidence clusters (Swarm→Huddle), contradictions spike (Huddle→Panel), stability is reached (Panel→Synthesis), or budget runs low (Any→Tighter). The `CollaborationRunner` re-forms the team when a transition fires.

3. **Shape-as-hypothesis** — the most radical: the collaboration shape itself competes as a hypothesis. Multiple candidate shapes are scored by evidence quality, convergence speed, or contradiction minimization. The learning layer calibrates priors so future derivations are informed by past outcomes. Over time the system discovers collaboration patterns that no human would design.

Those collaboration contracts sit in `organism-pack` with `TeamFormation`,
`CollaborationRole`, `ConsensusRule`, and `TurnCadence`.

`organism-runtime` adds `CollaborationRunner` for binding product-specific
participants to those charters and answering practical questions such as who
contributes, who votes, and who owns the final report.

### 4. Adversarial Review

Institutionalized disagreement. Five kinds of skepticism challenge every plan:

| Skepticism | Asks |
|---|---|
| **Assumption Breaking** | What are the unstated assumptions? |
| **Constraint Checking** | Do declared constraints hold? |
| **Causal Skepticism** | What are the second-order effects? |
| **Economic Skepticism** | What does this really cost? |
| **Operational Skepticism** | Can the organization actually execute this? |

Blocker findings stop the plan. Plans revise, adversaries challenge again — the loop converges when there's nothing left to challenge.

### 5. Simulation Swarm

Five dimensions tested in parallel:

- **Outcome** — does the plan achieve the intent?
- **Cost** — resource consumption envelope
- **Policy** — violations of declared policies?
- **Causal** — second-order effects and confounders
- **Operational** — can the team and systems execute?

Each dimension returns probability distributions, not point estimates. The swarm produces a recommendation: Proceed, ProceedWithCaution, or DoNotProceed.

### 6. Run a Formation in Converge

The surviving team becomes a `Formation`: a labeled team of heterogeneous
`Suggestor`s plus seeded inputs. `Formation::run()` creates a fresh
`converge_kernel::Engine`, registers the team, seeds a `ContextState`, and
calls `Engine.run()`.

Organism has **zero authority** here. It may construct `ProposedFact` or stage
inputs, but it does not construct `Fact`, bypass promotion, or depend on
`converge-core`. Converge recomputes authority at the promotion gate and
returns the governed `ConvergeResult`.

## Organizational Learning

The system learns from execution outcomes. Every completed intent produces a `LearningEpisode` linking the original intent → plan → predicted outcomes → actual outcomes → errors → lessons.

Learning signals flow **backward** into planning priors — never directly into authority. The system learns to plan better, not to bypass governance.

## Intent Resolution

Maps intent to the packs, capabilities, and invariants needed to fulfill it. Four levels, each building on the last:

| Level | How | Confidence |
|---|---|---|
| **Declarative** | App explicitly declares requirements | 1.0 |
| **Structural** | Match fact prefixes to packs (deterministic) | 0.85 |
| **Semantic** | Huddle matches intent to pack descriptions (LLM) | 0.5–0.9 |
| **Learned** | Prior calibration from execution history | Compounds over time |

The flywheel: more intents → more episodes → better Level 4 → fewer manual bindings → faster resolution → more intents processed.

## Domain Packs

Organizational workflow packs encoding how organizations operate. Each defines agents, lifecycles, and invariants.

| Pack | Lifecycle |
|---|---|
| `knowledge` | Signal → Hypothesis → Experiment → Decision → Canonical |
| `customers` | Lead → Enrich → Score → Route → Propose → Close → Handoff |
| `people` | Hire → Identity → Access → Onboard → Pay → Offboard |
| `legal` | Contract → Review → Sign → Execute |
| `autonomous_org` | Policy → Enforce → Approve → Budget → Delegate |
| `performance` | Reviews → Goals → Feedback → Calibration → Compensation |
| `growth_marketing` | Campaign → Channel → Budget → Experiment → Attribution |
| `product_engineering` | Roadmap → Feature → Task → Release → Incident → Postmortem |
| `ops_support` | Ticket → Triage → Route → SLA → Escalate → Resolve |
| `procurement` | Request → Approve → Order → Asset → Subscription → Renewal |
| `partnerships` | Source → Assess → Negotiate → Integrate → Review |
| `virtual_teams` | Team → Persona → Content → Review → Publish |
| `linkedin_research` | Signal → Evidence → Dossier → Path → Approval |
| `reskilling` | Assess → Validate → Plan → Track → Credential |
| `due_diligence` | Research → Extract → Detect Gaps → Synthesize |

### Blueprints

Compose organism-domain packs with Converge foundational packs (trust, money, delivery, data_metrics) into end-to-end workflows:

| Blueprint | Packs Composed |
|---|---|
| `lead_to_cash` | Customers → Delivery → Legal → Money |
| `hire_to_retire` | Legal → People → Trust → Money |
| `procure_to_pay` | Procurement → Legal → Money |
| `issue_to_resolution` | Ops Support → Knowledge |
| `idea_to_launch` | Product Engineering → Delivery |
| `campaign_to_revenue` | Growth Marketing → Customers → Money |
| `partner_to_value` | Partnerships → Legal → Delivery |
| `patent_research` | Knowledge → Legal → IP pipeline |
| `diligence_to_decision` | Due Diligence → Legal → Knowledge |

## Intelligence

Provider-shaped data acquisition from the world. Every result wrapped in `Observation<T>` with correlation ID, latency, cost, and vendor tracking.

| Capability | Providers |
|---|---|
| **OCR** | Tesseract, Apple Vision, Mistral, DeepSeek, LightOn |
| **Vision** | Claude, GPT-4o, Gemini, Pixtral |
| **Web** | URL capture, metadata extraction, HTML parsing |
| **Social** | LinkedIn, X, Instagram, Facebook (normalized profiles) |
| **Patent** | USPTO, EPO, WIPO, Google Patents, Lens |
| **PDF** | Text extraction, chunking, metadata capture |
| **Billing** | Stripe ACP (checkout, payments, metering) |

## Crates

### Public API

| Crate | Role |
|---|---|
| [`organism-pack`](crates/pack) | Curated planning contract — one import, full pipeline semantics |
| [`organism-runtime`](crates/runtime) | Embedding API — registry, resolution, readiness, and Formation execution |
| [`organism-intelligence`](crates/intelligence) | Provider-shaped capabilities: OCR, vision, web, social, patent, billing |
| [`organism-notes`](crates/notes) | Vault lifecycle: ingestion, cleanup, enrichment |
| [`organism-domain`](crates/domain) | Organizational pack library and blueprints |

### Internal Phase Crates

Use these only when extending Organism itself:

| Crate | Role |
|---|---|
| [`organism-intent`](crates/intent) | Intent packets, admission control, decomposition, resolution |
| [`organism-planning`](crates/planning) | Huddle, debate loop, plan annotations, 6 reasoning systems |
| [`organism-adversarial`](crates/adversarial) | Challenge types, 5 skepticism kinds, Skeptic trait |
| [`organism-simulation`](crates/simulation) | 5 simulation dimensions, SimulationRunner trait |
| [`organism-learning`](crates/learning) | Episodes, prediction error, prior calibration |

### Depending on Organism

```toml
[dependencies]
# Planning contract — one import, full pipeline semantics
organism-pack = { path = "../organism/crates/pack" }

# Embedded runtime — resolution, readiness, and Formation execution
organism-runtime = { path = "../organism/crates/runtime" }

# Converge integration
converge-kernel = "3"
converge-pack = "3"
```

## Converge Integration

Organism uses Converge types directly — no wrapper layers. The Rust type system enforces the axioms.

| Mode | Crate | Purpose |
|---|---|---|
| Embedded | `converge-kernel` | In-process engine, `ContextState`, `ConvergeResult`, re-exported `Suggestor` contract |
| Authoring | `converge-pack` | Suggestor trait, ProposedFact, Invariant |
| Reading | `converge-model` | Governed semantic types (Fact, Proposal, PromotionRecord) |
| Remote | `converge-client` | gRPC wire protocol for out-of-process deployment |

Important Converge changes:
- `Suggestor` is the one universal contract. LLMs, optimizers, policy gates, analytics, knowledge, adversaries, and simulators all enter the same loop that way.
- `converge_kernel::Context` is the trait; `ContextState` is the struct embedders create.
- `Fact` construction is kernel-gated. Consumers construct `ProposedFact` and let the engine promote.
- Deterministic ordering follows registration order and dependencies, not name sorting.
- Organism owns `Formation` assembly; Converge owns the governed run.

## Examples

| Example | What it demonstrates |
|---|---|
| [`vendor-selection`](examples/vendor-selection) | Swarm evaluation, multi-criteria scoring, domain pack metadata |
| [`expense-approval`](examples/expense-approval) | Full pipeline: admission → planning → adversarial → simulation |
| [`loan-application`](examples/loan-application) | Parallel eval, all 5 skepticism kinds, 5D simulation, learning capture |
| [`resolution-showcase`](examples/resolution-showcase) | Intent resolution across all 4 levels |
| [`debate-loop`](examples/debate-loop) | Adversarial challenge and plan revision cycle |
| [`collab-huddle`](examples/collab-huddle) | Strict huddle with done-gate voting and validation failures |
| [`collab-panel`](examples/collab-panel) | Curated panel with role matrix and formation enforcement |
| [`collab-self-organizing`](examples/collab-self-organizing) | Solo start → swarm growth, advisory consensus |
| [`collab-discussion`](examples/collab-discussion) | Moderated discussion, full topology/cadence/consensus comparison |
| [`charter-from-intent`](examples/charter-from-intent) | Dynamic charter derivation from intent properties |
| [`topology-transition`](examples/topology-transition) | Mid-run shape changes over a simulated convergence loop |
| [`shape-competition`](examples/shape-competition) | Competing shapes, scoring, winner selection, prior calibration |

Golden path for apps above Organism:

- `organism-pack` for intent, planning, challenge, simulation, and learning semantics
- `organism-runtime` for registry, binding, resolver, readiness, and commit-boundary wiring
- add `organism-domain`, `organism-intelligence`, and `organism-notes` only when the app actually needs them

## Develop

```sh
just build
just test
just lint
```

## License

[MIT](LICENSE) — Copyright 2025–2026 Reflective Group AB
