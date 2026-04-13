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
  └─ runtime: submit via converge-kernel (embedded) or converge-client (remote)
        │
        ↓
    Converge commit boundary
        │
        ↓
    Execution outcomes
        │
        ↓
    learning.rs: LearningSignal → calibrate priors
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
| Commit | `runtime` | Plan | ProposedFact / SubmitObservationRequest |
| Learning | `learning` | Execution outcome | LearningSignal |

See also: [[Concepts/Intent Pipeline]], [[Architecture/Crate Map]]
