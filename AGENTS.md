# Organism — Organizational Intelligence Runtime

This is the canonical agent entrypoint — all agents (Claude, Codex, Gemini, or otherwise) start here. Long-form documentation lives in `kb/`.

## Philosophy

Organism is Layer 2. It sits between human intent and Converge's governed convergence boundary. Read `kb/Philosophy/Why Organism.md` and `kb/Philosophy/Key Invariants.md`.

Authority is never inherited from reasoning. Plans must pass adversarial review and simulation before reaching Converge.

## The Knowledgebase

`kb/` is an Obsidian vault. It is THE documentation.

**Do NOT read the entire kb on startup.** Lazy-load:
1. Read `kb/Home.md` only when you need to find something.
2. Follow ONE wikilink to the specific page you need.
3. Never bulk-read `kb/`.

## Stack

| Layer | Technology |
|---|---|
| System logic | Rust (Edition 2024, rust-version 1.90) |
| Converge contract | `converge-pack`, `converge-kernel`, optional `converge-model` / `converge-client` (v3.4.x, rev `40dc92f`) |
| Task runner | just |

## Build

```bash
just build      # Build all crates
just test       # Run tests
just lint       # Format + clippy pedantic
just focus      # Session opener
just sync       # Team sync
```

## Rules

- No `unsafe` code. Ever.
- Authority is never inherited from reasoning — recomputed at Converge's authority boundary.
- Plans must pass adversarial review AND simulation before entering the governed Converge run.
- Reasoning, planning, governance, execution are separate layers.
- Organism assembles `Formation`s. Converge runs them. Do not model Converge as a dumb submission endpoint.
- `just lint` clean before considering work done.
- No feature flags. No backwards-compat shims.
- Use Converge types directly (`converge-pack`, `converge-kernel`, `converge-model`). No wrapper layers.
- `converge_kernel::Context` is the trait. `ContextState` is the concrete state. No `ContextView`, no `Context::new()`.
- Inside Converge there is ONE in-loop contract: `Suggestor`. Do not invent side-car pipeline traits to bypass the engine.
- Before building a core capability, check `~/dev/work/converge/CAPABILITIES.md` — Converge provides optimization solvers, knowledge base, policy engine, analytics/ML, LLM providers, tool integration, experience store, object storage.
- Do not depend on `converge-core`, `converge-runtime`, or other internal Converge crates.
- No mocking Converge in integration tests; use a real instance.

## Crate Layout

### Public surfaces
| Crate | Responsibility |
|---|---|
| `pack` | **Curated planning contract** — re-exports the full planning loop in one import |
| `runtime` | **Curated embedding surface** — runtime wiring, registry, resolution, readiness |

### Planning loop building blocks
| Crate | Responsibility |
|---|---|
| `intent` | Intent packets, admission control, decomposition |
| `planning` | Huddle (multi-model planning), debate loop, plan annotations |
| `adversarial` | Challenges, skepticism taxonomy, adversarial signals |
| `simulation` | 5-dimension simulation swarm, runner trait |
| `learning` | Episodes, prediction error, prior calibration |

Default downstream rule:
- Start with `organism-pack` + `organism-runtime`
- Add `organism-intelligence`, `organism-notes`, and `organism-domain` only if your app needs them
- Reach for `intent`, `planning`, `adversarial`, `simulation`, or `learning` directly only when extending Organism itself

### Capabilities (provider-shaped)
| Crate | Responsibility |
|---|---|
| `intelligence` | OCR, vision, web, social, patent, linkedin, billing |
| `notes` | Vault management, source adapters, cleanup, enrichment |

### Domain packs (pack-shaped)
| Crate | Responsibility |
|---|---|
| `domain` | 13 org packs + knowledge lifecycle + 8 blueprints |

## Workflows

| Workflow | Purpose |
|---|---|
| `/focus` / `just focus` | Session opener — orient yourself, see team activity |
| `/sync` / `just sync` | Team sync — who did what, PRs waiting, unclaimed issues |
| `/next` | Pick next task from backlog |
| `/dev` | Start local development environment |
| `/fix` | Fix a GitHub issue by number |
| `/check` | Run lint, check, and tests |
| `/pr` | Create a pull request |
| `/ticket` | Create an agent-ready issue |
| `/review` | Review a PR |
| `/wip` | Save and push WIP |
| `/done` | End-of-session — update milestones, record what moved |
| `/deploy` | Deploy to staging or production |
| `/audit` | Security, dependency, compliance, and drift audit |
| `/help` | Show available workflows |

### Daily habit

```
Morning:    /focus → /sync → /next
Work:       /fix, /check, /pr
Evening:    /done
Monday:     /audit
Anytime:    /help
```

## Legacy

The pre-restructure monolith has been retired. Current crates and examples are the only supported source of truth.

## Strategy

Canonical strategy: `~/dev/brand-kb/organism-business/strategy/STRATEGY.md`.

## Milestones

Read `MILESTONES.md` at the start of every session. Scope all work to the current milestone. See `~/dev/work/EPIC.md` for the strategic context (Organism = E2) and `~/dev/work/MILESTONES.md` for the cross-project rollup.
