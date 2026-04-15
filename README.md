# Organism

**Organizational intelligence runtime.** Layer 2 in the Reflective Labs stack — sits on top of [Converge](https://github.com/Reflective-Lab/converge), under SaaS product layers.

```
┌─────────────────────────────────────────────┐
│  Helm          Decision frameworks          │
├─────────────────────────────────────────────┤
│  Axiom         Truth validation & codegen   │
├─────────────────────────────────────────────┤
│  Organism      Reasoning, planning, debate  │  ← you are here
├─────────────────────────────────────────────┤
│  Converge      Engine, governance, commit   │
├─────────────────────────────────────────────┤
│  Providers     LLMs, tools, storage         │
└─────────────────────────────────────────────┘
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
| [`pack`](crates/pack) | Pack authoring utilities |

## Examples

| Example | What it shows |
|---|---|
| [`vendor-selection`](examples/vendor-selection) | Multi-criteria vendor evaluation |
| [`expense-approval`](examples/expense-approval) | Policy-gated expense workflow |
| [`loan-application`](examples/loan-application) | Risk-assessed loan decisioning |
| [`resolution-showcase`](examples/resolution-showcase) | Conflict resolution via debate |
| [`debate-loop`](examples/debate-loop) | Adversarial planning loop |

## Develop

```sh
just build
just test
just lint
```

## License

[MIT](LICENSE)
