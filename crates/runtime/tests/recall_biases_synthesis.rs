//! Recall confidence biases synthesis proposal output.
//!
//! Pairs `PlanningPriorAgent` (which consults the experience store and
//! publishes a `recall-summary` hypothesis) with a recall-aware
//! `RoundSynthesizer` in one Formation. Asserts that the synthesis
//! `ProposedFact` content reflects the recall avg confidence when an event is
//! present, and falls back to a `no_recall` marker when the store is empty.
//!
//! This is the "RecallCandidate.confidence shifts proposal generation during
//! huddle runs" integration target: a high-confidence recalled candidate
//! visibly biases the next proposal.
//!
//! Sibling test `crates/learning/tests/recall_feeds_priors.rs` covers the
//! upstream half of the loop (recall → blended posterior in the prior
//! hypothesis). This test covers the downstream half (recall → synthesis
//! proposal content).

use std::sync::{Arc, Mutex};

use converge_kernel::{
    ArtifactId, ArtifactKind, ContextKey, EventQuery, ExperienceEventEnvelope, ExperienceRecord,
    ExperienceStore, ExperienceStoreResult, LifecycleEvent, RecallPolicy, ReplayTrace, TraceLinkId,
    UserExperienceEvent, UserExperienceEventEnvelope,
};
use converge_pack::{ActorId, Context, ContextFact, GateId, UnitInterval};
use organism_learning::PlanningPriorAgent;
use organism_runtime::{Formation, RoundConventions, RoundSynthesizer, SynthesisProducer};

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

/// Synthesis producer that consults the `recall-summary` hypothesis emitted by
/// `PlanningPriorAgent` and embeds the recall avg confidence into its output.
/// When recall is absent, emits a `no_recall` marker.
struct RecallAwareProducer;

#[async_trait::async_trait]
impl SynthesisProducer for RecallAwareProducer {
    async fn synthesize(
        &self,
        round: u8,
        _notes: &[ContextFact],
        ctx: &dyn Context,
    ) -> Result<String, String> {
        let summary = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .find(|f| f.id().as_str() == "recall-summary")
            .cloned();
        match summary {
            Some(fact) => {
                let v: serde_json::Value = serde_json::from_str(fact.text().unwrap_or_default())
                    .map_err(|e| e.to_string())?;
                let avg = v["avg_confidence"].as_f64().unwrap_or(0.0);
                let count = v["count"].as_u64().unwrap_or(0);
                Ok(format!(
                    "round-{round}|recall_count={count}|recall_avg={avg:.4}"
                ))
            }
            None => Ok(format!("round-{round}|no_recall")),
        }
    }
}

/// Custom conventions that keep round notes out of `Hypotheses`, so
/// `PlanningPriorAgent::accepts` (which requires `!ctx.has(Hypotheses)`) still
/// fires before the synthesizer runs.
fn test_conventions() -> RoundConventions {
    RoundConventions {
        round_signal_key: ContextKey::Signals,
        round_signal_prefix: "round:start:",
        continue_key: ContextKey::Signals,
        continue_prefix: "round:continue:",
        note_key: ContextKey::Proposals,
        synthesis_key: ContextKey::Strategies,
        synthesis_prefix: "synthesis:",
    }
}

#[tokio::test]
async fn high_confidence_recall_biases_synthesis_proposal() {
    let store: Arc<TestStore> = Arc::new(TestStore::default());
    store
        .append_user_event(UserExperienceEventEnvelope::new(
            "evt-approval-1",
            UserExperienceEvent::UserApprovalGranted {
                gate_request_id: GateId::new("budget-approval"),
                actor: ActorId::new("operator-1"),
                policy_snapshot_hash: None,
                reason: Some("approved by sponsor".into()),
            },
        ))
        .expect("append user event");

    let policy = RecallPolicy {
        prior_weight: UnitInterval::clamped(1.0),
        ..RecallPolicy::enabled()
    };
    let prior_agent = PlanningPriorAgent::new().with_recall(store.clone(), policy);
    let synthesizer =
        RoundSynthesizer::new(1, RecallAwareProducer).with_conventions(test_conventions());

    let result = Formation::new("recall-biased-synthesis")
        .agent(prior_agent)
        .agent(synthesizer)
        .seed(ContextKey::Seeds, "prior-1", prior_seed_content(), "test")
        .seed(ContextKey::Signals, "round:start:1", "start", "test")
        .seed(
            ContextKey::Proposals,
            "note:alice:1",
            "alice's note",
            "test",
        )
        .run()
        .await
        .expect("formation should converge");

    let strategies = result.converge_result.context.get(ContextKey::Strategies);
    let synthesis = strategies
        .iter()
        .find(|f| f.id().as_str() == "synthesis:1")
        .expect("synthesis fact published");
    let content = synthesis.text().unwrap_or_default();
    assert!(
        content.contains("recall_count=1"),
        "expected recall to bias synthesis, got: {content}"
    );
    assert!(
        !content.contains("no_recall"),
        "synthesis should not fall back to no_recall when store has events: {content}"
    );

    // High-confidence (UserApprovalGranted = 0.7 base, prior_weight 1.0) should
    // produce avg_confidence > min_score_threshold default (0.5).
    let avg_str = content
        .split("recall_avg=")
        .nth(1)
        .expect("recall_avg embedded in synthesis");
    let avg: f64 = avg_str.parse().expect("avg parses as f64");
    assert!(
        avg > 0.5,
        "high-confidence recall should produce avg > threshold; got {avg}"
    );
}

#[tokio::test]
async fn empty_store_falls_back_to_no_recall_synthesis() {
    let empty_store: Arc<TestStore> = Arc::new(TestStore::default());

    let policy = RecallPolicy {
        prior_weight: UnitInterval::clamped(1.0),
        ..RecallPolicy::enabled()
    };
    let prior_agent = PlanningPriorAgent::new().with_recall(empty_store, policy);
    let synthesizer =
        RoundSynthesizer::new(1, RecallAwareProducer).with_conventions(test_conventions());

    let result = Formation::new("no-recall-synthesis")
        .agent(prior_agent)
        .agent(synthesizer)
        .seed(ContextKey::Seeds, "prior-1", prior_seed_content(), "test")
        .seed(ContextKey::Signals, "round:start:1", "start", "test")
        .seed(
            ContextKey::Proposals,
            "note:alice:1",
            "alice's note",
            "test",
        )
        .run()
        .await
        .expect("formation should converge");

    let strategies = result.converge_result.context.get(ContextKey::Strategies);
    let synthesis = strategies
        .iter()
        .find(|f| f.id().as_str() == "synthesis:1")
        .expect("synthesis fact published");
    let content = synthesis.text().unwrap_or_default();
    assert!(
        content.ends_with("|no_recall"),
        "expected no_recall fallback when store is empty, got: {content}"
    );
}
