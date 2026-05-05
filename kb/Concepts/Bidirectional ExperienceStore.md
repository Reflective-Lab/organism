---
tags: [concepts, architecture, load-bearing, cross-repo]
source: mixed
date: 2026-05-06
---
# Bidirectional ExperienceStore

> **THE GAP:** Converge's `ExperienceStore` records what *the engine* did
> (proposals, validations, promotions, blocked states, budget exhaustions). It
> does **not** yet record what *humans* did (approvals, overrides,
> corrections, boundary adjustments). The Engine→User direction works; the
> User→Engine direction is partial.
>
> **THE FIX:** Five `UserExperienceEvent` variants on the user-side ledger,
> driven by Helms operator actions, consumed by Organism's planning priors
> through `recall_from_store`. Two of the five exist in Converge 3.8.0; three
> are pending.

## Why this matters

Today, `PlanningPriorAgent` already calls `recall_from_store` and blends
recall confidence into planning priors (see
`crates/learning/src/prior_agent.rs` and
`crates/learning/tests/recall_feeds_priors.rs`). When the loop is bidirectional
end-to-end, every operator action becomes an event the planning loop can
learn from:

- An approval at the gate boundary biases future planning toward similar
  proposals.
- An override against a constraint biases future planning *away* from the
  shape that triggered the override.
- A correction explicitly tells the loop "this fact was wrong; here's the
  right one".
- A boundary adjustment ("from now on, treat $X over Y as needing approval")
  becomes part of the policy surface that future runs see.

Without these variants, recall is single-source (engine outcomes only) and the
loop can't learn from operator judgment.

## Status (Converge 3.8.0)

| Variant | Status | Source confidence (in `recall.rs`) | Source type |
|---|---|---|---|
| `UserApprovalGranted` | ✓ landed | `0.7` | `SimilarSuccess` |
| `UserOverrideIssued` | ✓ landed | `0.9` | `AntiPattern` |
| `UserApprovalRejected` | pending | proposed: `0.7` | proposed: `AntiPattern` |
| `UserCorrection` | pending | proposed: `0.85` | proposed: `Runbook` |
| `UserBoundaryAdjusted` | pending | proposed: `0.8` | proposed: `Runbook` |

The two landed variants are exercised end-to-end in
`crates/runtime/tests/recall_biases_synthesis.rs`. The three pending variants
need Converge implementation; the spec below is the contract Organism asks
Converge to fulfill.

## Variant specs

All variants live in `converge_core::experience_store::UserExperienceEvent`,
re-exported via `converge_kernel::UserExperienceEvent`. All envelope through
`UserExperienceEventEnvelope` with `event_id`, `occurred_at`, `tenant_id`,
`correlation_id`.

### `UserApprovalGranted` (landed)

```rust
UserApprovalGranted {
    gate_request_id: GateId,
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    reason: Option<String>,
}
```

**Semantics:** A human approved a paused gate request. The approval is
authoritative (it gates promotion) and historically informative (similar
future proposals can be planning-prior-weighted toward similar success).

**Recall mapping:** Confidence `0.7`, source type `SimilarSuccess`. Summary
text: `"user approval: <reason or 'granted'>"`.

**Provenance requirements:**
- `actor` must be a real `ActorId` from the operator surface (no
  `system-default` actors for human actions).
- `policy_snapshot_hash` should pin the Cedar policy version under which the
  approval was decided, so a future replay can detect drift.
- `reason` is optional but strongly encouraged — it carries the operator's
  rationale for the planning loop to weigh.

**Helms surface:** Approve button at HITL pause; payload includes the active
policy snapshot hash automatically.

### `UserOverrideIssued` (landed)

```rust
UserOverrideIssued {
    target: OverrideTarget,
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    reason: String,
}
```

**Semantics:** A human overrode the system's default — typically issued
*against* a constraint or proposed fact. Override is authoritative for the
single decision and historically informative as an anti-pattern: future
planning should weigh away from the shape that triggered the override.

**Recall mapping:** Confidence `0.9` (highest weight), source type
`AntiPattern`. Summary text: `"user override: <reason>"`.

**Provenance requirements:**
- `target` is `OverrideTarget` — variant identifies what was overridden
  (`Constraint(name)`, `Fact(id)`, `Proposal(id)`).
- `actor` and `policy_snapshot_hash` as above.
- `reason` is **required** — overrides without rationale create an
  uninterpretable signal in the planning loop.

**Helms surface:** Override action with required free-text reason.

### `UserApprovalRejected` (pending Converge)

```rust
UserApprovalRejected {
    gate_request_id: GateId,
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    reason: Option<String>,
}
```

**Semantics:** A human declined a paused gate request. The rejection is
authoritative (the gated action does not proceed) and historically informative
as an anti-pattern at the gate boundary.

**Proposed recall mapping:** Confidence `0.7`, source type `AntiPattern`.
Summary text: `"user rejection: <reason or 'declined'>"`. Same weight as
approval but opposite valence — the planning loop should mirror approval's
boost as a damp.

**Provenance requirements:**
- Same as `UserApprovalGranted`. Asymmetry: rejection without reason is more
  acceptable than override without reason, because the rejection itself is
  the signal (the action did not proceed).

**Helms surface:** Reject button at HITL pause.

### `UserCorrection` (pending Converge)

```rust
UserCorrection {
    target: CorrectionTarget,           // Fact(FactId) | Proposal(ProposalId)
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    original_content: ContentHash,
    corrected_content: String,
    reason: String,
}
```

**Semantics:** A human edited a previously promoted fact or proposal. The
correction does not retroactively rewrite the audit trail (the original fact
remains under its `FactId`); a new corrected fact is admitted, and the
correction event ledgers the relationship between the two.

**Proposed recall mapping:** Confidence `0.85`, source type `Runbook`. Summary
text: `"correction (<target_kind>): <reason>"`. The corrected content is
*authoritative for future planning* — recall should bias toward the corrected
shape.

**Provenance requirements:**
- `target` distinguishes Fact vs Proposal correction.
- `original_content` is a hash of the pre-correction content; lets a replay
  verify that the correction was made against the version it was made
  against.
- `corrected_content` is the new content body.
- `reason` is **required** — what was wrong about the original.

**Helms surface:** Correct/redirect surface (the redirect surface is one of
the four primitives in E5; this is the event variant it emits).

**Cross-cutting concern:** The kernel must admit the corrected fact through
the normal admission path before this event is appended. Order: `admit_observation`
→ promote → append `UserCorrection` envelope. The event references the new
corrected fact's id implicitly via `target` if it's a proposal becoming a
fact, or explicitly via the new `FactId` for fact-to-fact corrections.

### `UserBoundaryAdjusted` (pending Converge)

```rust
UserBoundaryAdjusted {
    boundary: BoundaryKind,             // Authority | Forbidden | Expiry | Reversibility
    target: BoundaryTarget,             // Pack(name) | Intent(id) | Global
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    previous_value: serde_json::Value,
    new_value: serde_json::Value,
    reason: String,
}
```

**Semantics:** A human changed one of the four primitive boundaries
(authority limit, forbidden actions, expiry rule, reversibility default) at
some scope. This is policy-shaped, not single-decision-shaped — it
applies forward, not retroactively.

**Proposed recall mapping:** Confidence `0.8`, source type `Runbook`. Summary
text: `"<boundary_kind> boundary adjusted on <target>: <reason>"`. Crucially:
recall for this event should be **scoped** by `target` — a global boundary
adjustment biases all future planning; a pack-scoped adjustment biases only
planning within that pack.

**Provenance requirements:**
- `boundary` identifies which of the four primitives was adjusted.
- `target` scopes the adjustment.
- `previous_value` and `new_value` carry the diff; replay can show what
  changed.
- `reason` is **required** — boundary adjustments without rationale are
  policy drift that future planning shouldn't learn from blindly.

**Helms surface:** Boundary editor (one of the four primitives in E5).
Authority/forbidden/expiry/reversibility editors all emit
`UserBoundaryAdjusted` with the appropriate `boundary` discriminant.

**Special behavior:** Unlike the other variants, this event should **also**
update the active Cedar policy / authority configuration synchronously —
recall after-the-fact is for planning priors, but the boundary has changed
*now*. The event records what happened; the policy update is a separate
write through Converge's policy surface.

## Integration test plan

Located in `crates/runtime/tests/` (because the wiring crosses
PlanningPriorAgent + Engine + ExperienceStore).

**Test 1: `user_override_dampens_subsequent_proposal_confidence`**

- Run formation A: no events in store → record the resulting synthesis
  proposal's confidence.
- Run formation B (same intent, same suggestors): store has a
  `UserOverrideIssued` against a similar constraint → assert the synthesis
  proposal confidence is *lower* than A. This proves anti-pattern recall
  visibly damps planning.

**Test 2: `user_correction_biases_planning_toward_corrected_shape`**

- Run formation A: no events → planning produces shape X.
- Run formation B: store has `UserCorrection` from shape X to shape Y →
  planning produces shape closer to Y, evidenced by hypothesis text
  containing Y's keyword.

**Test 3: `boundary_adjustment_is_scoped_by_target`**

- Two parallel formations, one inside `target=Pack(P1)` scope, one inside
  `target=Pack(P2)`. Store has `UserBoundaryAdjusted` with `target=Pack(P1)`.
- Assert: P1 formation's planning reflects the adjustment; P2's does not.

**Test 4: `recall_summary_count_matches_appended_events`**

- For each variant, append N envelopes and run `PlanningPriorAgent`.
- Assert: `recall-summary` hypothesis carries `count == N` and the per-event
  `summary` strings match the spec table above.

The first integration test (covering the two landed variants) already lives at
`crates/runtime/tests/recall_biases_synthesis.rs`. The remaining tests land
when the corresponding variants ship in Converge.

## Cross-references

- Converge: `converge_core::experience_store::{UserExperienceEvent, UserExperienceEventEnvelope, ExperienceStore}`
- Converge: `converge_core::recall::{recall_from_store, RecallCandidate, RecallPolicy, record_to_candidate}` — the consumer side
- Organism: [`PlanningPriorAgent`](../../crates/learning/src/prior_agent.rs) — first consumer
- Organism: [`recall_feeds_priors.rs`](../../crates/learning/tests/recall_feeds_priors.rs) — upstream half of the loop
- Organism: [`recall_biases_synthesis.rs`](../../crates/runtime/tests/recall_biases_synthesis.rs) — downstream half of the loop
- Helms (E5): the four-primitive operator surface that emits these events
- EPIC: E2 signal "ExperienceStore recall feeds planning"; E5 signal "Bidirectional ExperienceStore"

## Open questions

1. **`UserCorrection` granularity.** Should `original_content` be a hash *or*
   the full original? Hash is space-efficient but requires the reader to
   already have the original; full is verbose but self-contained for replay.
   Recommendation: hash, with the original stored adjacent in the artifact
   ledger.
2. **`UserBoundaryAdjusted` scoping.** Should the event itself enforce the
   scope, or should recall consumers honor the scope? Recommendation: event
   carries scope, consumers honor it. Keeps the ledger simple.
3. **Replay semantics for boundary adjustments.** When replaying a recorded
   run, should the boundary at replay-time match the boundary at
   record-time? Required for deterministic replay. Implies the
   `policy_snapshot_hash` on the event is load-bearing for replay verification.
