//! Bidirectional ExperienceStore variant consumption.
//!
//! Verifies that the three new 3.8.x user-event variants
//! (`UserApprovalRejected`, `UserCorrection`, `UserBoundaryAdjusted`) flow
//! through `PlanningPriorAgent::consult_recall` end-to-end. The two pre-3.8
//! variants (`UserApprovalGranted`, `UserOverrideIssued`) are covered by
//! sibling tests in this crate and `crates/runtime/tests/recall_biases_synthesis.rs`.
//!
//! The test asserts the recall-summary hypothesis published by the agent
//! carries a candidate per appended event with the (source_type, confidence)
//! mapping spec'd in `kb/Concepts/Bidirectional ExperienceStore.md`:
//!
//! | Variant                | source_type   | confidence |
//! |------------------------|---------------|------------|
//! | UserApprovalGranted    | similar_success | 0.7      |
//! | UserApprovalRejected   | anti_pattern  | 0.7        |
//! | UserOverrideIssued     | anti_pattern  | 0.9        |
//! | UserCorrection         | runbook       | 0.85       |
//! | UserBoundaryAdjusted   | runbook       | 0.8        |

use std::sync::{Arc, Mutex};

use converge_kernel::{
    ArtifactId, ArtifactKind, BoundaryKind, BoundaryTarget, ContextKey, ContextState,
    CorrectionTarget, Engine, EventQuery, ExperienceEventEnvelope, ExperienceRecord,
    ExperienceStore, ExperienceStoreResult, FactContent, FactContentKind, LifecycleEvent,
    OverrideTarget, PackId, RecallPolicy, ReplayTrace, TraceLinkId, TypesIntentId,
    UserExperienceEvent, UserExperienceEventEnvelope,
};
use converge_pack::{ActorId, ContentHash, FactId, GateId, UnitInterval};
use organism_learning::PlanningPriorAgent;

#[derive(Default)]
struct TestStore {
    user_events: Mutex<Vec<UserExperienceEventEnvelope>>,
}

impl ExperienceStore for TestStore {
    fn append_event(&self, _event: ExperienceEventEnvelope) -> ExperienceStoreResult<()> {
        Ok(())
    }

    fn query_events(
        &self,
        _query: &EventQuery,
    ) -> ExperienceStoreResult<Vec<ExperienceEventEnvelope>> {
        Ok(Vec::new())
    }

    fn write_artifact_state_transition(
        &self,
        _artifact_id: &ArtifactId,
        _artifact_kind: ArtifactKind,
        _event: LifecycleEvent,
    ) -> ExperienceStoreResult<()> {
        Ok(())
    }

    fn get_trace_link(
        &self,
        _trace_link_id: &TraceLinkId,
    ) -> ExperienceStoreResult<Option<ReplayTrace>> {
        Ok(None)
    }

    fn append_user_event(&self, event: UserExperienceEventEnvelope) -> ExperienceStoreResult<()> {
        self.user_events.lock().unwrap().push(event);
        Ok(())
    }

    fn query_records(&self, _query: &EventQuery) -> ExperienceStoreResult<Vec<ExperienceRecord>> {
        Ok(self
            .user_events
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .map(ExperienceRecord::User)
            .collect())
    }
}

fn prior_seed_content() -> String {
    serde_json::json!({
        "type": "prior_calibration",
        "calibration": {
            "assumption_type": "cost_accuracy",
            "context": "test",
            "prior_confidence": 0.5,
            "posterior_confidence": 0.7,
            "evidence_count": 1,
        }
    })
    .to_string()
}

async fn run_with_events(events: Vec<UserExperienceEvent>) -> serde_json::Value {
    let store: Arc<TestStore> = Arc::new(TestStore::default());
    for (i, event) in events.into_iter().enumerate() {
        store
            .append_user_event(UserExperienceEventEnvelope::new(format!("evt-{i}"), event))
            .expect("append user event");
    }
    let policy = RecallPolicy {
        prior_weight: UnitInterval::clamped(1.0),
        ..RecallPolicy::enabled()
    };
    let agent = PlanningPriorAgent::new().with_recall(store, policy);

    let mut engine = Engine::default();
    engine.register_suggestor(agent);
    let mut ctx = ContextState::default();
    let _ =
        ctx.add_input_with_provenance(ContextKey::Seeds, "prior-1", prior_seed_content(), "test");
    let result = engine.run(ctx).await.expect("converges");
    let hypotheses = result.context.get(ContextKey::Hypotheses);
    let recall_summary = hypotheses
        .iter()
        .find(|f| f.id().as_str() == "recall-summary")
        .expect("recall-summary hypothesis published");
    serde_json::from_str(recall_summary.content()).expect("recall-summary is valid JSON")
}

fn user_approval_rejected() -> UserExperienceEvent {
    UserExperienceEvent::UserApprovalRejected {
        gate_request_id: GateId::new("g-budget"),
        actor: ActorId::new("op-1"),
        policy_snapshot_hash: None,
        reason: Some("not aligned with quarterly plan".into()),
    }
}

fn user_correction() -> UserExperienceEvent {
    UserExperienceEvent::UserCorrection {
        target: CorrectionTarget::Fact {
            fact_id: FactId::new("f-42"),
        },
        actor: ActorId::new("op-1"),
        policy_snapshot_hash: None,
        original_content: ContentHash::zero(),
        corrected_content: FactContent::new(FactContentKind::Claim, "corrected amount: 1000"),
        reason: "amount field was off by one decimal".into(),
    }
}

fn user_boundary_adjusted() -> UserExperienceEvent {
    UserExperienceEvent::UserBoundaryAdjusted {
        boundary: BoundaryKind::Forbidden,
        target: BoundaryTarget::Pack {
            pack_id: PackId::new("customers"),
        },
        actor: ActorId::new("op-1"),
        policy_snapshot_hash: None,
        previous_value: serde_json::json!(["delete_account"]),
        new_value: serde_json::json!(["delete_account", "merge_accounts"]),
        reason: "merge events should follow the same gate as deletes".into(),
    }
}

/// Asserts a candidate exists with the expected (source, confidence) and that
/// `summary` contains the expected substring. The recall-summary hypothesis
/// embeds candidates as a JSON array under `candidates`.
fn assert_candidate(
    summary: &serde_json::Value,
    expected_source: &str,
    expected_confidence: f64,
    summary_substring: &str,
) {
    let candidates = summary["candidates"]
        .as_array()
        .expect("candidates is an array");
    let matched = candidates.iter().find(|c| {
        c["source"].as_str() == Some(expected_source)
            && c["summary"]
                .as_str()
                .is_some_and(|s| s.contains(summary_substring))
    });
    assert!(
        matched.is_some(),
        "expected candidate with source={expected_source} containing '{summary_substring}' in {candidates:?}"
    );
    let candidate = matched.unwrap();
    let confidence = candidate["confidence"]["bps"]
        .as_u64()
        .map(|bps| (bps as f64) / 10_000.0)
        .or_else(|| candidate["confidence"].as_f64())
        .expect("confidence parses as f64");
    assert!(
        (confidence - expected_confidence).abs() < 1e-3,
        "expected confidence={expected_confidence}, got {confidence} for source={expected_source}"
    );
}

#[tokio::test]
async fn user_approval_rejected_becomes_anti_pattern_recall() {
    let summary = run_with_events(vec![user_approval_rejected()]).await;
    assert_eq!(summary["count"], serde_json::json!(1));
    assert_candidate(&summary, "anti_pattern", 0.7, "user rejection");
}

#[tokio::test]
async fn user_correction_becomes_runbook_recall() {
    let summary = run_with_events(vec![user_correction()]).await;
    assert_eq!(summary["count"], serde_json::json!(1));
    assert_candidate(&summary, "runbook", 0.85, "correction");
}

#[tokio::test]
async fn user_boundary_adjusted_becomes_runbook_recall() {
    let summary = run_with_events(vec![user_boundary_adjusted()]).await;
    assert_eq!(summary["count"], serde_json::json!(1));
    assert_candidate(&summary, "runbook", 0.8, "boundary adjusted");
}

#[tokio::test]
async fn all_three_new_variants_flow_through_together() {
    let summary = run_with_events(vec![
        user_approval_rejected(),
        user_correction(),
        user_boundary_adjusted(),
    ])
    .await;
    assert_eq!(summary["count"], serde_json::json!(3));
    assert_candidate(&summary, "anti_pattern", 0.7, "user rejection");
    assert_candidate(&summary, "runbook", 0.85, "correction");
    assert_candidate(&summary, "runbook", 0.8, "boundary adjusted");
}

#[tokio::test]
async fn full_loop_with_all_five_variants_produces_distinct_candidates() {
    // Smoke test: the complete bidirectional ledger end-to-end.
    let summary = run_with_events(vec![
        UserExperienceEvent::UserApprovalGranted {
            gate_request_id: GateId::new("g-approve"),
            actor: ActorId::new("op-1"),
            policy_snapshot_hash: None,
            reason: Some("approved by sponsor".into()),
        },
        user_approval_rejected(),
        UserExperienceEvent::UserOverrideIssued {
            target: OverrideTarget::Constraint("budget-cap".into()),
            actor: ActorId::new("op-1"),
            policy_snapshot_hash: None,
            reason: "budget too tight".into(),
        },
        user_correction(),
        user_boundary_adjusted(),
    ])
    .await;
    assert_eq!(summary["count"], serde_json::json!(5));
    let sources: Vec<&str> = summary["candidates"]
        .as_array()
        .expect("candidates array")
        .iter()
        .filter_map(|c| c["source"].as_str())
        .collect();
    assert!(sources.contains(&"similar_success"));
    assert!(sources.contains(&"anti_pattern"));
    assert!(sources.contains(&"runbook"));
    // anti_pattern appears twice (rejection + override); runbook twice (correction + boundary).
    assert_eq!(sources.iter().filter(|s| **s == "anti_pattern").count(), 2);
    assert_eq!(sources.iter().filter(|s| **s == "runbook").count(), 2);
}

/// Scope-honoring contract: `UserBoundaryAdjusted` carries `target` so consumers
/// can filter pack-scoped or intent-scoped adjustments out of unrelated planning.
/// This test pins the contract by asserting the candidate summary preserves the
/// scope label so a downstream filter can read it. Filtering itself is the
/// consumer's responsibility per the spec.
#[tokio::test]
async fn boundary_adjusted_scope_is_visible_in_summary() {
    let pack_scoped = UserExperienceEvent::UserBoundaryAdjusted {
        boundary: BoundaryKind::Authority,
        target: BoundaryTarget::Pack {
            pack_id: PackId::new("partnerships"),
        },
        actor: ActorId::new("op-1"),
        policy_snapshot_hash: None,
        previous_value: serde_json::json!({"actor": "rev_lead"}),
        new_value: serde_json::json!({"actor": "rev_director"}),
        reason: "partnerships authority elevated".into(),
    };
    let intent_scoped = UserExperienceEvent::UserBoundaryAdjusted {
        boundary: BoundaryKind::Reversibility,
        target: BoundaryTarget::Intent {
            intent_id: TypesIntentId::new("intent-99"),
        },
        actor: ActorId::new("op-1"),
        policy_snapshot_hash: None,
        previous_value: serde_json::json!("reversible"),
        new_value: serde_json::json!("irreversible"),
        reason: "single-intent classification override".into(),
    };

    let summary = run_with_events(vec![pack_scoped, intent_scoped]).await;
    assert_eq!(summary["count"], serde_json::json!(2));
    let summaries: Vec<&str> = summary["candidates"]
        .as_array()
        .expect("candidates array")
        .iter()
        .filter_map(|c| c["summary"].as_str())
        .collect();
    // Pack scope visible.
    assert!(
        summaries.iter().any(|s| s.to_lowercase().contains("pack")),
        "pack-scoped boundary summary should surface the scope; got {summaries:?}"
    );
    // Intent scope visible.
    assert!(
        summaries
            .iter()
            .any(|s| s.to_lowercase().contains("intent")),
        "intent-scoped boundary summary should surface the scope; got {summaries:?}"
    );
}
