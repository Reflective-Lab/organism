# Organism — Organizational Intelligence Runtime

This is the canonical agent entrypoint — all agents (Claude, Codex, Gemini, or otherwise) start here. Long-form documentation lives in `kb/`.

## Philosophy

Organism is Layer 2. It sits between human intent and Converge's commit boundary. Read `kb/Philosophy/Why Organism.md` and `kb/Philosophy/Key Invariants.md`.

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
| System logic | Rust (Edition 2024, rust-version 1.94) |
| Converge contract | `converge-pack`, `converge-model`, `converge-kernel` (v3.0.0) |
| Task runner | just |

## Build

```bash
just build      # Build all crates
just test       # Run tests
just lint       # Format + clippy pedantic
just focus      # Session opener
just sync       # Team sync
just status     # Build health
```

## Rules

- No `unsafe` code. Ever.
- Authority is never inherited from reasoning — recomputed at commit boundary.
- Plans must pass adversarial review AND simulation before commit.
- Reasoning, planning, governance, execution are separate layers.
- `just lint` clean before considering work done.
- No feature flags. No backwards-compat shims.
- Use Converge types directly (`converge-pack`, `converge-kernel`, `converge-model`). No wrapper layers.
- Before building a core capability, check `~/dev/work/converge/CAPABILITIES.md` — Converge provides optimization solvers, knowledge base, policy engine, analytics/ML, LLM providers, tool integration, experience store, object storage.
- Do not depend on `converge-core`, `converge-runtime`, or other internal Converge crates.
- No mocking Converge in integration tests; use a real instance.

## Crate Layout

### Core contract
| Crate | Responsibility |
|---|---|
| `pack` | **Public surface** — re-exports the full planning loop contract in one import |

### Planning loop (internal, re-exported via pack)
| Crate | Responsibility |
|---|---|
| `intent` | Intent packets, admission control, decomposition |
| `planning` | Huddle (multi-model planning), debate loop, plan annotations |
| `adversarial` | Challenges, skepticism taxonomy, adversarial signals |
| `simulation` | 5-dimension simulation swarm, runner trait |
| `learning` | Episodes, prediction error, prior calibration |
| `runtime` | Agent orchestration, LLM integration, HITL, commit boundary |

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
| `/focus` / `just focus` | Session opener |
| `/sync` / `just sync` | Team sync |
| `/status` / `just status` | Build health |
| `/checkpoint` | End-of-session |
| `/fix` | Fix a GitHub issue |
| `/ticket` | Create an agent-ready issue |
| `/pr` | Create a pull request |
| `/review` | Review a PR |
| `/merge` | Squash-merge a PR |
| `/wip` | Save and push WIP |

## Legacy

`_legacy/` contains the pre-restructure monolith. Domain packs and planning types have been revitalized into current crates. Do not modify `_legacy/` in place.

## Strategy

Canonical strategy: `~/dev/brand-kb/organism-business/strategy/STRATEGY.md`.

## Milestones

Read `MILESTONES.md` at the start of every session. Scope all work to the current milestone. See `~/dev/work/EPIC.md` for the strategic context (Organism = E2) and `~/dev/work/MILESTONES.md` for the cross-project rollup.
