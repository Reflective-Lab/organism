---
tags: [concepts]
---
# Intent Pipeline

Every intent flows through this sequence. No stage can be skipped.

```
IntentPacket
  ↓ admission control                     [intent crate]
  ↓ decomposition into intent tree        [intent crate]
  ↓ huddle (multi-model planning)         [planning crate]
  ↓ adversarial review                    [adversarial crate]
  ↓ simulation swarm                      [simulation crate]
  ↓ commit submission                     [converge-client crate]
       ↓
   Converge commit boundary
```

## Intent Packet

```rust
pub struct IntentPacket {
    pub id: String,
    pub outcome: String,
    pub context: HashMap<String, String>,
    pub constraints: Vec<String>,
    pub authority: String,
    pub forbidden: Vec<String>,
    pub reversibility: Reversibility,
    pub expires: Option<DateTime>,
}
```

## Admission Control

Cheap feasibility gate. Checks expiry, empty outcome, basic sanity. Rejects obviously invalid intents before they consume planning resources.

## Decomposition

Breaks a complex intent into an `IntentNode` tree. Each node can be planned independently. Depth-first walk.

## Huddle → Adversarial → Simulation

See [[Concepts/Huddle and Debate]], [[Concepts/Adversarial Review]], [[Concepts/Simulation Swarm]].

## Commit Submission

The surviving plan becomes a `SubmitObservationRequest` to Converge. Converge decides whether to promote it. Organism has no authority here — see [[Philosophy/Key Invariants#1. Authority is Recomputed at the Commit Boundary]].

See also: [[Architecture/Pipeline Flow]]
