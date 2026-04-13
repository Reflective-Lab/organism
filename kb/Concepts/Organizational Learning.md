---
tags: [concepts]
---
# Organizational Learning

Organism learns from execution outcomes to improve future planning.

```rust
pub struct LearningSignal {
    pub kind: SignalKind,
    pub weight: f64,
    pub note: String,
}

pub enum SignalKind {
    OutcomeMatchedExpectation,
    OutcomeBeatExpectation,
    OutcomeMissedExpectation,
    AdversarialBlocker,
    AdversarialWarning,
}
```

Learning signals flow backward — from execution outcomes into planning priors. They never flow directly into authority ([[Philosophy/Key Invariants#3. Learning Flows Backward, Authority Flows Forward]]).

See also: [[Concepts/Intent Pipeline]], [[Philosophy/Key Invariants]]
