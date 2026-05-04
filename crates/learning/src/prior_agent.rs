//! Planning prior agent — Suggestor that injects calibrated priors into context.
//!
//! Reads prior calibrations from Seeds and publishes them as Hypotheses so
//! downstream simulation and adversarial agents can factor in historical
//! accuracy when evaluating new plans.
//!
//! When configured with an `ExperienceStore`, the agent additionally consults
//! recall during execute(): user-side trust events (overrides, approvals) and
//! prior failed outcomes weight the priors emitted to Hypotheses, closing the
//! "experience flows backward into planning" loop.

use std::sync::Arc;

use converge_kernel::{ExperienceStore, RecallPolicy, RecallQuery, recall_from_store};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

use crate::PriorCalibration;

/// Reads prior calibrations from Seeds and publishes confidence adjustments
/// as Hypotheses for downstream consumers.
///
/// This closes the learning loop: execution outcomes → calibrate_priors() →
/// store as seeds → PlanningPriorAgent reads them → downstream agents use them.
pub struct PlanningPriorAgent {
    recall: Option<RecallSource>,
}

struct RecallSource {
    store: Arc<dyn ExperienceStore>,
    policy: RecallPolicy,
    tenant_scope: Option<String>,
}

impl PlanningPriorAgent {
    #[must_use]
    pub fn new() -> Self {
        Self { recall: None }
    }

    /// Configure recall consultation. When the agent runs, it will pull
    /// recall candidates from `store` and weight prior confidence by each
    /// candidate's confidence times `policy.prior_weight`.
    #[must_use]
    pub fn with_recall(mut self, store: Arc<dyn ExperienceStore>, policy: RecallPolicy) -> Self {
        self.recall = Some(RecallSource {
            store,
            policy,
            tenant_scope: None,
        });
        self
    }

    /// Optional tenant scope applied to recall queries.
    #[must_use]
    pub fn with_tenant_scope(mut self, tenant_scope: impl Into<String>) -> Self {
        if let Some(ref mut source) = self.recall {
            source.tenant_scope = Some(tenant_scope.into());
        }
        self
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for PlanningPriorAgent {
    fn name(&self) -> &'static str {
        "planning-prior"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Run once at the start: seeds exist, hypotheses don't yet
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut proposals = Vec::new();

        // Look for prior calibration seeds
        let priors: Vec<PriorCalibration> = seeds
            .iter()
            .filter_map(|fact| {
                let v: serde_json::Value = serde_json::from_str(&fact.content).ok()?;
                if v.get("type").and_then(|t| t.as_str()) == Some("prior_calibration") {
                    serde_json::from_value(v.get("calibration")?.clone()).ok()
                } else {
                    None
                }
            })
            .collect();

        let recall = self.consult_recall();

        if priors.is_empty() && recall.is_none() {
            return AgentEffect::empty();
        }

        let recall_signal = recall.as_ref().map(RecallSummary::avg_confidence);

        // Publish each prior as a hypothesis, blended toward the recall signal
        // when one is available. Blend ratio = 0.3 (recall pulls posterior 30%
        // toward recent experience).
        for prior in &priors {
            let blended = recall_signal
                .map_or(prior.posterior_confidence, |signal| {
                    prior.posterior_confidence + (signal - prior.posterior_confidence) * 0.3
                })
                .clamp(0.0, 1.0);
            let adjustment = blended - prior.prior_confidence;
            let direction = if adjustment > 0.0 { "up" } else { "down" };

            proposals.push(ProposedFact::new(
                ContextKey::Hypotheses,
                format!("prior-{}", prior.assumption_type),
                serde_json::json!({
                    "type": "planning_prior",
                    "assumption_type": prior.assumption_type,
                    "confidence": blended,
                    "raw_posterior": prior.posterior_confidence,
                    "recall_signal": recall_signal,
                    "recall_count": recall.as_ref().map_or(0, |r| r.count),
                    "evidence_count": prior.evidence_count,
                    "adjustment": adjustment,
                    "direction": direction,
                    "context": prior.context,
                })
                .to_string(),
                "planning-prior",
            ));
        }

        if !priors.is_empty() {
            let avg_confidence: f64 = priors.iter().map(|p| p.posterior_confidence).sum::<f64>()
                / f64::from(u32::try_from(priors.len()).unwrap_or(1));

            proposals.push(ProposedFact::new(
                ContextKey::Hypotheses,
                String::from("prior-summary"),
                serde_json::json!({
                    "type": "planning_prior_summary",
                    "prior_count": priors.len(),
                    "avg_confidence": avg_confidence,
                    "recall_signal": recall_signal,
                    "priors": priors.iter().map(|p| serde_json::json!({
                        "assumption": &p.assumption_type,
                        "confidence": p.posterior_confidence,
                        "evidence": p.evidence_count,
                    })).collect::<Vec<_>>(),
                })
                .to_string(),
                "planning-prior",
            ));
        }

        if let Some(summary) = recall {
            proposals.push(ProposedFact::new(
                ContextKey::Hypotheses,
                String::from("recall-summary"),
                serde_json::json!({
                    "type": "recall_summary",
                    "count": summary.count,
                    "avg_confidence": summary.avg_confidence(),
                    "candidates": summary.candidate_summaries,
                })
                .to_string(),
                "planning-prior",
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

struct RecallSummary {
    count: usize,
    total_confidence: f64,
    candidate_summaries: Vec<serde_json::Value>,
}

impl RecallSummary {
    fn avg_confidence(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_confidence / f64::from(u32::try_from(self.count).unwrap_or(1))
        }
    }
}

impl PlanningPriorAgent {
    fn consult_recall(&self) -> Option<RecallSummary> {
        let source = self.recall.as_ref()?;
        let mut query = RecallQuery::new("planning-priors", source.policy.max_k_total);
        if let Some(ref scope) = source.tenant_scope {
            query = query.with_tenant_scope(scope);
        }
        let candidates = recall_from_store(source.store.as_ref(), &query, &source.policy).ok()?;
        if candidates.is_empty() {
            return None;
        }
        let total_confidence = candidates.iter().map(|c| c.confidence).sum();
        let candidate_summaries = candidates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "summary": c.summary,
                    "confidence": c.confidence,
                    "source": match c.source_type {
                        converge_kernel::CandidateSourceType::SimilarFailure => "similar_failure",
                        converge_kernel::CandidateSourceType::SimilarSuccess => "similar_success",
                        converge_kernel::CandidateSourceType::Runbook => "runbook",
                        converge_kernel::CandidateSourceType::AdapterConfig => "adapter_config",
                        converge_kernel::CandidateSourceType::AntiPattern => "anti_pattern",
                    },
                })
            })
            .collect();
        Some(RecallSummary {
            count: candidates.len(),
            total_confidence,
            candidate_summaries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{ContextState, Engine};
    use converge_pack::ContextKey;

    fn make_prior_seed(assumption: &str, confidence: f64, evidence: u32) -> String {
        serde_json::json!({
            "type": "prior_calibration",
            "calibration": {
                "assumption_type": assumption,
                "context": "test",
                "prior_confidence": 0.5,
                "posterior_confidence": confidence,
                "evidence_count": evidence,
            }
        })
        .to_string()
    }

    #[tokio::test]
    async fn publishes_priors_as_hypotheses() {
        // Run through Engine so seeds get promoted before agent sees them
        let mut engine = Engine::default();
        engine.register_suggestor(PlanningPriorAgent::new());

        let mut ctx = ContextState::default();
        let _ = ctx.add_input_with_provenance(
            ContextKey::Seeds,
            "prior-1",
            make_prior_seed("cost_accuracy", 0.7, 5),
            "test",
        );

        let result = engine.run(ctx).await.expect("should converge");
        assert!(result.converged);

        let hypotheses = result.context.get(ContextKey::Hypotheses);
        assert!(hypotheses.len() >= 2); // one per prior + summary
        assert!(
            hypotheses
                .iter()
                .any(|f| f.content.contains("cost_accuracy"))
        );
    }

    #[tokio::test]
    async fn no_priors_produces_no_hypotheses() {
        let mut engine = Engine::default();
        engine.register_suggestor(PlanningPriorAgent::new());

        let mut ctx = ContextState::default();
        let _ = ctx.add_input(ContextKey::Seeds, "other", r#"{"type": "intent"}"#);

        let result = engine.run(ctx).await.expect("should converge");
        assert!(!result.context.has(ContextKey::Hypotheses));
    }

    #[tokio::test]
    async fn does_not_run_when_hypotheses_already_exist() {
        let mut engine = Engine::default();
        engine.register_suggestor(PlanningPriorAgent::new());

        let mut ctx = ContextState::default();
        let _ = ctx.add_input_with_provenance(
            ContextKey::Seeds,
            "prior-1",
            make_prior_seed("x", 0.8, 3),
            "test",
        );
        let _ = ctx.add_input_with_provenance(
            ContextKey::Hypotheses,
            "existing",
            "already here",
            "test",
        );

        let result = engine.run(ctx).await.expect("should converge");
        // Should only have the pre-existing hypothesis, not generate new ones
        let hypotheses = result.context.get(ContextKey::Hypotheses);
        assert_eq!(hypotheses.len(), 1);
        assert_eq!(hypotheses[0].content, "already here");
    }
}
