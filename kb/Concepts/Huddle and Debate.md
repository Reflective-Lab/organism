---
tags: [concepts]
---
# Huddle and Debate

## Huddle

Multi-model collaborative planning. Multiple reasoners are given the same intent and produce candidate plans in parallel. Failures are dropped. Survivors proceed to debate.

```rust
pub struct Huddle {
    reasoners: Vec<Box<dyn Reasoner>>,
}
```

The huddle orchestrator runs all reasoners, collects their plans, and passes survivors to the debate loop.

## Debate

Plans that survive the huddle enter debate. The debate loop filters and ranks candidates, producing a `PlanBundle` of the strongest survivors.

```rust
pub struct PlanBundle {
    pub candidates: Vec<Plan>,
}
```

The bundle then goes to [[Concepts/Adversarial Review]] before simulation.

See also: [[Concepts/Intent Pipeline]], [[Concepts/Adversarial Review]]
