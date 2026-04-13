---
tags: [architecture]
---
# Crate Map

All crates live under `crates/`. Version 0.1.0.

## Planning Loop

```
intent           (no internal deps)     Intent packets, admission, decomposition
planning         → intent               Huddle, debate loop, plan annotations
adversarial      (no internal deps)     Challenges, skepticism taxonomy, adversarial signals
simulation       (no internal deps)     Dimension results, simulation runner trait
learning         (no internal deps)     Episodes, prediction error, prior calibration
runtime          → all above            Agent orchestration, LLM integration, HITL, commit boundary
```

## Capabilities (provider-shaped — acquire data from the world)

```
intelligence     (no internal deps)     OCR, vision, web, social, patent, linkedin, billing
```

## Domain Packs (pack-shaped — encode reusable org workflows)

```
domain           (no internal deps)     13 organizational packs + 8 blueprints + knowledge lifecycle
```

## Converge Integration

Organism uses Converge types directly — no wrapper crates. The Rust type system enforces the axioms.

| Mode | Converge Crate | Purpose |
|---|---|---|
| Embedded (in-process) | `converge-kernel` | Engine, Context, run Suggestors directly |
| Authoring | `converge-pack` | Suggestor trait, ProposedFact, Invariant |
| Reading results | `converge-model` | Governed semantic types (Fact, Proposal, PromotionRecord) |
| Remote (out-of-process) | `converge-client` | gRPC wire protocol to a deployed Converge instance |

No dependency on `converge-core`, `converge-runtime`, or other internal Converge crates.

## Domain Pack / Converge Pack Split

| Layer | Packs | Nature |
|---|---|---|
| converge-domain | trust, money, delivery, data_metrics | Foundational state machines — any system needs these |
| organism-domain | customers, people, legal, procurement, ... | Organizational workflows — build on top of foundational packs |
| organism-domain | knowledge lifecycle | Moved from converge-domain — organizational learning, not kernel infra |

Blueprints (lead-to-cash, hire-to-retire, etc.) compose organism-domain packs with converge-domain foundational packs.

## Legacy

`_legacy/` contains the pre-restructure monolith. The domain packs and planning types have been revitalized into the current crates. Do not modify `_legacy/` in place.

See also: [[Architecture/Converge Contract]], [[Architecture/Pipeline Flow]]
