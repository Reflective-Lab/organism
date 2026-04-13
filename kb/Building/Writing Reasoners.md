---
tags: [building]
---
# Writing Reasoners

A reasoner produces candidate plans from an intent. Multiple reasoners run in the huddle — failures are dropped, survivors proceed to debate.

## The Trait

```rust
#[async_trait]
pub trait Reasoner: Send + Sync {
    async fn reason(&self, intent: &IntentPacket) -> Result<Plan>;
}
```

## Plan Output

```rust
pub struct Plan {
    pub id: String,
    pub intent_id: String,
    pub steps: Vec<PlanStep>,
    pub rationale: String,
}

pub struct PlanStep {
    pub description: String,
    pub dependencies: Vec<String>,
}
```

## Rules

- A reasoner produces ONE plan per call
- The huddle runs multiple reasoners in parallel
- Failures are silently dropped (the huddle is fault-tolerant)
- Reasoners do not coordinate with each other
- The plan's `rationale` field is for the adversarial review — explain your reasoning

See also: [[Concepts/Huddle and Debate]], [[Concepts/Intent Pipeline]]
