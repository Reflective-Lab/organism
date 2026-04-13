---
tags: [concepts]
---
# Adversarial Review

Every plan is stress-tested by adversarial agents before it can proceed to simulation. This is not optional — see [[Philosophy/Key Invariants#2. No Plan Commits Without Passing Both Gates]].

## Finding Types

```rust
pub struct Finding {
    pub agent: String,
    pub severity: Severity,
    pub message: String,
}

pub enum Severity {
    Info,
    Warning,
    Blocker,
}
```

A `Blocker` finding stops the plan. A `Warning` is logged but does not block. `Info` is purely informational.

## Adversarial Agent Types

- **Assumption breakers** — challenge unstated assumptions in the plan
- **Constraint checkers** — verify the plan respects declared constraints
- **Skeptics** — look for overconfidence, missing edge cases, hidden costs

See also: [[Concepts/Intent Pipeline]], [[Concepts/Simulation Swarm]]
