---
tags: [building]
---
# Getting Started

## Prerequisites

- Rust 1.94+ (stable)
- just (task runner)

## Build

```bash
just build      # Build all crates
just test       # Run tests
just lint       # Format + clippy pedantic
```

## Crate Layout

| Crate | What it does |
|---|---|
| `intent` | Intent packets, admission control, decomposition |
| `planning` | Huddle (multi-model planning), debate loop |
| `adversarial` | Assumption breakers, constraint checkers, skeptics |
| `simulation` | Outcome / cost / policy / causal / operational simulation |
| `learning` | Planning priors, calibration, strategy adaptation |
| `runtime` | Agent orchestration, LLM integration, HITL |
| `converge-client` | Thin client over Converge's commit boundary |

## Next Steps

1. Read [[Philosophy/Why Organism]] — understand the mission
2. Read [[Philosophy/Relationship to Converge]] — understand what Organism owns vs what Converge owns
3. Read [[Concepts/Intent Pipeline]] — the full data flow
4. Read [[Building/Starter Task]] — what to build first

See also: [[Architecture/Crate Map]]
