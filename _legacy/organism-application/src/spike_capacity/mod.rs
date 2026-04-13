// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Spike 3: Organism capacity planning with richer converge module usage.
//!
//! This spike keeps the vertical-slice shape from Spike 2, but moves the
//! decision problem to a harder planning domain:
//! local datasets -> research analyses -> huddle candidate plans -> voting ->
//! analytics scoring -> converge-optimization capacity-planning solve.

pub mod agents;
pub mod consensus;
pub mod invariants;
pub mod optimization;
pub mod scenario;

use converge_core::{Context, ContextKey};
use converge_provider::ProviderRegistry;

use scenario::{
    ExperimentTopic, SpikeConfig, build_consensus_engine, build_consensus_engine_with_providers,
    build_experiment_engine, build_experiment_engine_with_providers, discover_providers,
    load_capacity_bundle, test_capacity_intent,
};

/// Result of running Spike 3.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CapacityPlanningResult {
    pub recommended_plan_id: String,
    pub total_cycles: u32,
    pub context: Context,
}

/// Error from running the spike.
#[derive(Debug)]
pub enum CapacityPlanningError {
    Dataset(String),
    Provider(String),
    ExperimentFailed { topic: String, reason: String },
    ConsensusFailed(String),
    MissingRecommendation,
}

impl std::fmt::Display for CapacityPlanningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dataset(reason) => write!(f, "dataset error: {reason}"),
            Self::Provider(reason) => write!(f, "provider error: {reason}"),
            Self::ExperimentFailed { topic, reason } => {
                write!(f, "experiment '{topic}' failed: {reason}")
            }
            Self::ConsensusFailed(reason) => write!(f, "consensus failed: {reason}"),
            Self::MissingRecommendation => write!(f, "recommendation was not produced"),
        }
    }
}

impl std::error::Error for CapacityPlanningError {}

/// Run Spike 3 with verbose output.
pub fn run_capacity_planning_verbose() -> Result<CapacityPlanningResult, CapacityPlanningError> {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Spike 3: Organism Capacity Planning Convergence                ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    let bundle = load_capacity_bundle().map_err(CapacityPlanningError::Dataset)?;
    let intent = test_capacity_intent();
    let config = SpikeConfig::default();
    let registry = ProviderRegistry::from_env();
    let live_providers = discover_providers(&registry).ok();

    println!(
        "▸ Intent: {}",
        intent
            .root_intent()
            .objective
            .as_ref()
            .map_or("(none)", |o| match o {
                converge_core::Objective::Custom(s) => s.as_str(),
                _ => "(default)",
            })
    );
    println!(
        "▸ Datasets: forecast={}, history={}, capacity={}",
        bundle.demand.dataset_version,
        bundle.history.dataset_version,
        bundle.capacity.dataset_version
    );

    // ─── Dataset Overview ───
    println!("\n┌─ Dataset Overview ─────────────────────────────────────────────┐");
    let total_demand: f64 = bundle.demand.records.iter().map(|r| r.demand_units).sum();
    let total_capacity: f64 = bundle.capacity.records.iter().map(|r| r.available_capacity).sum();
    println!(
        "│  Demand forecast: {} records, {:.0} total units across {} periods",
        bundle.demand.records.len(),
        total_demand,
        bundle
            .demand
            .records
            .iter()
            .map(|r| r.period_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    println!(
        "│  Delivery history: {} skill records (avg lateness {:.0}%)",
        bundle.history.records.len(),
        bundle.history.records.iter().map(|r| r.lateness_rate).sum::<f64>()
            / bundle.history.records.len() as f64
            * 100.0
    );
    println!(
        "│  Team capacity: {} teams, {:.0} total units, {} headcount",
        bundle.capacity.records.len(),
        total_capacity,
        bundle.capacity.records.iter().map(|r| r.headcount).sum::<i32>()
    );
    let gap = total_demand - total_capacity;
    println!(
        "│  Demand-capacity gap: {:.0} units ({:.0}% coverage baseline)",
        gap,
        (total_capacity / total_demand) * 100.0
    );
    println!("└────────────────────────────────────────────────────────────────┘");

    if let Some(providers) = &live_providers {
        println!("\n▸ Live providers:");
        println!(
            "  Planner:   {}/{}",
            providers.planner_selection.selected.provider,
            providers.planner_selection.selected.model
        );
        println!(
            "  Analyst:   {}/{}",
            providers.analyst_selection.selected.provider,
            providers.analyst_selection.selected.model
        );
        println!(
            "  Huddle:    {}/{}",
            providers.huddle_selection.selected.provider, providers.huddle_selection.selected.model
        );
        println!(
            "  Simulator: {}/{}",
            providers.simulator_selection.selected.provider,
            providers.simulator_selection.selected.model
        );
        println!("  Search:    Brave Web Search");
    } else {
        println!("\n▸ Mode: local deterministic datasets and simulations");
    }

    // ─── Phase 1: Parallel Research Experiments ───
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Phase 1: Parallel Research Experiments                         ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let topics = [
        ExperimentTopic::DemandResearch,
        ExperimentTopic::DeliveryDeepResearch,
        ExperimentTopic::WorkforceResearch,
    ];

    let mut experiment_contexts = Vec::new();
    let mut total_cycles = 0u32;

    for topic in topics {
        println!("\n  ┌─ Experiment: {} ─", topic.name());
        println!("  │  Research question: {}", topic.research_question());
        let mut engine = if let Some(providers) = &live_providers {
            println!("  │  Mode: hybrid (local dataset + web search + LLM synthesis)");
            build_experiment_engine_with_providers(topic, providers)
        } else {
            println!("  │  Mode: local dataset analysis");
            build_experiment_engine(topic)
        };
        let result =
            engine
                .run(Context::new())
                .map_err(|e| CapacityPlanningError::ExperimentFailed {
                    topic: topic.name().to_string(),
                    reason: e.to_string(),
                })?;
        total_cycles += result.cycles;
        println!("  │  ✓ Converged in {} cycles", result.cycles);

        for fact in result.context.get(ContextKey::Hypotheses) {
            if fact.id == format!("analysis:{}", topic.name()) {
                println!("  │  Summary: {}", summary_snippet(&fact.content));
                print_analysis_detail(&fact.content, topic);
            }
        }
        println!("  └─");

        experiment_contexts.push(result.context);
    }

    // ─── Phase 2: Consensus Pipeline ───
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Phase 2: Planning Consensus Pipeline                           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut consensus_ctx = Context::new();
    for exp_ctx in &experiment_contexts {
        for key in [ContextKey::Signals, ContextKey::Hypotheses] {
            for fact in exp_ctx.get(key) {
                let _ = consensus_ctx.add_fact(fact.clone());
            }
        }
    }

    let mut consensus_engine = if let Some(providers) = &live_providers {
        build_consensus_engine_with_providers(&config, providers)
    } else {
        build_consensus_engine()
    };
    let result = consensus_engine
        .run(consensus_ctx)
        .map_err(|e| CapacityPlanningError::ConsensusFailed(e.to_string()))?;
    total_cycles += result.cycles;
    println!("  ✓ Consensus converged in {} cycles\n", result.cycles);

    print_consensus_detail(&result.context, &config);

    let recommendation = result
        .context
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "recommendation")
        .ok_or(CapacityPlanningError::MissingRecommendation)?;

    let parsed: serde_json::Value =
        serde_json::from_str(&recommendation.content).unwrap_or_default();
    let recommended_plan_id = parsed
        .get("selected_plan_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // ─── Final Summary ───
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Final Result                                                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!(
        "  Selected plan: {}",
        recommended_plan_id.replace('_', " ")
    );
    println!(
        "  Overall score: {}",
        parsed
            .get("overall_score")
            .and_then(|v| v.as_f64())
            .map_or("?".to_string(), |s| format!("{s:.2}"))
    );
    println!(
        "  Rationale: {}",
        parsed
            .get("rationale")
            .and_then(|v| v.as_str())
            .unwrap_or("(none)")
    );
    println!("  Total convergence cycles: {total_cycles}");
    println!(
        "  Invariants: research_coverage ✓  dataset_provenance ✓  feasible_recommendation ✓"
    );

    Ok(CapacityPlanningResult {
        recommended_plan_id,
        total_cycles,
        context: result.context,
    })
}

fn summary_snippet(content: &str) -> String {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("summary")
                .and_then(|s| s.as_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| content.chars().take(90).collect())
}

fn print_analysis_detail(content: &str, topic: scenario::ExperimentTopic) {
    let parsed: serde_json::Value = serde_json::from_str(content).unwrap_or_default();
    match topic {
        scenario::ExperimentTopic::DemandResearch => {
            if let Some(by_skill) = parsed.get("demand_by_skill").and_then(|v| v.as_object()) {
                print!("  │  Demand by skill: ");
                let parts: Vec<String> = by_skill
                    .iter()
                    .map(|(k, v)| format!("{}={:.0}", k, v.as_f64().unwrap_or_default()))
                    .collect();
                println!("{}", parts.join(", "));
            }
            if let Some(top) = parsed.get("top_skill").and_then(|v| v.as_str()) {
                println!("  │  Top pressure skill: {top}");
            }
        }
        scenario::ExperimentTopic::DeliveryDeepResearch => {
            if let Some(by_skill) = parsed.get("risk_by_skill").and_then(|v| v.as_object()) {
                print!("  │  Risk by skill (lateness+spillover): ");
                let parts: Vec<String> = by_skill
                    .iter()
                    .map(|(k, v)| format!("{}={:.0}%", k, v.as_f64().unwrap_or_default() * 100.0))
                    .collect();
                println!("{}", parts.join(", "));
            }
            if let Some(highest) = parsed.get("highest_risk_skill").and_then(|v| v.as_str()) {
                println!("  │  Highest risk skill: {highest}");
            }
        }
        scenario::ExperimentTopic::WorkforceResearch => {
            if let Some(by_skill) = parsed.get("capacity_by_skill").and_then(|v| v.as_object()) {
                print!("  │  Capacity by skill: ");
                let parts: Vec<String> = by_skill
                    .iter()
                    .map(|(k, v)| format!("{}={:.0}", k, v.as_f64().unwrap_or_default()))
                    .collect();
                println!("{}", parts.join(", "));
            }
            if let Some(tightest) = parsed.get("tightest_skill").and_then(|v| v.as_str()) {
                println!("  │  Structurally thinnest: {tightest}");
            }
        }
    }
}

fn print_consensus_detail(ctx: &Context, config: &scenario::SpikeConfig) {
    // ─── Huddle ───
    if let Some(huddle_fact) = ctx
        .get(ContextKey::Hypotheses)
        .iter()
        .find(|f| f.id == "preplanning_huddle")
    {
        let parsed: serde_json::Value =
            serde_json::from_str(&huddle_fact.content).unwrap_or_default();
        println!("  ┌─ Preplanning Huddle ─────────────────────────────────────────┐");
        if let Some(voices) = parsed.get("voices").and_then(|v| v.as_array()) {
            println!("  │  Voices ({}):", voices.len());
            for voice in voices {
                let name = voice.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let focus = voice.get("focus").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  │    {:<18} → {focus}", name);
            }
        }
        if let Some(tensions) = parsed.get("tensions").and_then(|v| v.as_array()) {
            println!("  │  Tensions ({}):", tensions.len());
            for tension in tensions {
                if let Some(t) = tension.as_str() {
                    println!("  │    • {t}");
                }
            }
        }
        if let Some(frame) = parsed.get("decision_frame").and_then(|v| v.as_str()) {
            println!("  │  Decision frame: {frame}");
        }
        println!("  └────────────────────────────────────────────────────────────────┘");
    }

    // ─── Candidate Plans ───
    if let Some(plan_fact) = ctx
        .get(ContextKey::Strategies)
        .iter()
        .find(|f| f.id == "candidate_plans")
    {
        if let Ok(plans) =
            serde_json::from_str::<Vec<consensus::CandidatePlan>>(&plan_fact.content)
        {
            println!("\n  ┌─ Candidate Plans ({}) ─────────────────────────────────────┐", plans.len());
            for plan in &plans {
                let capacity_added: f64 = plan.adjustments.iter().map(|a| a.added_capacity).sum();
                println!("  │");
                println!(
                    "  │  {} [{}]",
                    plan.name, plan.plan_id
                );
                println!("  │    Thesis: {}", plan.strategic_thesis);
                println!(
                    "  │    Spend: ${:.0}k   Capacity added: {:.0} units",
                    plan.incremental_spend / 1000.0,
                    capacity_added
                );
                println!("  │    Focus: {}", plan.planning_focus.join(", "));
                print!("  │    Adjustments: ");
                let adj_parts: Vec<String> = plan
                    .adjustments
                    .iter()
                    .map(|a| format!("{} +{:.0}", a.team_id, a.added_capacity))
                    .collect();
                println!("{}", adj_parts.join(", "));
                let contributors: Vec<&str> = plan
                    .annotation
                    .contributors
                    .iter()
                    .map(|c| match c {
                        organism_core::planning::ReasoningSystem::LlmReasoning => "LLM",
                        organism_core::planning::ReasoningSystem::ConstraintSolver => "Solver",
                        organism_core::planning::ReasoningSystem::MlPrediction => "ML",
                        organism_core::planning::ReasoningSystem::CausalAnalysis => "Causal",
                        organism_core::planning::ReasoningSystem::CostEstimation => "Cost",
                        organism_core::planning::ReasoningSystem::DomainModel => "Domain",
                    })
                    .collect();
                println!("  │    Reasoning: {}", contributors.join(" + "));
            }
            println!("  └────────────────────────────────────────────────────────────────┘");
        }
    }

    // ─── Voting ───
    if let Some(vote_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "plan_votes")
    {
        if let Ok(votes) =
            serde_json::from_str::<Vec<consensus::PlanVote>>(&vote_fact.content)
        {
            println!("\n  ┌─ Multi-Stakeholder Voting ({} votes) ──────────────────────┐", votes.len());
            println!(
                "  │  {:<24} {:>10} {:>10} {:>10}",
                "Plan", "Finance", "Delivery", "Strategy"
            );
            println!("  │  {}", "─".repeat(58));

            let plan_ids: Vec<String> = votes
                .iter()
                .map(|v| v.plan_id.clone())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();

            for plan_id in &plan_ids {
                let finance = votes
                    .iter()
                    .find(|v| v.plan_id == *plan_id && v.voter == "finance_owner")
                    .map_or(0.0, |v| v.score);
                let delivery = votes
                    .iter()
                    .find(|v| v.plan_id == *plan_id && v.voter == "delivery_lead")
                    .map_or(0.0, |v| v.score);
                let strategy = votes
                    .iter()
                    .find(|v| v.plan_id == *plan_id && v.voter == "strategy_lead")
                    .map_or(0.0, |v| v.score);
                let total = finance + delivery + strategy;
                println!(
                    "  │  {:<24} {:>9.2} {:>9.2} {:>9.2}  Σ={:.2}",
                    plan_id, finance, delivery, strategy, total
                );
            }
            println!("  └────────────────────────────────────────────────────────────────┘");
        }
    }

    // ─── Analytics Scoring ───
    if let Some(analytics_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "analytics_scores")
    {
        if let Ok(scores) =
            serde_json::from_str::<Vec<serde_json::Value>>(&analytics_fact.content)
        {
            println!("\n  ┌─ Analytics Scoring (converge-analytics) ─────────────────────┐");
            println!(
                "  │  {:<24} {:>12} {:>12} {:>18}",
                "Plan", "Throughput", "Lateness", "Confidence"
            );
            println!("  │  {}", "─".repeat(62));
            for score in &scores {
                let plan_id = score
                    .get("plan_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let throughput = score
                    .get("expected_throughput")
                    .and_then(|v| v.as_f64())
                    .unwrap_or_default();
                let lateness = score
                    .get("lateness_risk")
                    .and_then(|v| v.as_f64())
                    .unwrap_or_default();
                let interval = score
                    .get("confidence_interval")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        format!(
                            "[{:.0}, {:.0}]",
                            arr.first()
                                .and_then(|v| v.as_f64())
                                .unwrap_or_default(),
                            arr.get(1).and_then(|v| v.as_f64()).unwrap_or_default()
                        )
                    })
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "  │  {:<24} {:>11.0} {:>11.0}% {:>18}",
                    plan_id,
                    throughput,
                    lateness * 100.0,
                    interval
                );
            }
            if let Some(first) = scores.first() {
                if let Some(version) = first.get("dataset_version").and_then(|v| v.as_str()) {
                    println!("  │  Dataset: {version}");
                }
            }
            println!("  └────────────────────────────────────────────────────────────────┘");
        }
    }

    // ─── Simulation Matrix ───
    if let Some(sim_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "simulation_runs")
    {
        if let Ok(runs) = serde_json::from_str::<Vec<serde_json::Value>>(&sim_fact.content) {
            let plausible_count = runs
                .iter()
                .filter(|run| {
                    run.get("plausible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .count();

            println!(
                "\n  ┌─ Parallel Stress Simulations ({} runs, {} plausible) ─────────┐",
                runs.len(),
                plausible_count
            );
            println!(
                "  │  {:<24} {:<18} {:>10} {:>6} {:>9}",
                "Plan", "Scenario", "Fulfill%", "Gate", "Plaus?"
            );
            println!("  │  {}", "─".repeat(66));

            let scenarios = ["base_case", "demand_spike", "attrition_shock", "execution_recovery"];
            let plan_ids: Vec<String> = runs
                .iter()
                .filter_map(|r| r.get("plan_id").and_then(|v| v.as_str()).map(String::from))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();

            for plan_id in &plan_ids {
                for scenario in &scenarios {
                    if let Some(run) = runs.iter().find(|r| {
                        r.get("plan_id")
                            .and_then(|v| v.as_str())
                            .is_some_and(|id| id == plan_id)
                            && r.get("scenario")
                                .and_then(|v| v.as_str())
                                .is_some_and(|s| s == *scenario)
                    }) {
                        let fulfillment = run
                            .get("overall_fulfillment_ratio")
                            .and_then(|v| v.as_f64())
                            .unwrap_or_default();
                        let gate = run
                            .get("gate_decision")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let plausible = run
                            .get("plausible")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        println!(
                            "  │  {:<24} {:<18} {:>9.0}% {:>6} {:>9}",
                            plan_id,
                            scenario,
                            fulfillment * 100.0,
                            gate,
                            if plausible { "✓" } else { "✗" }
                        );
                    }
                }
            }
            println!(
                "  │  Min fulfillment target: {:.0}% (plausibility floor: {:.0}%)",
                config.min_overall_fulfillment * 100.0,
                config.min_overall_fulfillment * 0.95 * 100.0
            );
            println!("  └────────────────────────────────────────────────────────────────┘");
        }
    }

    // ─── Simulation Summary (LLM) ───
    if let Some(summary_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "simulation_summary")
    {
        let parsed: serde_json::Value =
            serde_json::from_str(&summary_fact.content).unwrap_or_default();
        if let Some(summary) = parsed.get("summary").and_then(|v| v.as_str()) {
            println!("\n  ▸ Simulation readout (LLM): {summary}");
        }
    }

    // ─── Solver / Feasible Plans ───
    if let Some(feasible_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "feasible_plans")
    {
        if let Ok(plans) = serde_json::from_str::<Vec<serde_json::Value>>(&feasible_fact.content) {
            println!(
                "\n  ┌─ CP-SAT Solver Results ({} plans) ─────────────────────────────┐",
                plans.len()
            );
            for plan in &plans {
                let id = plan.get("plan_id").and_then(|v| v.as_str()).unwrap_or("?");
                let fulfillment = plan
                    .get("overall_fulfillment_ratio")
                    .and_then(|v| v.as_f64())
                    .unwrap_or_default();
                let cost = plan
                    .get("total_cost")
                    .and_then(|v| v.as_f64())
                    .unwrap_or_default();
                let utilization = plan
                    .get("average_utilization")
                    .and_then(|v| v.as_f64())
                    .unwrap_or_default();
                let over_cap = plan
                    .get("teams_over_capacity")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_default();
                let unmet = plan
                    .get("unmet_demands")
                    .and_then(|v| v.as_u64())
                    .unwrap_or_default();
                let gate = plan
                    .get("gate_decision")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let rationale = plan
                    .get("gate_rationale")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                println!("  │");
                println!(
                    "  │  {} → gate: {}",
                    id,
                    match gate {
                        "promote" => "PROMOTE ✓",
                        "reject" => "REJECT ✗",
                        "escalate" => "ESCALATE ⚠",
                        other => other,
                    }
                );
                println!(
                    "  │    Fulfillment: {:.0}%  Cost: ${:.0}  Utilization: {:.0}%",
                    fulfillment * 100.0,
                    cost,
                    utilization * 100.0
                );
                println!(
                    "  │    Over-capacity teams: {}  Unmet demands: {}",
                    over_cap, unmet
                );
                if !rationale.is_empty() {
                    println!("  │    Rationale: {rationale}");
                }
            }
            println!("  └────────────────────────────────────────────────────────────────┘");
        }
    }

    // ─── Recommendation ───
    if let Some(rec_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "recommendation")
    {
        let parsed: serde_json::Value = serde_json::from_str(&rec_fact.content).unwrap_or_default();
        println!(
            "\n  ▸ Recommendation: {}",
            parsed
                .get("recommendation")
                .and_then(|v| v.as_str())
                .unwrap_or("(missing)")
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spike_3_runs_end_to_end() {
        let result = run_capacity_planning_verbose().expect("spike should converge");
        assert_eq!(result.recommended_plan_id, "balanced_growth");
        assert!(result.total_cycles >= 5);
    }
}
