---
tags: [handoff, cross-repo, blocking-organism]
source: human
date: 2026-05-06
target: converge
unblocks: organism task #12 (bidirectional ExperienceStore variant consumption)
---
# Handoff to Converge — Three new `UserExperienceEvent` variants

This brief describes a self-contained Converge change. When it ships, Organism's Task #12 unblocks and the bidirectional ExperienceStore loop closes end-to-end.

## Why this is needed

**What's already in 3.8.0** (working today, exercised by `crates/runtime/tests/recall_biases_synthesis.rs` in Organism):

```rust
// converge_core::experience_store::UserExperienceEvent
pub enum UserExperienceEvent {
    UserApprovalGranted { gate_request_id, actor, policy_snapshot_hash, reason },
    UserOverrideIssued  { target, actor, policy_snapshot_hash, reason },
}
```

Two variants. They flow through `recall_from_store` → `RecallCandidate` and Organism's `PlanningPriorAgent` blends them into planning priors.

**What the loop needs to be complete:**

| Variant | What it records | Why a separate variant |
|---|---|---|
| `UserApprovalRejected` | A human declined a paused gate. The action does NOT proceed. | Approval and rejection are symmetric authoritatively but opposite as planning signals. Folding into `UserApprovalGranted { reason: Some("declined") }` would lose the typed signal. |
| `UserCorrection` | A human edited a previously promoted fact / proposal. The original stays in the audit trail; the correction is admitted as a new fact and the event ledgers the relationship. | Correction has different fields (`original_content`, `corrected_content`) and different recall semantics (Runbook source, not AntiPattern). |
| `UserBoundaryAdjusted` | A human changed one of the four primitive boundaries (authority / forbidden / expiry / reversibility) at some scope. Forward-applying, not retroactive. | Boundary changes are policy-shaped, not single-decision-shaped. The recall signal must be **scope-aware** (Pack(P1) vs Global) — folding into another variant loses scope.

**What breaks without them:**
- Helms cannot ledger reject/correct/boundary actions through a typed surface; the user-side ledger stays partial.
- Organism's planning priors only learn from approvals + overrides, missing correction/boundary signals that are typically the highest-information events (corrections especially — humans rarely correct without strong reason).
- Replay verification cannot reproduce a run where a boundary was adjusted mid-flight, because there's no event to replay against.

**Canonical spec:** `kb/Concepts/Bidirectional ExperienceStore.md` in the Organism repo. This handoff matches that spec; if any field disagrees, the spec wins.

## Where the work lands in Converge

Three files:

1. `crates/core/src/experience_store.rs` — add the variants to `UserExperienceEvent` + add the supporting enums.
2. `crates/core/src/recall.rs` — add three cases to `record_to_candidate`.
3. `crates/kernel/src/lib.rs` — re-export the new enums via the existing `pub use converge_core::{...}` block.

Plus tests in those files' `#[cfg(test)] mod tests` blocks.

## Variant 1 — `UserApprovalRejected`

### Shape

```rust
UserApprovalRejected {
    gate_request_id: GateId,
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    reason: Option<String>,
}
```

Mirrors `UserApprovalGranted` exactly — same fields, same optionality. The variant **discriminates the decision**; the fields don't need to differ. This makes the pair symmetric for serialization, replay, and pattern-matching.

### Recall mapping

In `record_to_candidate`:

```rust
UserExperienceEvent::UserApprovalRejected { reason, .. } => Some(make_candidate(
    env.event_id.as_str(),
    env.occurred_at.as_str(),
    format!(
        "user rejection: {}",
        reason.as_deref().unwrap_or("declined"),
    ),
    UnitInterval::clamped(0.7),       // same weight as approval
    CandidateSourceType::AntiPattern, // opposite valence from approval's SimilarSuccess
)),
```

Confidence `0.7` matches `UserApprovalGranted` (rejection is as authoritative as approval at the gate). Source type is `AntiPattern` so that planning priors *damp* the rejected shape — mirror image of how approval `SimilarSuccess` boosts.

Note the asymmetry vs. `UserOverrideIssued`'s `0.9` `AntiPattern`: an override is more informative than a rejection (an override is an active "no, do this other thing"; a rejection is just "don't do this"). The 0.7 ↔ 0.9 spread keeps that distinction.

### Test

In `crates/core/src/recall.rs` tests, add a case to whatever existing `record_to_candidate_*` test covers `UserApprovalGranted`:

```rust
#[test]
fn user_approval_rejected_becomes_anti_pattern_candidate() {
    let env = UserExperienceEventEnvelope::new(
        "evt-rej-1",
        UserExperienceEvent::UserApprovalRejected {
            gate_request_id: GateId::new("g1"),
            actor: ActorId::new("op-1"),
            policy_snapshot_hash: None,
            reason: Some("not aligned with quarterly plan".into()),
        },
    );
    let cand = record_to_candidate(&ExperienceRecord::User(env)).expect("rejection → candidate");
    assert_eq!(cand.source_type, CandidateSourceType::AntiPattern);
    assert_eq!(cand.confidence.as_f64(), 0.7);
    assert!(cand.summary.contains("user rejection"));
}
```

## Variant 2 — `UserCorrection`

### Supporting enum (new)

```rust
/// What kind of artifact was corrected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CorrectionTarget {
    /// A previously promoted fact had its content corrected. The original
    /// fact stays in the ledger; a new corrected fact is admitted and this
    /// event ties the two together.
    Fact { fact_id: FactId },
    /// A proposal that hadn't been promoted yet was corrected before
    /// promotion. Different from Fact correction because no audit trail
    /// rewrite concern applies.
    Proposal { proposal_id: ProposalId },
}

impl CorrectionTarget {
    /// Short label for use in recall summaries and logs. Use this rather
    /// than `Debug` so the format stays stable across refactors.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Fact { .. } => "fact",
            Self::Proposal { .. } => "proposal",
        }
    }
}
```

### Variant shape

```rust
UserCorrection {
    target: CorrectionTarget,
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    /// Hash of the pre-correction content. Lets a replay verify that the
    /// correction was made against the version it was made against, even if
    /// the corrected content's full body has been pruned from cold storage.
    original_content: ContentHash,
    /// New content body. Authoritative for future planning — recall consumers
    /// should bias toward this shape.
    corrected_content: String,
    /// Required. Corrections without rationale create an uninterpretable
    /// signal in the planning loop.
    reason: String,
}
```

**Why `original_content` is a hash, not the full body:**
Hash is space-efficient and sufficient for replay verification. The full original body lives in the artifact ledger keyed by `target.{fact_id, proposal_id}`. Storing it inline would either duplicate the ledger or invite divergence. (Open question in the spec; this resolves it.)

**Why `reason` is required (not `Option<String>`):**
Unlike approval/rejection where the *outcome* is the signal (the action did or didn't proceed), a correction's signal is *what to learn from*. Without rationale, the planning loop can match the corrected content shape but can't generalize. Make rationale a hard constraint at the type level rather than relying on convention.

### Recall mapping

```rust
UserExperienceEvent::UserCorrection { reason, target, .. } => Some(make_candidate(
    env.event_id.as_str(),
    env.occurred_at.as_str(),
    format!("correction ({}): {reason}", target.kind_label()),
    UnitInterval::clamped(0.85),
    CandidateSourceType::Runbook,
)),
```

Confidence `0.85` — between approval/rejection (0.7) and override (0.9). Corrections are highly informative (humans don't correct lightly) but they're constructive rather than blocking, so they don't outweigh the override's anti-pattern weight.

Source type `Runbook` because a correction tells the loop *"do it this way next time"* — that's runbook-shaped, not anti-pattern (rejection-shaped) and not similar-success (approval-shaped).

### Cross-cutting concern: admission ordering

The kernel must admit the **corrected fact** through the normal admission path BEFORE this event is appended. Order:

1. `admit_observation(corrected_fact)` → returns `AdmissionReceipt`.
2. Engine promotes through the gate (normal flow).
3. THEN append the `UserCorrection` envelope referencing the new `FactId` (for `CorrectionTarget::Fact`) or the original `ProposalId` (for `CorrectionTarget::Proposal`).

This keeps correction events off any path that could become a forge-vector. The event ledgers a relationship; it doesn't *cause* admission.

Document this ordering in the variant's doc comment and add a note in `kb/Architecture/Plug Boundary.md` if relevant.

### Test

```rust
#[test]
fn user_correction_becomes_runbook_candidate() {
    let env = UserExperienceEventEnvelope::new(
        "evt-corr-1",
        UserExperienceEvent::UserCorrection {
            target: CorrectionTarget::Fact { fact_id: FactId::new("f-42") },
            actor: ActorId::new("op-1"),
            policy_snapshot_hash: None,
            original_content: ContentHash::sha256_of("old body"),
            corrected_content: "new body".into(),
            reason: "amount field was off by one decimal".into(),
        },
    );
    let cand = record_to_candidate(&ExperienceRecord::User(env)).expect("correction → candidate");
    assert_eq!(cand.source_type, CandidateSourceType::Runbook);
    assert_eq!(cand.confidence.as_f64(), 0.85);
    assert!(cand.summary.contains("correction (fact)"));
    assert!(cand.summary.contains("amount field was off"));
}
```

## Variant 3 — `UserBoundaryAdjusted`

### Supporting enums (new)

```rust
/// Which of the four primitive boundaries was adjusted.
///
/// Maps to the four-primitive operator surface in Helms (E5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryKind {
    /// Authority limit (who may do what).
    Authority,
    /// Forbidden actions list.
    Forbidden,
    /// Expiry rules (when intents auto-halt).
    Expiry,
    /// Default reversibility class for new intents.
    Reversibility,
}

/// Scope at which a boundary adjustment applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum BoundaryTarget {
    /// Adjustment applies only within a named pack.
    Pack { pack_id: PackId },
    /// Adjustment applies to a single in-flight intent.
    Intent { intent_id: TypesIntentId },
    /// Adjustment applies workspace-wide.
    Global,
}
```

### Variant shape

```rust
UserBoundaryAdjusted {
    boundary: BoundaryKind,
    target: BoundaryTarget,
    actor: ActorId,
    policy_snapshot_hash: Option<ContentHash>,
    /// The boundary's previous value, JSON-encoded. Recorded so a replay can
    /// reproduce the pre-adjustment state.
    previous_value: serde_json::Value,
    /// The boundary's new value, JSON-encoded.
    new_value: serde_json::Value,
    /// Required. Boundary adjustments without rationale are policy drift the
    /// planning loop should not learn from blindly.
    reason: String,
}
```

**Why JSON values for previous/new:**
The four `BoundaryKind`s carry different value shapes (string actor for Authority, `Vec<String>` for Forbidden, `chrono::DateTime` rules for Expiry, enum for Reversibility). A typed sum type would couple this event to every future boundary kind. JSON values keep the event open while still being self-describing through `boundary` + serialization.

### Recall mapping

```rust
UserExperienceEvent::UserBoundaryAdjusted { boundary, target, reason, .. } => Some(make_candidate(
    env.event_id.as_str(),
    env.occurred_at.as_str(),
    format!("{boundary:?} boundary adjusted on {target:?}: {reason}"),
    UnitInterval::clamped(0.8),
    CandidateSourceType::Runbook,
)),
```

Confidence `0.8` — slightly under correction (0.85). Boundary adjustments are forward-applying policy, not corrections of past mistakes; the loop should learn from them but they're less direct evidence about any single decision.

Source type `Runbook` — a boundary change is a future-runbook update.

### Critical: scope-aware consumption

The variant carries scope (`target`), but **consumers must honor it**. `recall_from_store` returns the candidate to all callers; it's `PlanningPriorAgent` (or whichever Organism consumer) that must filter by `target` against the current intent's scope.

Document this on the variant: *"Recall consumers must filter `UserBoundaryAdjusted` candidates by `target` to avoid spilling pack-scoped or intent-scoped adjustments into unrelated planning runs."*

This is **not** something Converge enforces — it's Organism's responsibility. But the doc on the variant should make the contract explicit so future consumers don't accidentally apply a Pack(P1) boundary adjustment to a Pack(P2) run.

### Side-effect note: Cedar policy update

Unlike the other variants (which are pure ledger writes), a `UserBoundaryAdjusted` event represents a change that **also** needs to update active policy synchronously. The flow:

1. Helms operator edits the boundary in the four-primitive UI.
2. Helms calls Converge's policy surface to write the new active policy.
3. **Then** Helms appends the `UserBoundaryAdjusted` envelope to the experience store.

The event is a record of what happened, **not** the mechanism that caused it. Keep this clear in the variant's doc comment so no one tries to use ledger writes as a policy-update channel.

### Test

```rust
#[test]
fn user_boundary_adjusted_becomes_runbook_candidate() {
    let env = UserExperienceEventEnvelope::new(
        "evt-bnd-1",
        UserExperienceEvent::UserBoundaryAdjusted {
            boundary: BoundaryKind::Forbidden,
            target: BoundaryTarget::Pack { pack_id: PackId::new("customers") },
            actor: ActorId::new("op-1"),
            policy_snapshot_hash: None,
            previous_value: serde_json::json!(["delete_account"]),
            new_value: serde_json::json!(["delete_account", "merge_accounts"]),
            reason: "merge events should follow the same gate as deletes".into(),
        },
    );
    let cand = record_to_candidate(&ExperienceRecord::User(env)).expect("boundary → candidate");
    assert_eq!(cand.source_type, CandidateSourceType::Runbook);
    assert_eq!(cand.confidence.as_f64(), 0.8);
    assert!(cand.summary.contains("Forbidden"));
    assert!(cand.summary.contains("Pack"));
    assert!(cand.summary.contains("merge events"));
}
```

## Re-exports in `crates/kernel/src/lib.rs`

Add to the existing `pub use converge_core::{...}` block (around line 67):

```rust
pub use converge_core::{
    // ... existing items ...
    UserExperienceEvent, UserExperienceEventEnvelope,
    // new in 3.8.x:
    BoundaryKind, BoundaryTarget, CorrectionTarget,
};
```

## Acceptance criteria

A 3.8.x release of Converge that includes:

- [ ] Three new `UserExperienceEvent` variants compile and serialize round-trip with `serde_json`.
- [ ] `CorrectionTarget`, `BoundaryKind`, `BoundaryTarget` enums exist in `converge_core` with `Serialize` + `Deserialize` + `Debug` + `Clone` + `PartialEq`.
- [ ] `record_to_candidate` returns a `RecallCandidate` for each of the three new variants per the mappings above.
- [ ] Re-exports flow through `converge_kernel::{...}`.
- [ ] One unit test per new variant in `crates/core/src/recall.rs` confirming the candidate shape (templates above).
- [ ] No `kernel-authority` feature anywhere in the new code (consistent with the 3.8 declaration).

When that's published (or available via path-dep from `../converge`), Organism will:

1. Drop in the new variants in `crates/runtime/tests/recall_biases_synthesis.rs` — same pattern as the existing `UserApprovalGranted` / `UserOverrideIssued` cases.
2. Add a sibling test that asserts each variant maps to the right `source_type` end-to-end (envelope → recall_from_store → recall-summary hypothesis).
3. Update `kb/Concepts/Bidirectional ExperienceStore.md` to mark the three variants as ✓ landed in the status table.

That closes Organism Task #12 and the bidirectional ExperienceStore loop.

## Open question (not blocking)

`UserCorrection` for `CorrectionTarget::Proposal` — does the proposal get re-staged through `admit_observation` after correction, or is the corrected proposal a brand-new admission with a new ID? Recommend: brand-new admission. The original proposal stays in the ledger with its original `ProposalId`; the correction event ties the new ID to the old one. Keeps admission idempotency clean.

If you disagree, the alternative is to allow `admit_observation` to accept an optional `replaces: ProposalId` field on the request — that's a bigger admission API change and probably wants its own ADR.

## Provenance

- **Spec:** `~/dev/work/organism/kb/Concepts/Bidirectional ExperienceStore.md`
- **Existing pattern:** `~/dev/work/converge/crates/core/src/recall.rs` lines 617-655 (`record_to_candidate`)
- **First Organism consumer:** `~/dev/work/organism/crates/runtime/tests/recall_biases_synthesis.rs` (already exercises the two landed variants)
- **Originating session:** Organism's Phase B push for the 1.5.0 sync release (2026-05-05/06)
