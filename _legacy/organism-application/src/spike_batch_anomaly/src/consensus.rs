// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Consensus agents for Spike 4: aggregation, triage, and recommendation.
//!
//! These agents form the "decision engine" layer:
//! 1. Aggregate multi-agent scores into a composite signal
//! 2. Feed into converge-optimization's `anomaly_triage` pack for prioritization
//! 3. Produce a final ranked recommendation

use std::collections::BTreeMap;

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact, ProposedFact};
use converge_optimization::gate::{ObjectiveSpec, ProblemSpec};
use converge_optimization::packs::Pack;
use converge_optimization::packs::anomaly_triage::{
    Anomaly, AnomalyTriageInput, AnomalyTriagePack, EscalationPolicy, SeverityThresholds,
};

use crate::agents::UserScore;

/// Composite score for a single user (all agents combined).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompositeScore {
    pub user_id: String,
    pub temporal_z: f64,
    pub burst_z: f64,
    pub behavior_z: f64,
    pub ml_z: f64,
    pub composite: f64,
}

// ─── Agent 5: Score Aggregation ─────────────────────────────────────────────

/// Aggregates scores from all 4 scoring agents into a weighted composite.
///
/// Weights: temporal=0.3, burst=0.25, behavior=0.25, ml=0.2
pub struct ScoreAggregationAgent;

const AGENT_IDS: &[&str] = &[
    "temporal_scores",
    "burst_scores",
    "behavior_scores",
    "ml_scores",
];
const WEIGHTS: &[f64] = &[0.30, 0.25, 0.25, 0.20];

impl Agent for ScoreAggregationAgent {
    fn name(&self) -> &str {
        "ScoreAggregationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_all = AGENT_IDS.iter().all(|id| {
            ctx.get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id == *id)
        });
        let has_aggregated = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "aggregated_scores");
        has_all && !has_aggregated
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Parse scores from each agent.
        let mut agent_scores: Vec<BTreeMap<String, f64>> = Vec::new();

        for agent_id in AGENT_IDS {
            let scores_map: BTreeMap<String, f64> = ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .find(|f| f.id == *agent_id)
                .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
                .and_then(|v| {
                    v.get("scores")
                        .and_then(|s| serde_json::from_value::<Vec<UserScore>>(s.clone()).ok())
                })
                .map(|scores| {
                    scores
                        .into_iter()
                        .map(|s| (s.user_id, s.z_score))
                        .collect()
                })
                .unwrap_or_default();
            agent_scores.push(scores_map);
        }

        // Collect all user_ids.
        let user_ids: Vec<String> = agent_scores
            .first()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();

        // Compute weighted composite for each user.
        let composites: Vec<CompositeScore> = user_ids
            .iter()
            .map(|uid| {
                let temporal_z = agent_scores[0].get(uid).copied().unwrap_or(0.0);
                let burst_z = agent_scores[1].get(uid).copied().unwrap_or(0.0);
                let behavior_z = agent_scores[2].get(uid).copied().unwrap_or(0.0);
                let ml_z = agent_scores[3].get(uid).copied().unwrap_or(0.0);
                let composite = temporal_z * WEIGHTS[0]
                    + burst_z * WEIGHTS[1]
                    + behavior_z * WEIGHTS[2]
                    + ml_z * WEIGHTS[3];
                CompositeScore {
                    user_id: uid.clone(),
                    temporal_z,
                    burst_z,
                    behavior_z,
                    ml_z,
                    composite,
                }
            })
            .collect();

        AgentEffect::with_fact(Fact::new(
            ContextKey::Strategies,
            "aggregated_scores",
            serde_json::to_string(&composites).unwrap_or_default(),
        ))
    }
}

// ─── Agent 6: Anomaly Triage (converge-optimization) ────────────────────────

/// Feeds composite scores into the `anomaly_triage` optimization pack.
///
/// This is where CP-SAT / operations research meets ML:
/// - ML predicts what's important (z-scores)
/// - The triage pack decides what to do given constraints (escalation policies, SLAs)
pub struct AnomalyTriageAgent;

impl Agent for AnomalyTriageAgent {
    fn name(&self) -> &str {
        "AnomalyTriageAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "aggregated_scores")
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|f| f.id == "anomaly_triage")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let composites: Vec<CompositeScore> = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|f| f.id == "aggregated_scores")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        // Convert composite scores to anomaly_triage Anomaly structs.
        let anomalies: Vec<Anomaly> = composites
            .iter()
            .map(|c| Anomaly {
                id: c.user_id.clone(),
                timestamp: 0, // batch — not relevant
                source: "multi_agent_scoring".to_string(),
                z_score: c.composite,
                features: serde_json::json!({
                    "temporal_z": c.temporal_z,
                    "burst_z": c.burst_z,
                    "behavior_z": c.behavior_z,
                    "ml_z": c.ml_z,
                }),
            })
            .collect();

        let input = AnomalyTriageInput {
            anomalies,
            thresholds: SeverityThresholds {
                critical: 3.0,
                high: 2.0,
                medium: 1.0,
            },
            escalation_policies: vec![
                EscalationPolicy {
                    severity_level: "critical".to_string(),
                    auto_escalate: true,
                    notify_channels: vec!["security-team".to_string(), "oncall".to_string()],
                    response_sla_minutes: 15,
                },
                EscalationPolicy {
                    severity_level: "high".to_string(),
                    auto_escalate: true,
                    notify_channels: vec!["security-team".to_string()],
                    response_sla_minutes: 60,
                },
            ],
        };

        let pack = AnomalyTriagePack;
        let spec = match ProblemSpec::builder("spike-4-triage", "organism-application")
            .objective(ObjectiveSpec::minimize("risk"))
            .inputs(&input)
            .and_then(|b| b.seed(42).build())
        {
            Ok(spec) => spec,
            Err(e) => {
                return AgentEffect::with_fact(Fact::new(
                    ContextKey::Diagnostic,
                    "triage_error",
                    format!("failed to build problem spec: {e}"),
                ));
            }
        };

        match pack.solve(&spec) {
            Ok(result) => {
                let invariant_results = pack
                    .check_invariants(&result.plan)
                    .unwrap_or_default();
                let gate = pack.evaluate_gate(&result.plan, &invariant_results);

                // Extract the output for display.
                let output: serde_json::Value = result
                    .plan
                    .plan
                    .clone();

                let triage_result = serde_json::json!({
                    "output": output,
                    "severity_summary": output.get("severity_summary"),
                    "escalation_count": output.get("escalation_count"),
                    "triaged_count": output.get("triaged").and_then(|t| t.as_array()).map(|a| a.len()),
                    "gate_decision": format!("{:?}", gate.decision),
                    "gate_rationale": gate.rationale,
                    "solver_reports": serde_json::to_value(&result.reports).ok(),
                });

                AgentEffect::with_fact(Fact::new(
                    ContextKey::Evaluations,
                    "anomaly_triage",
                    triage_result.to_string(),
                ))
            }
            Err(e) => AgentEffect::with_fact(Fact::new(
                ContextKey::Diagnostic,
                "triage_solve_error",
                format!("triage solve failed: {e}"),
            )),
        }
    }
}

// ─── Agent 7: Recommendation ────────────────────────────────────────────────

/// Produces the final ranked recommendation from triage results.
pub struct RecommendationAgent;

impl Agent for RecommendationAgent {
    fn name(&self) -> &str {
        "RecommendationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "anomaly_triage")
            && !ctx
                .get(ContextKey::Evaluations)
                .iter()
                .any(|f| f.id == "recommendation")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triage: serde_json::Value = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "anomaly_triage")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        let composites: Vec<CompositeScore> = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|f| f.id == "aggregated_scores")
            .and_then(|f| serde_json::from_str(&f.content).ok())
            .unwrap_or_default();

        // Build top-flagged list from triage output.
        let triaged = triage
            .get("output")
            .and_then(|o| o.get("triaged"))
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        let flagged_count = triaged
            .iter()
            .filter(|t| {
                t.get("severity")
                    .and_then(|s| s.as_str())
                    .is_some_and(|s| s != "low")
            })
            .count();

        let mut top_flagged: Vec<serde_json::Value> = triaged
            .iter()
            .filter_map(|t| {
                let user_id = t.get("anomaly_id").and_then(|v| v.as_str())?;
                let severity = t.get("severity").and_then(|v| v.as_str())?;
                let composite = composites
                    .iter()
                    .find(|c| c.user_id == user_id)
                    .map(|c| c.composite)
                    .unwrap_or(0.0);
                Some(serde_json::json!({
                    "user_id": user_id,
                    "severity": severity,
                    "composite_score": composite,
                    "escalate": t.get("escalate").and_then(|v| v.as_bool()).unwrap_or(false),
                    "reason": t.get("reason").and_then(|v| v.as_str()).unwrap_or(""),
                }))
            })
            .collect();

        // Sort by composite score descending.
        top_flagged.sort_by(|a, b| {
            let sa = a
                .get("composite_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let sb = b
                .get("composite_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let recommendation = serde_json::json!({
            "flagged_count": flagged_count,
            "total_triaged": triaged.len(),
            "top_flagged": top_flagged,
            "gate_decision": triage.get("gate_decision"),
            "rationale": format!(
                "{} users flagged (critical+high+medium) out of {} triaged",
                flagged_count,
                triaged.len()
            ),
        });

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "recommendation",
            recommendation.to_string(),
        ))
    }
}
