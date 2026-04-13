# Organism Milestones

> This file is the single source of truth for what ships and when.
> Every session starts by reading this file and scoping work to the **current milestone**.
>
> Rules:
> - No feature work outside the current milestone without explicit approval.
> - Side-fixes get 15 min max, then back to the milestone.
> - Each session ends with a checkpoint: what moved, what's left, any date risk.
> - When a deliverable is done, mark it `[x]` and note the date.
>
> See `~/dev/work/EPIC.md` for the coarse-grained outcomes these milestones advance.

---

## Current: Stage 1 — "The pipeline compiles"
**Epic:** E2 (Organism reasons before it acts)

**Deadline: 2026-05-15**

End-to-end skeleton: intent in, observation out to Converge. Every crate compiles with real types, wired together through the pipeline.

### Converge integration

- [ ] **Wire converge-pack + converge-kernel** — Add real Converge dependencies to runtime (starter task)
- [ ] **Embedded CommitBoundary** — In-process Converge via converge-kernel
- [ ] **Axiom enforcement test** — Prove ProposedFact compiles, Fact construction does not (trybuild)

### Intent layer

- [x] **Intent packet types** — Typed contracts with ForbiddenAction, ExpiryAction, reversibility (2026-04-12)
- [x] **Admission control types** — AdmissionController trait, 4 feasibility dimensions (2026-04-12)
- [x] **Intent decomposition** — IntentNode tree with authority narrowing (2026-04-12)
- [ ] **Admission controller impl** — At least one real admission controller

### Planning layer

- [x] **Plan annotations** — Impact, cost, risk modeling on every plan (2026-04-12)
- [x] **Huddle scaffold** — HuddleParticipant/Reasoner traits, 6 reasoning systems (2026-04-12)
- [x] **Adversarial types** — Challenge, 5 SkepticismKinds, AdversarialSignal, Skeptic trait (2026-04-12)
- [ ] **Debate loop impl** — Working propose → attack → revise cycle

### Simulation layer

- [x] **Simulation swarm types** — 5 dimensions, SimulationRunner trait, recommendations (2026-04-12)
- [ ] **Outcome simulation impl** — At least one working simulator

### Learning layer

- [x] **Learning episode types** — Full episode, prediction error, lesson, prior calibration (2026-04-12)
- [ ] **Calibration impl** — Working prior update from execution outcomes

### Domain packs

- [x] **Knowledge lifecycle** — Signal → Hypothesis → Experiment → Decision → Canonical (2026-04-12)
- [x] **13 organizational packs** — customers, people, legal, performance, autonomous_org, growth_marketing, product_engineering, ops_support, procurement, partnerships, virtual_teams, linkedin_research, reskilling (2026-04-12)
- [x] **8 blueprints** — lead-to-cash, hire-to-retire, procure-to-pay, etc. (2026-04-12)

### Runtime

- [ ] **Pipeline wiring** — Intent → planning → adversarial → simulation → Converge end-to-end
- [ ] **LLM integration** — At least one reasoning provider wired to the huddle

---

## Stage 2 — "Plans get stress-tested"

**Deadline: 2026-06-30**

Adversarial review and simulation swarm produce real signal. Plans that reach Converge have been argued over.

### Adversarial layer

- [ ] **Assumption breakers** — Challenge hidden assumptions in candidate plans
- [ ] **Constraint checkers** — Verify plans against organizational constraints
- [ ] **Economic skeptics** — Challenge cost and resource assumptions
- [ ] **Operational skeptics** — Challenge feasibility given team/system state

### Simulation layer

- [ ] **Cost simulation** — Compute, spend, resource envelope
- [ ] **Policy simulation** — Constraint and authority compliance
- [ ] **Causal simulation** — Correlation vs causation, confounders
- [ ] **Operational simulation** — Systems and team feasibility

### Learning

- [ ] **Planning priors** — Initial calibration framework
- [ ] **Adversarial signal capture** — Firings become labeled training signals

---

## Stage 3 — "The runtime learns"

**Deadline: 2026-08-31**

Organizational learning closes the loop. Planning improves from execution outcomes.

### Learning layer

- [ ] **Outcome tracking** — Compare predicted vs actual outcomes
- [ ] **Prior calibration** — Adjust planning priors from execution feedback
- [ ] **Strategy adaptation** — Environmental feedback shapes future planning

### Runtime maturity

- [ ] **HITL integration** — Human-in-the-loop at key decision points
- [ ] **Multi-organization isolation** — Runtime serves multiple orgs without cross-contamination
- [ ] **Observability** — Planning traces, simulation results, adversarial verdicts queryable
