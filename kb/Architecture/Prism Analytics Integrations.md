---
tags: [architecture, integrations, prism, fuzzy, ml, anomaly]
source: mixed
date: 2026-05-09
---

# Prism Analytics Integrations

Bridges between organism's planning / admission / skeptic contracts and the
inference algorithms in `prism::packs`. Each integration is a concrete
implementation of an organism trait that wraps a prism solver — apps wire
them into Formations and huddles directly.

## Overview

| Integration | Crate | Trait | Wraps | Discovery |
|---|---|---|---|---|
| `FuzzyReasoner` | `organism-planning` | `Reasoner` | `prism::fuzzy::FuzzyInferenceEngine` (Mamdani) | `ReasoningSystem::FuzzyReasoning` (new variant) |
| `MlPredictionReasoner` | `organism-planning` | `Reasoner` | `prism::packs::regression::LinearRegressionSolver` and `prism::packs::classification::LogisticClassifier` (selectable via `MlPredictionMode`) | `ReasoningSystem::MlPrediction` (existing slot) |
| `AnomalySkepticAgent` | `organism-adversarial` | `converge_pack::Suggestor` | `prism::packs::anomaly_detection::ZScoreSolver` | `SkepticismKind::StatisticalAnomaly` (new variant) + `ANOMALY_SKEPTIC_META` descriptor |
| `GradedAdmissionController` | `organism-intent` | `AdmissionController` | `prism::fuzzy::FuzzyInferenceEngine` (Mamdani, one rulebook per `FeasibilityDimension`) | Direct `impl AdmissionController` |

## Wiring into Formations

### Reasoners (FuzzyReasoner, MlPredictionReasoner)

Reasoners participate in huddles, not in the converge `Suggestor`-based
formation registry. Apps construct an instance and pass it to the huddle
setup directly:

```rust
use organism_planning::{FuzzyReasoner, MlPredictionReasoner};

let fuzzy = FuzzyReasoner::new("expectation-fuzz", variables, rules);
let ml = MlPredictionReasoner::regression("cost-predictor", weights, bias);
huddle.add_reasoner(Box::new(fuzzy));
huddle.add_reasoner(Box::new(ml));
```

`FuzzyReasoner` extracts inputs from `intent.context` by linguistic-variable
name; `MlPredictionReasoner` reads the feature vector from
`intent.context["features"]` (configurable via `with_feature_field`).

### Adversarial agents (AnomalySkepticAgent)

`AnomalySkepticAgent` is a `Suggestor` and *can* be discovered via the
runtime `Registry`. The crate exports an `ANOMALY_SKEPTIC_META:
AnomalySkepticDescriptor` constant whose fields mirror
`organism_pack::AgentMeta`. App-level catalogs convert and register:

```rust
use organism_pack::AgentMeta;
use organism_adversarial::{AnomalySkepticAgent, ANOMALY_SKEPTIC_META};

let meta = AgentMeta {
    name: ANOMALY_SKEPTIC_META.name,
    dependencies: ANOMALY_SKEPTIC_META.dependencies,
    fact_prefix: ANOMALY_SKEPTIC_META.fact_prefix,
    target_key: organism_pack::ContextKey::Constraints,
    description: ANOMALY_SKEPTIC_META.description,
};
registry.register_pack("anomaly-skeptic", &[meta], &[]);
engine.register_suggestor(AnomalySkepticAgent::default_config());
```

The two-step indirection (descriptor → `AgentMeta`) avoids a cyclic
dependency: `organism-pack` already pulls in `organism-adversarial`, so
the latter cannot directly depend on `organism-pack`.

### Admission controllers (GradedAdmissionController)

Constructed with one `DimensionRulebook` per `FeasibilityDimension` and used
directly via the `AdmissionController` trait:

```rust
use organism_intent::{GradedAdmissionController, DimensionRulebook};

let controller = GradedAdmissionController::new(vec![
    DimensionRulebook::new(
        FeasibilityDimension::Resources,
        "resources",
        variables,
        rules,
    ),
    // ... one per dimension
]);
let result = controller.evaluate(&intent);
```

Output linguistic-variable set names should match `FeasibilityKind` variants
(snake_case): `feasible`, `feasible_with_constraints`, `uncertain`,
`infeasible`. The kind with the highest membership wins. Activated-rule
trace is preserved on the `FeasibilityAssessment::reason` string so a
reviewer can see which rules drove the decision.

Pairs naturally with `DefaultAdmissionController` — hard-edged checks
first (missing-capability, expired, irreversible-without-authority), graded
checks for the uncertain middle.

## When to use which

- **`FuzzyReasoner`** — graded reasoning over linguistic variables, rulebook
  authored by a domain expert, output preserves activated-rule traces. Best
  fit when stakeholders need to defend the reasoning, not just the answer.
- **`MlPredictionReasoner`** — pre-trained model produces a numeric
  prediction or classification probability; weights ship with the
  deployment, features come from intent context. Best fit when a model has
  been trained on historical data and the prediction is the deliverable.
- **`AnomalySkepticAgent`** — flag plans whose key metric is an outlier
  *relative to the active strategy set* (Z-score over plans). Abstains on
  N<3 because anomaly detection is meaningless for a sample of one.
- **`GradedAdmissionController`** — admission control where feasibility is
  a matter of degree. Stack on top of `DefaultAdmissionController` for the
  cases that don't decide cleanly with hard rules.

## Boundary

These integrations live where the consumer-facing trait lives, not in
prism. `prism::fuzzy` and `prism::packs::*` stay domain-neutral — they
provide math + invariants. The organism crates own the contract bridging
(Reasoner / Suggestor / AdmissionController shapes). Apps own the rulebook
content, weights, and feature names.

When a future integration needs a non-goal capability (Type-2, ANFIS,
Sugeno extensions), the promotion pull originates here — describe the
concrete need before extending prism.
