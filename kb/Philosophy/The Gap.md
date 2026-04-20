---
tags: [philosophy, architecture]
source: mixed
---
# The Gap

The big problem Organism solves: filling the gap between a human's intent
and a converging solution in Converge.

## The Model

```
Human intent (what you want)
    ↓
Organism (matchmaker — who should work on this, how, in what formation)
    ↓
Converge Engine runs (mathematical — honest, replayable, converging)
    ↓
World changes
```

## What Converge Provides

Converge is the **mathematics**. A single Engine run is:

- Honest: ProposedFact ≠ Fact, promotion goes through the gate
- Replayable: ExperienceStore captures every event
- Converging: fixed-point loop terminates when context stabilizes
- Auditable: every Fact has a PromotionRecord

Converge does not decide who runs. It runs whoever you register and finds
the fixed point.

## What Organism Provides

Organism is the **matchmaker and formation guru**. Given an intent, it:

1. Decides which agents should work on the problem
2. Forms them into teams (Formations)
3. Runs multiple competing formations concurrently
4. Evaluates results, forms new teams, restarts
5. Learns which formations work for which kinds of problems

## Key Principles

### Heterogeneous agents

Not all agents are LLMs. A problem might be best solved by:

- An LLM reasoning about strategy
- An optimization solver (OR-Tools) finding the schedule
- A causal model estimating outcomes
- A policy engine checking compliance
- A scheduling algorithm allocating resources

All implement `Suggestor` — Converge's trait. That's the common interface.
The agent's internal implementation is invisible to Converge.

### Dynamic formation

You don't know beforehand if the solution needs 3 or 5 layers. Formations
are hypotheses that Organism tests:

- Start with a formation
- If it doesn't converge well, form a different team
- Agents that learn become creative — they can request another loop
- Competing runs race to a solution
- Winners share results, losers learn why

### Async, seed-driven

Agents work at independent speeds. When one agent produces a seed (a
ProposedFact that gets promoted), agents downstream that depend on that
key become eligible. Converge's dependency mechanism handles this naturally.

### The intent codec

The system IS the intent codec. A higher-level truth statement goes in.
Organism decodes it into formations. Formations run in Converge. What
comes out is a governed, auditable convergence on the answer.

## What CommitBoundary Was Getting Wrong

The old `CommitBoundary` trait modeled Converge as a submission endpoint:
"here's my observation, please commit it." This is backwards.

Converge is not a database you push to. It's a **convergence engine you
form teams for**. Organism's job is not to submit results — it's to
assemble the right agents and let Converge find the fixed point.

The correct flow:
1. Organism forms a `Formation` (team of Suggestors + seed Context)
2. Formation runs as a Converge `Engine.run()`
3. Converge returns `ConvergeResult`
4. Organism evaluates, possibly forms new teams
5. ExperienceStore captures everything

## The Hard Problem

The gap between intent and convergence is where all the intelligence lives.
Filling this gap means:

- Decomposing intent into tractable sub-problems
- Knowing which agent types are good at which sub-problems
- Forming teams that complement each other
- Detecting when a formation is stuck and needs restructuring
- Learning from past runs to form better teams next time

This is what Organism's planning, adversarial, simulation, and learning
layers are for. They don't produce the answer — they produce the
**formation that produces the answer**.
