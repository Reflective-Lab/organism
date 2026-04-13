# organism.zone

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
| [`runtime`](crates/runtime) | Agent orchestration, LLM integration, HITL |
| [`converge-client`](crates/converge-client) | Client for Converge's commit boundary (via `converge-client` + `converge-pack` v3.0.0) |

## Develop

```sh
just build
just test
just lint
```

## Strategy

Canonical strategy: `~/dev/brand/organism-business/strategy/STRATEGY.md`.
