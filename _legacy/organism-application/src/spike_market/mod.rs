// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Nordic Market Expansion Decision spike — exercises LLM agents, web search,
//! CP-SAT optimization, HITL gates, and multi-engine consensus.
//!
//! # Architecture
//!
//! ```text
//! OrganismIntent: "Select optimal European R&D location"
//!   │
//!   ├─ Experiment 1: Market Demand Analysis    (own Engine)
//!   ├─ Experiment 2: Competitive Landscape     (own Engine)
//!   ├─ Experiment 3: Go-to-Market Cost Model   (own Engine)
//!   │
//!   └─ Consensus Engine
//!        ├─ AggregationAgent     — merges 3 experiment results
//!        ├─ VotingAgent          — ranks cities by multi-criteria vote
//!        ├─ OptimizationAgent    — CP-SAT: select best city within constraints
//!        ├─ RecommendationAgent  — emits ProposedFact (confidence 0.72 → HITL pause)
//!        └─ HITL gate            — human approves/rejects/modifies recommendation
//! ```

pub mod agents;
pub mod consensus;
pub mod invariants;
pub mod optimization;
pub mod scenario;

use converge_core::gates::hitl::GateDecision;
use converge_core::{Context, ContextKey, Fact, RunResult};
use converge_provider::ProviderRegistry;

use scenario::{
    ExperimentTopic, build_consensus_engine, build_experiment_engine_from_registry,
    candidate_cities, discover_providers, test_market_intent,
};

/// Error from running the market expansion spike.
#[derive(Debug)]
pub enum MarketExpansionError {
    /// An experiment engine failed.
    ExperimentFailed { topic: String, reason: String },
    /// Consensus engine failed.
    ConsensusFailed(String),
    /// Provider configuration error.
    ProviderError(String),
}

impl std::fmt::Display for MarketExpansionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExperimentFailed { topic, reason } => {
                write!(f, "Experiment '{topic}' failed: {reason}")
            }
            Self::ConsensusFailed(reason) => write!(f, "Consensus failed: {reason}"),
            Self::ProviderError(reason) => write!(f, "Provider error: {reason}"),
        }
    }
}

impl std::error::Error for MarketExpansionError {}

/// Result of the market expansion spike.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketExpansionResult {
    /// The recommended city.
    pub recommended_city: String,
    /// Whether the HITL gate was triggered and approved.
    pub hitl_triggered: bool,
    /// Number of convergence cycles across all engines.
    pub total_cycles: u32,
    /// Final consensus context.
    pub context: Context,
}

/// Run the full market expansion spike with verbose output.
///
/// # Errors
///
/// Returns `MarketExpansionError` if any component fails.
pub fn run_market_expansion_verbose() -> Result<MarketExpansionResult, MarketExpansionError> {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Spike 2: Nordic Market Expansion Decision                      ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // ── Discover providers by capability ─────────────────────────────
    println!("▸ Discovering providers...");

    let registry = ProviderRegistry::from_env();
    let providers = discover_providers(&registry).map_err(MarketExpansionError::ProviderError)?;

    println!(
        "  Planner: {}/{} (fast/cheap)",
        providers.planner_selection.selected.provider, providers.planner_selection.selected.model
    );
    println!(
        "  Analyst: {}/{} (powerful)",
        providers.analyst_selection.selected.provider, providers.analyst_selection.selected.model
    );

    let intent = test_market_intent();
    println!(
        "  Intent: {}",
        intent
            .root_intent()
            .objective
            .as_ref()
            .map_or("(none)", |o| match o {
                converge_core::Objective::Custom(s) => s.as_str(),
                _ => "(default)",
            })
    );

    // ── Print candidate cities ──────────────────────────────────────
    println!("\n▸ Candidate Cities:");
    println!(
        "  {:<12} {:>8} {:>7} {:>7} {:>5} {:>8}",
        "City", "Cost(€K)", "Talent", "Market", "Tax", "Score"
    );
    println!("  {}", "─".repeat(55));
    for city in candidate_cities() {
        println!(
            "  {:<12} {:>8} {:>7} {:>7} {:>5} {:>8.1}",
            city.name,
            city.entry_cost_k,
            city.talent_score,
            city.market_access,
            city.tax_incentive,
            city.weighted_score()
        );
    }

    // ── Run 3 experiments ───────────────────────────────────────────
    let topics = [
        ExperimentTopic::MarketDemand,
        ExperimentTopic::CompetitiveLandscape,
        ExperimentTopic::GoToMarketCost,
    ];

    let mut experiment_contexts = Vec::new();
    let mut total_cycles = 0u32;

    for topic in &topics {
        println!("\n▸ Experiment: {} ...", topic.name());

        let mut engine = build_experiment_engine_from_registry(*topic, &providers);

        let result =
            engine
                .run(Context::new())
                .map_err(|e| MarketExpansionError::ExperimentFailed {
                    topic: topic.name().to_string(),
                    reason: format!("{e}"),
                })?;

        total_cycles += result.cycles;
        println!("  ✓ Converged in {} cycles", result.cycles);

        // Show analysis summary
        for fact in result.context.get(ContextKey::Hypotheses) {
            if fact.id.starts_with("analysis:") {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    if let Some(summary) = parsed.get("summary").and_then(|s| s.as_str()) {
                        println!("  Summary: {}", &summary[..summary.len().min(100)]);
                    }
                }
            }
        }

        experiment_contexts.push(result.context);
    }

    // ── Build consensus context ─────────────────────────────────────
    println!("\n▸ Running Consensus Engine...");
    let mut consensus_ctx = Context::new();

    // Merge experiment hypotheses into consensus context
    for exp_ctx in &experiment_contexts {
        for fact in exp_ctx.get(ContextKey::Hypotheses) {
            if fact.id.starts_with("analysis:") {
                let _ = consensus_ctx.add_fact(fact.clone());
            }
        }
    }

    let mut consensus_engine = build_consensus_engine();
    let run_result = consensus_engine.run_with_hitl(consensus_ctx);

    match run_result {
        RunResult::HitlPause(pause) => {
            println!("\n▸ HITL Gate Triggered!");
            println!("  Gate ID: {}", pause.request.gate_id);
            println!(
                "  Proposal: {}",
                pause.request.summary.chars().take(120).collect::<String>()
            );
            println!("  Confidence: 0.72 (threshold: 0.80)");
            println!("  → Auto-approving for spike demonstration...\n");

            // Approve the HITL gate
            let decision =
                GateDecision::approve(pause.request.gate_id.clone(), "spike-demo-auto-approver");
            let resume_result = consensus_engine.resume(*pause, decision);

            match resume_result {
                RunResult::Complete(Ok(result)) => {
                    total_cycles += result.cycles;
                    println!("  ✓ Consensus converged in {} cycles", result.cycles);
                    print_result(&result.context, total_cycles, true)
                }
                RunResult::Complete(Err(e)) => Err(MarketExpansionError::ConsensusFailed(format!(
                    "after HITL: {e}"
                ))),
                RunResult::HitlPause(_) => Err(MarketExpansionError::ConsensusFailed(
                    "unexpected second HITL pause".into(),
                )),
            }
        }
        RunResult::Complete(Ok(result)) => {
            total_cycles += result.cycles;
            println!(
                "  ✓ Consensus converged in {} cycles (no HITL triggered)",
                result.cycles
            );
            print_result(&result.context, total_cycles, false)
        }
        RunResult::Complete(Err(e)) => Err(MarketExpansionError::ConsensusFailed(format!("{e}"))),
    }
}

fn print_result(
    ctx: &Context,
    total_cycles: u32,
    hitl_triggered: bool,
) -> Result<MarketExpansionResult, MarketExpansionError> {
    // Extract recommendation
    let recommendation = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "recommendation");

    let recommended_city = if let Some(rec) = recommendation {
        let parsed: serde_json::Value = serde_json::from_str(&rec.content).unwrap_or_default();
        let city = parsed
            .get("selected_city")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║  RECOMMENDATION: Establish R&D center in {:<22}  ║", city);
        println!("╚══════════════════════════════════════════════════════════════════╝");

        if let Some(rationale) = parsed.get("rationale").and_then(|v| v.as_str()) {
            println!("\n  Rationale: {rationale}");
        }

        city
    } else {
        println!("\n  ⚠ No recommendation produced");
        String::new()
    };

    // Show optimization details
    if let Some(opt) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "optimization_result")
    {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&opt.content) {
            println!("\n  Optimization Details:");
            println!(
                "    Status: {}",
                parsed.get("status").and_then(|v| v.as_str()).unwrap_or("?")
            );
            if let Some(details) = parsed.get("city_details") {
                println!(
                    "    Entry Cost: {}K EUR",
                    details
                        .get("entry_cost_k")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "    Talent Score: {}",
                    details
                        .get("talent_score")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "    Market Access: {}",
                    details
                        .get("market_access")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "    Tax Incentive: {}",
                    details
                        .get("tax_incentive")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
            }
        }
    }

    // Show ranking
    if let Some(ranking_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "city_ranking")
    {
        if let Ok(ranking) = serde_json::from_str::<Vec<serde_json::Value>>(&ranking_fact.content) {
            println!("\n  City Rankings:");
            for entry in ranking.iter().take(5) {
                println!(
                    "    #{}: {} (score: {:.1})",
                    entry.get("rank").and_then(|v| v.as_u64()).unwrap_or(0),
                    entry.get("city").and_then(|v| v.as_str()).unwrap_or("?"),
                    entry
                        .get("composite_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                );
            }
        }
    }

    println!("\n  Total convergence cycles: {total_cycles}");
    println!("  HITL gate triggered: {hitl_triggered}");

    Ok(MarketExpansionResult {
        recommended_city,
        hitl_triggered,
        total_cycles,
        context: ctx.clone(),
    })
}

/// Run the consensus-only path with canned experiment data (deterministic, no API keys).
///
/// Useful for testing HITL flow and CP-SAT optimization without external dependencies.
pub fn run_consensus_only_deterministic() -> Result<MarketExpansionResult, MarketExpansionError> {
    let mut ctx = Context::new();

    // Inject canned experiment analyses
    let topics = [
        ("market_demand", [85, 80, 78, 90, 72, 76, 88, 74]),
        ("competitive_landscape", [82, 84, 80, 92, 70, 78, 90, 75]),
        ("go_to_market_cost", [80, 85, 82, 70, 88, 78, 65, 90]),
    ];

    let city_names = [
        "Stockholm",
        "Berlin",
        "Amsterdam",
        "London",
        "Helsinki",
        "Copenhagen",
        "Zurich",
        "Dublin",
    ];

    for (topic, scores) in &topics {
        let score_map: serde_json::Map<String, serde_json::Value> = city_names
            .iter()
            .zip(scores.iter())
            .map(|(name, score)| ((*name).to_string(), serde_json::json!(*score)))
            .collect();

        let content = serde_json::json!({
            "scores": score_map,
            "summary": format!("Canned analysis for {topic}"),
        });

        ctx.add_fact(Fact::new(
            ContextKey::Hypotheses,
            format!("analysis:{topic}"),
            content.to_string(),
        ))
        .map_err(|e| MarketExpansionError::ConsensusFailed(format!("{e}")))?;
    }

    let mut engine = build_consensus_engine();
    let run_result = engine.run_with_hitl(ctx);

    match run_result {
        RunResult::HitlPause(pause) => {
            let decision =
                GateDecision::approve(pause.request.gate_id.clone(), "deterministic-test-approver");
            let resume_result = engine.resume(*pause, decision);

            match resume_result {
                RunResult::Complete(Ok(result)) => {
                    let city = result
                        .context
                        .get(ContextKey::Evaluations)
                        .iter()
                        .find(|f| f.id == "recommendation")
                        .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
                        .and_then(|v| v.get("selected_city")?.as_str().map(String::from))
                        .unwrap_or_default();

                    Ok(MarketExpansionResult {
                        recommended_city: city,
                        hitl_triggered: true,
                        total_cycles: result.cycles,
                        context: result.context,
                    })
                }
                RunResult::Complete(Err(e)) => {
                    Err(MarketExpansionError::ConsensusFailed(format!("{e}")))
                }
                RunResult::HitlPause(_) => {
                    Err(MarketExpansionError::ConsensusFailed("double HITL".into()))
                }
            }
        }
        RunResult::Complete(Ok(result)) => {
            let city = result
                .context
                .get(ContextKey::Evaluations)
                .iter()
                .find(|f| f.id == "recommendation")
                .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.content).ok())
                .and_then(|v| v.get("selected_city")?.as_str().map(String::from))
                .unwrap_or_default();

            Ok(MarketExpansionResult {
                recommended_city: city,
                hitl_triggered: false,
                total_cycles: result.cycles,
                context: result.context,
            })
        }
        RunResult::Complete(Err(e)) => Err(MarketExpansionError::ConsensusFailed(format!("{e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_consensus_converges() {
        let result = run_consensus_only_deterministic().expect("should converge");
        assert!(
            !result.recommended_city.is_empty(),
            "should recommend a city"
        );
        assert!(
            result.hitl_triggered,
            "HITL should trigger at confidence 0.72"
        );
    }

    #[test]
    fn deterministic_consensus_recommends_valid_city() {
        let result = run_consensus_only_deterministic().expect("should converge");
        let valid_cities = [
            "Stockholm",
            "Berlin",
            "Amsterdam",
            "London",
            "Helsinki",
            "Copenhagen",
            "Zurich",
            "Dublin",
        ];
        assert!(
            valid_cities.contains(&result.recommended_city.as_str()),
            "recommended city '{}' is not a candidate",
            result.recommended_city
        );
    }

    #[test]
    fn deterministic_consensus_respects_budget() {
        let result = run_consensus_only_deterministic().expect("should converge");
        let cities = candidate_cities();
        let selected = cities.iter().find(|c| c.name == result.recommended_city);
        assert!(selected.is_some(), "selected city not found");
        assert!(
            selected.unwrap().entry_cost_k <= 500,
            "selected city exceeds budget"
        );
    }

    #[test]
    fn hitl_pause_triggers_at_072_confidence() {
        let result = run_consensus_only_deterministic().expect("should converge");
        assert!(
            result.hitl_triggered,
            "HITL should trigger at 0.72 confidence < 0.80 threshold"
        );
    }

    #[test]
    #[ignore] // Requires ANTHROPIC_API_KEY + BRAVE_API_KEY
    fn full_pipeline_with_api_keys() {
        let result = run_market_expansion_verbose().expect("should converge");
        assert!(!result.recommended_city.is_empty());
        println!("Recommended: {}", result.recommended_city);
    }
}
