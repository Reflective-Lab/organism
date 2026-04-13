// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Consensus agents for Spike 3.

use std::collections::BTreeMap;
use std::sync::Arc;

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};
use converge_provider::provider_api::{LlmProvider, LlmRequest};
use organism_core::planning::{
    CostEstimate, Impact, Likelihood, PlanAnnotation, ReasoningSystem, Risk, RiskImpact,
};

use crate::spike_capacity::optimization::{
    FeasiblePlanResult, SimulationRun, run_parallel_simulations, solve_candidate_plan,
};
use crate::spike_capacity::scenario::{SpikeConfig, load_capacity_bundle};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanAdjustment {
    pub team_id: String,
    pub added_capacity: f64,
    pub added_headcount: i32,
    pub extra_skills: Vec<String>,
}

impl PlanAdjustment {
    pub fn new(
        team_id: impl Into<String>,
        added_capacity: f64,
        added_headcount: i32,
        extra_skills: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            team_id: team_id.into(),
            added_capacity,
            added_headcount,
            extra_skills: extra_skills.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CandidatePlan {
    pub plan_id: String,
    pub name: String,
    pub strategic_thesis: String,
    pub adjustments: Vec<PlanAdjustment>,
    pub annotation: PlanAnnotation,
    pub incremental_spend: f64,
    pub planning_focus: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanVote {
    pub plan_id: String,
    pub voter: String,
    pub score: f64,
    pub rationale: String,
}

pub struct PreplanningHuddleAgent {
    llm: Option<Arc<dyn LlmProvider>>,
}

impl PreplanningHuddleAgent {
    pub fn local() -> Self {
        Self { llm: None }
    }

    pub fn live(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm: Some(llm) }
    }
}

impl Agent for PreplanningHuddleAgent {
    fn name(&self) -> &str {
        "PreplanningHuddleAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_all = [
            "analysis:demand_research",
            "analysis:delivery_deep_research",
            "analysis:workforce_research",
        ]
        .iter()
        .all(|id| ctx.get(ContextKey::Hypotheses).iter().any(|f| f.id == *id));
        let has_huddle = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .any(|f| f.id == "preplanning_huddle");
        has_all && !has_huddle
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let content = if let Some(llm) = &self.llm {
            let prompt = ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .map(|fact| format!("{}: {}", fact.id, fact.content))
                .collect::<Vec<_>>()
                .join("\n\n");
            let system = "You are running a preplanning huddle before optimization. \
                Return raw JSON only with keys voices, tensions, and decision_frame. \
                Voices should be 3 short named perspectives with what each cares about.";
            let request = LlmRequest::new(prompt)
                .with_system(system)
                .with_max_tokens(600)
                .with_temperature(0.2);
            match llm.complete(&request) {
                Ok(response) => normalize_huddle_output(&response.content),
                Err(_) => default_huddle_content(),
            }
        } else {
            default_huddle_content()
        };

        AgentEffect::with_fact(Fact::new(
            ContextKey::Hypotheses,
            "preplanning_huddle",
            content,
        ))
    }
}

pub struct HuddleSynthesisAgent;

impl Agent for HuddleSynthesisAgent {
    fn name(&self) -> &str {
        "HuddleSynthesisAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_all = [
            "analysis:demand_research",
            "analysis:delivery_deep_research",
            "analysis:workforce_research",
        ]
        .iter()
        .all(|id| ctx.get(ContextKey::Hypotheses).iter().any(|f| f.id == *id));
        let has_plans = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "candidate_plans");
        has_all && !has_plans
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let plans = candidate_plans();
        AgentEffect::with_fact(Fact::new(
            ContextKey::Strategies,
            "candidate_plans",
            serde_json::to_string(&plans).unwrap_or_default(),
        ))
    }
}

pub struct VotingAgent;

impl Agent for VotingAgent {
    fn name(&self) -> &str {
        "VotingAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_plans = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "candidate_plans");
        let has_votes = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "plan_votes");
        has_plans && !has_votes
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let plans = parse_plans(ctx);
        let mut votes = Vec::new();

        for plan in &plans {
            let capacity_added: f64 = plan.adjustments.iter().map(|a| a.added_capacity).sum();
            let spend_penalty = plan.incremental_spend / 100_000.0;
            let finance = 8.5 - spend_penalty;
            let delivery =
                6.0 + skill_bonus(plan, &["backend", "qa", "platform"]) + capacity_added / 150.0;
            let strategy = 6.5 + skill_bonus(plan, &["data", "ml"]) + capacity_added / 220.0;

            votes.push(PlanVote {
                plan_id: plan.plan_id.clone(),
                voter: "finance_owner".into(),
                score: round(finance),
                rationale: format!("Finance discounts {}", plan.incremental_spend),
            });
            votes.push(PlanVote {
                plan_id: plan.plan_id.clone(),
                voter: "delivery_lead".into(),
                score: round(delivery),
                rationale: "Delivery prefers plans that reduce backend/platform bottlenecks".into(),
            });
            votes.push(PlanVote {
                plan_id: plan.plan_id.clone(),
                voter: "strategy_lead".into(),
                score: round(strategy),
                rationale: "Strategy rewards future-facing data and ML options".into(),
            });
        }

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "plan_votes",
            serde_json::to_string(&votes).unwrap_or_default(),
        ))
    }
}

pub struct AnalyticsScoringAgent;

impl Agent for AnalyticsScoringAgent {
    fn name(&self) -> &str {
        "AnalyticsScoringAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_plans = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "candidate_plans");
        let has_scores = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "analytics_scores");
        has_plans && !has_scores
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let plans = parse_plans(ctx);
        let bundle = match load_capacity_bundle() {
            Ok(bundle) => bundle,
            Err(err) => {
                return AgentEffect::with_fact(Fact::new(
                    ContextKey::Diagnostic,
                    "analytics-load-error",
                    err,
                ));
            }
        };

        let baseline_lateness = bundle
            .history
            .records
            .iter()
            .map(|r| r.lateness_rate)
            .sum::<f64>()
            / bundle.history.records.len() as f64;
        let forecast_demand = bundle
            .demand
            .records
            .iter()
            .map(|r| r.demand_units)
            .sum::<f64>();

        let evaluations: Vec<_> = plans
            .iter()
            .map(|plan| {
                let capacity_added: f64 = plan.adjustments.iter().map(|a| a.added_capacity).sum();
                let throughput = forecast_demand.min(640.0 + capacity_added * 1.2);
                let risk_reduction = skill_bonus(plan, &["backend", "qa", "platform"]) * 0.025
                    + skill_bonus(plan, &["data", "ml"]) * 0.015;
                let lateness_risk = (baseline_lateness - risk_reduction).clamp(0.05, 0.3);
                serde_json::json!({
                    "plan_id": plan.plan_id,
                    "expected_throughput": round(throughput),
                    "lateness_risk": round(lateness_risk),
                    "confidence_interval": [round(throughput * 0.92), round(throughput * 1.08)],
                    "dataset_version": bundle.history.dataset_version,
                })
            })
            .collect();

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "analytics_scores",
            serde_json::to_string(&evaluations).unwrap_or_default(),
        ))
    }
}

pub struct SimulationAgent {
    config: SpikeConfig,
    simulator: Option<Arc<dyn LlmProvider>>,
}

impl SimulationAgent {
    pub fn new(config: SpikeConfig) -> Self {
        Self {
            config,
            simulator: None,
        }
    }

    pub fn live(config: SpikeConfig, simulator: Arc<dyn LlmProvider>) -> Self {
        Self {
            config,
            simulator: Some(simulator),
        }
    }
}

impl Agent for SimulationAgent {
    fn name(&self) -> &str {
        "SimulationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_plans = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "candidate_plans");
        let has_analytics = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "analytics_scores");
        let has_runs = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "simulation_runs");
        has_plans && has_analytics && !has_runs
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let plans = parse_plans(ctx);
        match run_parallel_simulations(&plans, &self.config) {
            Ok(runs) => {
                let mut facts = vec![Fact::new(
                    ContextKey::Evaluations,
                    "simulation_runs",
                    serde_json::to_string(&runs).unwrap_or_default(),
                )];

                if let Some(simulator) = &self.simulator {
                    let prompt = serde_json::to_string(&runs).unwrap_or_default();
                    let system = "You are reviewing planning simulation branches. \
                        Summarize which approaches remain plausible under stress and why. \
                        Return 2-3 concise sentences.";
                    let request = LlmRequest::new(prompt)
                        .with_system(system)
                        .with_max_tokens(220)
                        .with_temperature(0.2);
                    if let Ok(response) = simulator.complete(&request) {
                        facts.push(Fact::new(
                            ContextKey::Evaluations,
                            "simulation_summary",
                            serde_json::json!({
                                "summary": response.content,
                                "provider": simulator.name(),
                                "model": simulator.model(),
                            })
                            .to_string(),
                        ));
                    }
                }

                AgentEffect::with_facts(facts)
            }
            Err(err) => {
                AgentEffect::with_fact(Fact::new(ContextKey::Diagnostic, "simulation_error", err))
            }
        }
    }
}

pub struct CapacityOptimizationAgent {
    config: SpikeConfig,
}

impl CapacityOptimizationAgent {
    pub fn new(config: SpikeConfig) -> Self {
        Self { config }
    }
}

impl Agent for CapacityOptimizationAgent {
    fn name(&self) -> &str {
        "CapacityOptimizationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_plans = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "candidate_plans");
        let has_analytics = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "analytics_scores");
        let has_feasible = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "feasible_plans");
        has_plans && has_analytics && !has_feasible
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let plans = parse_plans(ctx);
        let mut feasible = Vec::<FeasiblePlanResult>::new();

        for plan in plans {
            match solve_candidate_plan(&plan, &self.config) {
                Ok(result) => feasible.push(result),
                Err(err) => {
                    return AgentEffect::with_fact(Fact::new(
                        ContextKey::Diagnostic,
                        format!("optimization-error:{}", plan.plan_id),
                        err,
                    ));
                }
            }
        }

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "feasible_plans",
            serde_json::to_string(&feasible).unwrap_or_default(),
        ))
    }
}

pub struct RecommendationAgent;

impl Agent for RecommendationAgent {
    fn name(&self) -> &str {
        "RecommendationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_feasible = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "feasible_plans");
        let has_recommendation = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "recommendation");
        has_feasible && !has_recommendation
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let feasible_fact = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "feasible_plans")
        {
            Some(fact) => fact,
            None => return AgentEffect::empty(),
        };
        let analytics_fact = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "analytics_scores")
        {
            Some(fact) => fact,
            None => return AgentEffect::empty(),
        };
        let vote_fact = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "plan_votes")
        {
            Some(fact) => fact,
            None => return AgentEffect::empty(),
        };
        let simulation_fact = match ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "simulation_runs")
        {
            Some(fact) => fact,
            None => return AgentEffect::empty(),
        };

        let feasible: Vec<FeasiblePlanResult> =
            serde_json::from_str(&feasible_fact.content).unwrap_or_default();
        let analytics: Vec<serde_json::Value> =
            serde_json::from_str(&analytics_fact.content).unwrap_or_default();
        let votes: Vec<PlanVote> = serde_json::from_str(&vote_fact.content).unwrap_or_default();
        let simulations: Vec<SimulationRun> =
            serde_json::from_str(&simulation_fact.content).unwrap_or_default();

        let mut vote_totals = BTreeMap::<String, f64>::new();
        for vote in votes {
            *vote_totals.entry(vote.plan_id).or_default() += vote.score;
        }

        let best = feasible
            .iter()
            .filter(|plan| plan.gate_decision != "reject")
            .max_by(|a, b| {
                final_score(a, &analytics, &vote_totals, &simulations)
                    .partial_cmp(&final_score(b, &analytics, &vote_totals, &simulations))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let best = match best {
            Some(best) => best,
            None => return AgentEffect::empty(),
        };

        let score = final_score(best, &analytics, &vote_totals, &simulations);
        let plausible_scenarios = simulations
            .iter()
            .filter(|run| run.plan_id == best.plan_id && run.plausible)
            .count();
        let content = serde_json::json!({
            "selected_plan_id": best.plan_id,
            "recommendation": format!("Commit to {}", best.plan_id.replace('_', " ")),
            "overall_score": round(score),
            "rationale": format!(
                "This plan achieved {:.0}% fulfillment with gate '{}' while maintaining the strongest combined analytics, huddle vote support, and simulation plausibility across {} scenarios.",
                best.overall_fulfillment_ratio * 100.0,
                best.gate_decision,
                plausible_scenarios
            ),
            "evidence": {
                "analytics_dataset_version": best.analytics_dataset_version,
                "capacity_dataset_version": best.capacity_dataset_version,
                "plausible_scenarios": plausible_scenarios
            }
        })
        .to_string();

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "recommendation",
            content,
        ))
    }
}

pub fn candidate_plans() -> Vec<CandidatePlan> {
    vec![
        CandidatePlan {
            plan_id: "stabilize_delivery".into(),
            name: "Stabilize Delivery".into(),
            strategic_thesis: "Reinforce current bottlenecks first, then expand".into(),
            adjustments: vec![
                PlanAdjustment::new("backend_platform", 110.0, 3, Vec::<String>::new()),
                PlanAdjustment::new("experience_qa", 40.0, 1, Vec::<String>::new()),
                PlanAdjustment::new("shared_services", 20.0, 1, Vec::<String>::new()),
            ],
            annotation: common_annotation(
                "Prioritize backend/platform stability and QA containment.",
                0.78,
                vec![
                    ReasoningSystem::ConstraintSolver,
                    ReasoningSystem::DomainModel,
                    ReasoningSystem::CostEstimation,
                ],
            ),
            incremental_spend: 240_000.0,
            planning_focus: vec!["backend".into(), "platform".into(), "qa".into()],
        },
        CandidatePlan {
            plan_id: "balanced_growth".into(),
            name: "Balanced Growth".into(),
            strategic_thesis:
                "Address current bottlenecks while expanding data execution capacity.".into(),
            adjustments: vec![
                PlanAdjustment::new("backend_platform", 70.0, 2, Vec::<String>::new()),
                PlanAdjustment::new("data_ml", 90.0, 3, Vec::<String>::new()),
                PlanAdjustment::new("experience_qa", 30.0, 1, Vec::<String>::new()),
            ],
            annotation: common_annotation(
                "Balance execution resilience with demand capture in data and ML.",
                0.84,
                vec![
                    ReasoningSystem::ConstraintSolver,
                    ReasoningSystem::MlPrediction,
                    ReasoningSystem::DomainModel,
                    ReasoningSystem::CostEstimation,
                ],
            ),
            incremental_spend: 280_000.0,
            planning_focus: vec!["backend".into(), "data".into(), "ml".into()],
        },
        CandidatePlan {
            plan_id: "accelerate_intelligence".into(),
            name: "Accelerate Intelligence".into(),
            strategic_thesis: "Bias the quarter toward data and ML acceleration.".into(),
            adjustments: vec![
                PlanAdjustment::new("data_ml", 150.0, 4, Vec::<String>::new()),
                PlanAdjustment::new("backend_platform", 40.0, 1, Vec::<String>::new()),
                PlanAdjustment::new("shared_services", 10.0, 0, ["data"]),
            ],
            annotation: common_annotation(
                "Push harder into intelligence work, accepting more delivery concentration risk.",
                0.74,
                vec![
                    ReasoningSystem::MlPrediction,
                    ReasoningSystem::ConstraintSolver,
                    ReasoningSystem::LlmReasoning,
                ],
            ),
            incremental_spend: 310_000.0,
            planning_focus: vec!["data".into(), "ml".into()],
        },
    ]
}

fn common_annotation(
    description: &str,
    confidence: f64,
    contributors: Vec<ReasoningSystem>,
) -> PlanAnnotation {
    PlanAnnotation {
        description: description.to_string(),
        expected_impact: Impact {
            description: "Increase feasible fulfillment while keeping delivery stable".into(),
            confidence,
        },
        cost_estimate: CostEstimate {
            compute_cost: 0.0,
            time_cost: Some("one quarter".into()),
            unit: "USD".into(),
        },
        risks: vec![Risk {
            description: "Capacity shifts may still leave skill-local bottlenecks".into(),
            likelihood: Likelihood::Medium,
            impact: RiskImpact::Medium,
            mitigation: Some("Use solver-backed assignment before commitment".into()),
        }],
        contributors,
    }
}

fn parse_plans(ctx: &Context) -> Vec<CandidatePlan> {
    ctx.get(ContextKey::Strategies)
        .iter()
        .find(|f| f.id == "candidate_plans")
        .and_then(|f| serde_json::from_str::<Vec<CandidatePlan>>(&f.content).ok())
        .unwrap_or_default()
}

fn skill_bonus(plan: &CandidatePlan, preferred: &[&str]) -> f64 {
    plan.planning_focus
        .iter()
        .filter(|skill| preferred.contains(&skill.as_str()))
        .count() as f64
}

fn final_score(
    feasible: &FeasiblePlanResult,
    analytics: &[serde_json::Value],
    votes: &BTreeMap<String, f64>,
    simulations: &[SimulationRun],
) -> f64 {
    let analytics_score = analytics
        .iter()
        .find(|entry| {
            entry
                .get("plan_id")
                .and_then(|v| v.as_str())
                .is_some_and(|id| id == feasible.plan_id)
        })
        .map(|entry| {
            let throughput = entry
                .get("expected_throughput")
                .and_then(|v| v.as_f64())
                .unwrap_or_default();
            let risk = entry
                .get("lateness_risk")
                .and_then(|v| v.as_f64())
                .unwrap_or_default();
            throughput / 100.0 - risk * 10.0
        })
        .unwrap_or_default();

    let plausibility = simulations
        .iter()
        .filter(|run| run.plan_id == feasible.plan_id)
        .fold((0usize, 0usize), |(total, plausible), run| {
            (total + 1, plausible + usize::from(run.plausible))
        });
    let simulation_score = if plausibility.0 > 0 {
        5.0 * plausibility.1 as f64 / plausibility.0 as f64
    } else {
        0.0
    };

    feasible.overall_fulfillment_ratio * 10.0
        + analytics_score
        + votes.get(&feasible.plan_id).copied().unwrap_or_default() / 3.0
        + simulation_score
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn default_huddle_content() -> String {
    serde_json::json!({
        "voices": [
            {"name": "delivery_lead", "focus": "protect backend and QA throughput"},
            {"name": "finance_owner", "focus": "avoid overcommitting fixed spend"},
            {"name": "strategy_lead", "focus": "expand data and ML capacity where future demand is rising"}
        ],
        "tensions": [
            "stability vs growth",
            "delivery coverage vs future capability building"
        ],
        "decision_frame": "Prefer plans that remain plausible under simulation and survive solver review."
    })
    .to_string()
}

fn normalize_huddle_output(content: &str) -> String {
    let trimmed = content.trim();
    let cleaned = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        trimmed.to_string()
    };

    if serde_json::from_str::<serde_json::Value>(&cleaned).is_ok() {
        cleaned
    } else {
        default_huddle_content()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huddle_produces_three_plans() {
        assert_eq!(candidate_plans().len(), 3);
    }
}
