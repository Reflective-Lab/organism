---
tags: [concepts]
---
# Simulation Swarm

Plans that pass adversarial review are stress-tested through simulation across multiple dimensions.

```rust
pub struct SimulationReport {
    pub outcome: Vec<Sample>,
    pub cost: Vec<Sample>,
    pub policy_violations: Vec<String>,
}

pub struct Sample {
    pub value: f64,
    pub probability: f64,
}
```

## Simulation Dimensions

- **Outcome** — does the plan achieve the stated intent?
- **Cost** — what resources does it consume?
- **Policy** — does it violate any declared policies?
- **Causal** — what are the second-order effects?
- **Operational** — can the organization actually execute this?

See also: [[Concepts/Adversarial Review]], [[Concepts/Intent Pipeline]]
