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

## Collaboration Charters

Organism separates the **planning payload** from the **collaboration contract**.

- `CollaborationCharter::huddle()` is the strict default: explicit turns, synthesis, dissent map, and done gate.
- `CollaborationCharter::discussion_group()` is moderated and lighter-weight.
- `CollaborationCharter::panel()` is a curated expert panel with explicit roles and decision policy.
- `CollaborationCharter::open_claw()` is the loose self-organizing mode for "figure it out" collaboration.

Those charters work with:

- `TeamFormation` — how the team is formed
- `CollaborationRole` — lead, domain, critic, synthesizer, judge, moderator, report writer, and others
- `ConsensusRule` — majority, supermajority, unanimous, lead-decides, advisory-only
- `TurnCadence` — round-robin, lead-then-round-robin, moderator-then-round-robin, synthesis-only, or figure-it-out

## Debate

Plans that survive the huddle enter debate. The debate loop filters and ranks candidates, producing a `PlanBundle` of the strongest survivors.

```rust
pub struct PlanBundle {
    pub candidates: Vec<Plan>,
}
```

The bundle then goes to [[Concepts/Adversarial Review]] before simulation.

See also: [[Concepts/Intent Pipeline]], [[Concepts/Adversarial Review]]
