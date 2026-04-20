//! Planning prior agent — Suggestor that injects calibrated priors into context.
//!
//! Reads prior calibrations from Seeds and publishes them as Hypotheses so
//! downstream simulation and adversarial agents can factor in historical
//! accuracy when evaluating new plans.

use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

use crate::PriorCalibration;

/// Reads prior calibrations from Seeds and publishes confidence adjustments
/// as Hypotheses for downstream consumers.
///
/// This closes the learning loop: execution outcomes → calibrate_priors() →
/// store as seeds → PlanningPriorAgent reads them → downstream agents use them.
pub struct PlanningPriorAgent;

impl PlanningPriorAgent {
    #[must_use]
    pub fn new() -> Self {
        Self
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

        if priors.is_empty() {
            return AgentEffect::empty();
        }

        // Publish each prior as a hypothesis for downstream consumers
        for prior in &priors {
            let adjustment = prior.posterior_confidence - prior.prior_confidence;
            let direction = if adjustment > 0.0 { "up" } else { "down" };

            proposals.push(ProposedFact::new(
                ContextKey::Hypotheses,
                format!("prior-{}", prior.assumption_type),
                serde_json::json!({
                    "type": "planning_prior",
                    "assumption_type": prior.assumption_type,
                    "confidence": prior.posterior_confidence,
                    "evidence_count": prior.evidence_count,
                    "adjustment": adjustment,
                    "direction": direction,
                    "context": prior.context,
                })
                .to_string(),
                "planning-prior",
            ));
        }

        // Also publish a summary for quick consumption
        let avg_confidence: f64 = priors.iter().map(|p| p.posterior_confidence).sum::<f64>()
            / f64::from(u32::try_from(priors.len()).unwrap_or(1));

        proposals.push(ProposedFact::new(
            ContextKey::Hypotheses,
            String::from("prior-summary"),
            serde_json::json!({
                "type": "planning_prior_summary",
                "prior_count": priors.len(),
                "avg_confidence": avg_confidence,
                "priors": priors.iter().map(|p| serde_json::json!({
                    "assumption": &p.assumption_type,
                    "confidence": p.posterior_confidence,
                    "evidence": p.evidence_count,
                })).collect::<Vec<_>>(),
            })
            .to_string(),
            "planning-prior",
        ));

        AgentEffect::with_proposals(proposals)
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
