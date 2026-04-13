# Organism

**Organizational intelligence runtime.** Layer 2 in the Reflective Labs stack — sits on top of [Converge](https://github.com/Reflective-Lab/converge), under SaaS product layers.

```
┌─────────────────────────────────────────┐
│  SaaS Products                          │
│  (specific organism configurations)     │
└──────────────────┬──────────────────────┘
                   │ runs on
┌──────────────────▼──────────────────────┐
│  Organism                               │
│  intent · planning · adaptation         │
└──────────────────┬──────────────────────┘
                   │ calls into
┌──────────────────▼──────────────────────┐
│  Converge                               │
│  axioms · authority · commit            │
└─────────────────────────────────────────┘
```

Where Converge answers *"what actions are allowed to happen?"*, Organism answers *"how does an autonomous organization think, plan, and evolve?"*

## Crates

| Crate | Role |
|---|---|
| [`intent`](crates/intent) | Intent packets, admission control, decomposition |
| [`planning`](crates/planning) | Huddle, debate loop |
| [`adversarial`](crates/adversarial) | Assumption breakers, skeptics, constraint checkers |
| [`simulation`](crates/simulation) | Outcome / cost / policy / causal / operational simulation |
| [`learning`](crates/learning) | Planning priors, calibration |
| [`runtime`](crates/runtime) | Curated embedded runtime: registry, readiness, pipeline wiring |
| [`intelligence`](crates/intelligence) | Provider-shaped capabilities: OCR, vision, web, social, billing |
| [`notes`](crates/notes) | Vault lifecycle: ingestion, cleanup, enrichment |
| [`domain`](crates/domain) | Organizational pack library and blueprints |

## Develop

```sh
just build
just test
just lint
```

## Strategy

Canonical strategy: `~/dev/brand-kb/organism-business/strategy/STRATEGY.md`.
