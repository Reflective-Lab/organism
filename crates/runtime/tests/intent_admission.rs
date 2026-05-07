//! End-to-end: IntentPacket → Converge admission.
//!
//! Organism's runtime takes pre-compiled `IntentPacket`s and stages them
//! through `converge_kernel::admission::admit_observation`. Source-specific
//! compilation (e.g. parsing `.truths` documents and producing an
//! IntentPacket) lives outside organism — for Truth-shaped sources, in
//! `axiom_truth::compile_intent`. The tests below build IntentPackets
//! directly to verify each admission transition observably.

use chrono::{Duration, Utc};
use converge_kernel::ContextState;
use converge_kernel::admission::{AdmissionActor, AdmissionActorKind, AdmissionSource};
use organism_pack::{ForbiddenAction, IntentPacket};
use organism_runtime::{IntentAdmissionError, Runtime};

fn lead_qualification_intent() -> IntentPacket {
    let mut intent = IntentPacket::new(
        "qualify inbound leads end-to-end",
        Utc::now() + Duration::hours(24),
    )
    .with_authority(vec![
        "actor: revops_team".into(),
        "approve_qualified_lead".into(),
    ]);
    intent.forbidden = vec![ForbiddenAction {
        action: "approve_unverified_lead".into(),
        reason: "authority".into(),
    }];
    intent.constraints = vec!["budget: 500_USD/week".into()];
    intent
}

fn helms_actor() -> AdmissionActor {
    AdmissionActor::new("helms-truth-catalog", AdmissionActorKind::External)
        .expect("valid admission actor")
}

fn helms_source() -> AdmissionSource {
    AdmissionSource::new("helms.truth-catalog").expect("valid admission source")
}

#[test]
fn intent_admits_through_runtime_into_kernel() {
    let runtime = Runtime::new();
    let intent = lead_qualification_intent();
    let mut context = ContextState::default();

    let receipt = runtime
        .admit_intent(&intent, helms_actor(), helms_source(), &mut context)
        .expect("intent admits cleanly");

    assert!(
        receipt.staged(),
        "admit_observation should have staged the proposal"
    );

    let expected_id = format!("intent:{}", intent.id);
    assert_eq!(receipt.proposal_id().as_str(), expected_id);
}

#[test]
fn blank_outcome_intent_is_rejected_at_gate() {
    let runtime = Runtime::new();
    let intent = IntentPacket::new("   ", Utc::now() + Duration::hours(1));
    let mut context = ContextState::default();

    let result = runtime.admit_intent(&intent, helms_actor(), helms_source(), &mut context);
    assert!(
        matches!(result, Err(IntentAdmissionError::Rejected(_))),
        "expected Rejected for blank outcome, got {result:?}"
    );
}

#[test]
fn admission_actor_validation_surfaces_through_runtime() {
    // An empty actor id fails Converge's admission validation before reaching
    // the runtime.
    let bad_actor = AdmissionActor::new("", AdmissionActorKind::External);
    assert!(bad_actor.is_err(), "empty actor id rejected upstream");
}

#[test]
fn duplicate_admission_returns_consistent_receipt() {
    // Idempotency check: admitting the same compiled intent twice produces the
    // same proposal id under the same intent — Converge's admission layer
    // treats it as the same staged observation.
    let runtime = Runtime::new();
    let intent = lead_qualification_intent();
    let mut context = ContextState::default();

    let receipt_a = runtime
        .admit_intent(&intent, helms_actor(), helms_source(), &mut context)
        .expect("first admission");
    let receipt_b = runtime
        .admit_intent(&intent, helms_actor(), helms_source(), &mut context)
        .expect("second admission");

    assert_eq!(receipt_a.proposal_id(), receipt_b.proposal_id());
    assert_eq!(receipt_a.content_hash(), receipt_b.content_hash());
}
