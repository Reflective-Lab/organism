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

## Shape Calibration

The learning system also calibrates which collaboration shapes work for which problem classes. `ShapeCalibration` records the posterior score for a topology within a problem class (e.g., `"irreversible_high_multi_authority"` → Panel scored 0.85 over 10 observations).

`calibrate_shape()` uses the same Bayesian-ish update as `calibrate_priors()`: blend prior with observation, weighted by evidence count. Over episodes, the system converges toward the shapes that produce the best evidence quality, convergence speed, or contradiction minimization for each problem class.

These calibrations feed back into `derive_charter_with_priors()`, closing the loop: intent → derive shape → run → observe → calibrate → derive better next time.

See also: [[Concepts/Intent Pipeline]], [[Concepts/Huddle and Debate]], [[Philosophy/Key Invariants]]
