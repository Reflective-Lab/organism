//! End-to-end: a user override recorded in the experience store flows back
//! into planning priors via PlanningPriorAgent's recall consultation.

use std::sync::{Arc, Mutex};

use converge_kernel::{
    ArtifactId, ArtifactKind, ContextKey, ContextState, Engine, EventQuery,
    ExperienceEventEnvelope, ExperienceRecord, ExperienceStore, ExperienceStoreResult,
    LifecycleEvent, OverrideTarget, RecallPolicy, ReplayTrace, TraceLinkId, UserExperienceEvent,
    UserExperienceEventEnvelope,
};
use converge_pack::{ActorId, UnitInterval};
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

fn prior_seed(assumption: &str, posterior: f64) -> String {
    serde_json::json!({
        "type": "prior_calibration",
        "calibration": {
            "assumption_type": assumption,
            "context": "test",
            "prior_confidence": 0.5,
            "posterior_confidence": posterior,
            "evidence_count": 1,
        }
    })
    .to_string()
}

#[tokio::test]
async fn user_override_feeds_into_planning_priors() {
    let store = Arc::new(TestStore::default());

    store
        .append_user_event(
            UserExperienceEventEnvelope::new(
                "evt-user-1",
                UserExperienceEvent::UserOverrideIssued {
                    target: OverrideTarget::Constraint("budget-cap".into()),
                    actor: ActorId::new("operator-1"),
                    policy_snapshot_hash: None,
                    reason: "budget too tight for this segment".into(),
                },
            )
            .with_correlation("scope-1"),
        )
        .expect("append user event");

    let policy = RecallPolicy {
        prior_weight: UnitInterval::clamped(0.8),
        ..RecallPolicy::enabled()
    };

    let agent = PlanningPriorAgent::new().with_recall(store.clone(), policy);

    let mut engine = Engine::default();
    engine.register_suggestor(agent);

    let mut ctx = ContextState::default();
    let _ = ctx.add_input_with_provenance(
        ContextKey::Seeds,
        "prior-1",
        prior_seed("cost_accuracy", 0.7),
        "test",
    );

    let result = engine.run(ctx).await.expect("converges");
    assert!(result.converged);

    let hypotheses = result.context.get(ContextKey::Hypotheses);

    let recall_summary = hypotheses
        .iter()
        .find(|f| f.id().as_str() == "recall-summary")
        .expect("recall-summary hypothesis published");
    let parsed: serde_json::Value =
        serde_json::from_str(recall_summary.text().unwrap_or_default()).expect("valid json");
    assert_eq!(parsed["count"], serde_json::json!(1));

    let prior = hypotheses
        .iter()
        .find(|f| f.id().as_str() == "prior-cost_accuracy")
        .expect("prior hypothesis published");
    let prior_payload: serde_json::Value =
        serde_json::from_str(prior.text().unwrap_or_default()).expect("valid json");
    assert!(prior_payload["recall_signal"].is_number());
    assert_eq!(prior_payload["recall_count"], serde_json::json!(1));

    let blended = prior_payload["confidence"]
        .as_f64()
        .expect("confidence number");
    let raw_posterior = prior_payload["raw_posterior"]
        .as_f64()
        .expect("raw_posterior number");
    assert!(
        (blended - raw_posterior).abs() > f64::EPSILON,
        "recall should shift posterior; got blended={blended} raw={raw_posterior}"
    );
}
