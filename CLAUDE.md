# Organism — Agent Instructions

Organism is the **organizational intelligence runtime** — Layer 2 in the three-layer stack. It sits between human intent and Converge's governed convergence boundary.

```
Human intent → Organism (form, reason, debate, simulate) → Converge (run, promote, govern) → World
```

## Session Scope

- **Milestones:** Read `MILESTONES.md` at the start of every session. Scope work to the current milestone.
- **Changelog:** Update `CHANGELOG.md` when shipping notable changes.
- **Strategic context:** `~/dev/work/EPIC.md`

## Claude-Specific Notes

- **Available skills:** `/experiment` — hypothesis-driven development with evidence logging.
- Prefer `Edit` over `Write` for modifying existing files. Use `Grep`/`Glob` over `Bash` for search.
- Run `just lint` before considering work done.

## Architectural rules

- **Organism is a client of Converge.** Converge does not know Organism exists. Use Converge types (`converge-pack`, `converge-kernel`, `converge-model`) directly — the Rust type system enforces the axioms.
- **Authority is never inherited from reasoning.** It is recomputed at Converge's authority boundary. Planning code must not assume that producing a plan grants the right to execute it.
- **Reasoning, planning, governance, execution are separate layers.** Do not collapse them for convenience.
- **Plans become Formations, not commits.** Organism assembles teams of `Suggestor`s and Converge runs them to fixed point.
- **Everything inside Converge is a `Suggestor`.** Planning, adversarial review, simulation, optimization, policy, analytics, and knowledge all enter through the same loop contract.
- **No wrapper layers over Converge.** Extend through composition (e.g. `OrganismPlan { proposed_fact: ProposedFact, ... }`), not by wrapping the Converge API.

## Crate layout

| Crate | Responsibility |
|---|---|
| `intent` | Intent packets, admission control, intent decomposition |
| `planning` | Huddle (multi-model collaborative planning), debate loop |
| `adversarial` | Assumption breakers, constraint checkers, skeptics |
| `simulation` | Outcome / cost / policy / causal / operational simulation |
| `learning` | Planning priors, calibration, strategy adaptation |
| `runtime` | Formation assembly, runtime wiring, readiness, Converge execution |
| `intelligence` | OCR, vision, web, social, patent, linkedin, billing |
| `notes` | Vault management, source adapters, cleanup, enrichment |
| `domain` | 13 org packs + knowledge lifecycle + 8 blueprints |

## Converge v3.4.x contract

Organism uses Converge types directly. Two deployment modes:

| Mode | Crate | Purpose |
|---|---|---|
| Embedded | `converge-kernel` | In-process: Engine, `ContextState`, `ConvergeResult`, re-exported `Suggestor` contract |
| Authoring | `converge-pack` | Suggestor trait, ProposedFact, Invariant |
| Reading | `converge-model` | Governed semantic types (Fact, Proposal, PromotionRecord) |
| Remote | `converge-client` | gRPC wire protocol (only for out-of-process) |

Do NOT depend on `converge-core`, `converge-runtime`, or other internal crates.

Converge changes that matter to Organism:
- `Suggestor` is the one universal in-loop contract.
- `converge_kernel::Context` is the trait and `ContextState` is the concrete state.
- `Fact` construction is kernel-gated. Consumers construct `ProposedFact`, never `Fact`.
- Sequencing is by dependencies and deterministic registration order, not by suggestor name.

**Before implementing a core/basic/fundamental function, check if Converge already provides it:**
`~/dev/work/converge/CAPABILITIES.md` — optimization solvers, knowledge base, policy engine, analytics/ML, LLM providers, tool integration, experience store, object storage.

See `~/dev/work/converge/kb/Architecture/API Surfaces.md` for the full public contract.

## What Organism MUST NOT do

- Own the 9 convergence axioms (Converge owns them)
- Own the authority / promotion boundary (Converge)
- Own policy and authority primitives (Converge)
- Own traceability and audit (Converge)
- Own packs / business domain truths (Converge)
- Own SaaS product UX (the application layer above)

## Code style

- Edition 2024, `unsafe_code = forbid`, clippy pedantic.
- Run `just lint` before considering work done.
- No feature flags or backwards-compat shims unless asked.
- No mocking Converge in integration tests; use a real instance.

## Source of strategy truth

`~/dev/brand-kb/organism-business/strategy/STRATEGY.md` is canonical. When in doubt about scope or framing, read it.

## Legacy

The pre-restructure crates have been retired. Use the current `crates/` and `examples/` trees as the only supported implementation surface.
