---
tags: [architecture]
---
# Pipeline Flow

Detailed data flow through the organism pipeline.

```
IntentPacket
  │
  ├─ admission.rs: admit() → Admission::Feasible | Rejected
  │
  ├─ decomposition.rs: decompose() → IntentNode tree
  │
  ├─ huddle.rs: Huddle::run() → Vec<Plan>
  │     (parallel reasoning, failures dropped)
  │
  ├─ debate.rs: debate() → PlanBundle
  │     (filter, rank survivors)
  │
  ├─ adversarial: review() → Vec<Finding>
  │     (any Blocker → plan rejected)
  │
  ├─ simulation: simulate() → SimulationReport
  │     (outcome, cost, policy, causal, operational)
  │
  └─ runtime: assemble `Formation` (team + seeds + budget)
        │
        ↓
    embedded: `Formation::run()` → `Engine.run()`
    remote: equivalent deployed Converge run
        │
        ↓
    ConvergeResult (promoted facts, stop reason, integrity proof)
        │
        ↓
    learning.rs: LearningSignal → calibrate priors
        │
        ↓
    shape calibration → derive_charter_with_priors (loop closes)
```

## Dynamic Collaboration Layer

Charter derivation sits between intent admission and the huddle:

```
IntentPacket
  │
  ├─ derive_charter(intent, now) → DerivedCharter
  │     (6 complexity signals → charter + rationale)
  │
  ├─ [during run] evaluate_transitions(signals) → TopologyTransition
  │     (Swarm→Huddle, Huddle→Panel, Panel→Synthesis, budget tighten)
  │
  └─ [post-run] calibrate_shape(observation) → ShapeCalibration
        (priors fed back into derive_charter_with_priors)
```

## Where Each Crate Owns

| Stage | Crate | Input | Output |
|---|---|---|---|
| Admission | `intent` | IntentPacket | Admission |
| Decomposition | `intent` | IntentPacket | IntentNode tree |
| Planning | `planning` | IntentNode | Vec<Plan> |
| Debate | `planning` | Vec<Plan> | PlanBundle |
| Adversarial | `adversarial` | PlanBundle | Vec<Finding> |
| Simulation | `simulation` | Plan | SimulationReport |
| Formation run | `runtime` | Selected team + seeds + budget | `FormationResult` / `ConvergeResult` |
| Learning | `learning` | Execution outcome | LearningSignal |

## Boundary Rule

Planning, adversarial review, simulation, optimization, policy, analytics, and
knowledge may all participate inside the Converge run, but once they do, they
enter through `Suggestor`. There is no side-car in-loop pipeline contract.

See also: [[Concepts/Intent Pipeline]], [[Architecture/Crate Map]]
