# organism-core

Runtime primitives for the Organism organizational intelligence layer.

## What is Organism?

Organism sits above **Converge** (the deterministic execution kernel) and below **SaaS products**. It answers:

> How does an autonomous organization think, plan, and evolve?

Where Converge answers "what actions are allowed?", Organism answers how organizations reason about intent, plan strategies, challenge assumptions, and learn from execution.

## Architecture

```
SaaS Products (specific organism configurations)
    ↓ runs on
Organism.zone (this crate — organizational intelligence runtime)
    ↓ runs on
Converge.zone (deterministic execution kernel)
```

## Core Primitives

| Module | Purpose |
|--------|---------|
| `intent` | Intent packets, admission control, decomposition |
| `planning` | Planning huddle, multi-model reasoning |
| `adversarial` | Adversarial agents for plan validation |
| `simulation` | Simulation swarm for stress-testing proposals |
| `authority` | Non-monotonic authority and commit barrier |
| `learning` | Organizational learning and strategy evolution |

## Key Properties

- **Intent-driven**: Work enters as intent packets with outcome, context, constraints, authority, forbidden actions, reversibility, and expiry
- **Adversarial planning**: Institutionalized disagreement before commitment
- **Non-monotonic authority**: Authority recomputed at commit time, never inherited
- **Organizational learning**: Adversarial firings become labeled training signals

## Philosophy

> The autonomous firm is not a simulation of the human firm.
> Hierarchy was a workaround for slow coordination. We can do better now.
