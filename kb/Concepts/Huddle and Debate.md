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
- `CollaborationCharter::self_organizing()` is the loose self-organizing mode for "figure it out" collaboration.

Those charters work with:

- `TeamFormation` — how the team is formed
- `CollaborationRole` — lead, domain, critic, synthesizer, judge, moderator, report writer, and others
- `ConsensusRule` — majority, supermajority, unanimous, lead-decides, advisory-only
- `TurnCadence` — round-robin, lead-then-round-robin, moderator-then-round-robin, synthesis-only, or figure-it-out

For runtime binding, `organism-runtime::CollaborationRunner` maps a charter and
team definition onto product-specific participant metadata.

## Dynamic Collaboration

Charters can be derived, adapted, and discovered:

### Charter Derivation

`derive_charter(intent, now)` reads 6 complexity signals from an `IntentPacket` — reversibility weight, authority breadth, constraint pressure, forbidden density, time pressure, escalation requirement — and produces a charter with a transparent `DerivationRationale`. `derive_charter_with_priors()` integrates historical `ShapeCalibration` to bias toward shapes that have worked for the problem class.

### Topology Transitions

Mid-run shape changes driven by convergence signals. Five trigger types:

| Trigger | Fires when |
|---|---|
| `EvidenceClustering` | Stable fact ratio exceeds threshold for N cycles |
| `ContradictionSpike` | Contradiction ratio exceeds threshold |
| `StabilityReached` | N stable cycles with minimum hypotheses |
| `BudgetPressure` | Remaining budget fraction drops below threshold |
| `ConsensusDeadlock` | Failed vote count exceeds threshold |

Default rules: Swarm→Huddle, Huddle→Panel, Panel→Synthesis, Any→Tighter on budget pressure.

### Shape-as-Hypothesis

The collaboration shape itself competes as a hypothesis. `generate_candidates()` produces 2–3 candidate shapes. Each is evaluated by `score_observation()` against one of four metrics (EvidenceQuality, ConvergenceSpeed, ContradictionMinimization, Balanced). `calibrate_shape()` converges priors over episodes. Future derivations are informed by past outcomes.

## Non-Obvious Formations That Matter

The point is not to collect cute topologies. The point is to use the wrong
shape less often.

- **Start loose, tighten later.**
  `CollaborationCharter::self_organizing()` is useful when the work is still
  exploratory and a prematurely curated team would narrow the search space too
  early. See `examples/collab-self-organizing`.
- **Change shape mid-run.**
  A team that was appropriate for exploration is often the wrong team once
  contradictions spike or budget pressure rises. See `examples/topology-transition`.
- **Let shape compete.**
  When the best topology is genuinely unknown, treat it as a hypothesis and let
  the learning layer calibrate it over episodes. See `examples/shape-competition`.
- **Let actual formations compete.**
  Once shapes become concrete teams of suggestors, run them through
  `organism-runtime::FormationTournament` and score the governed results instead
  of arguing from taste. See `examples/formation-tournament`.
- **Use adaptive loops for diligence.**
  Some flows are neither a fixed huddle nor a fixed pipeline. Due diligence is
  the canonical case: kickoff, research, extraction, gap reopening,
  contradiction detection, then synthesis at stability. See
  `examples/due-diligence-loop`.

## Debate

Plans that survive the huddle enter debate. The debate loop filters and ranks candidates, producing a `PlanBundle` of the strongest survivors.

```rust
pub struct PlanBundle {
    pub candidates: Vec<Plan>,
}
```

The bundle then goes to [[Concepts/Adversarial Review]] before simulation.

See also: [[Concepts/Intent Pipeline]], [[Concepts/Adversarial Review]]
