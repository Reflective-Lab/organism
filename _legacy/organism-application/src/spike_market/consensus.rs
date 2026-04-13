// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Consensus agents for the Nordic Market Expansion spike.
//!
//! 4 agents that merge experiment results into a final recommendation:
//! 1. AggregationAgent — merges 3 experiment results
//! 2. VotingAgent — ranks cities by multi-criteria vote
//! 3. OptimizationAgent — CP-SAT selects best city within constraints
//! 4. RecommendationAgent — emits ProposedFact (confidence 0.72 → HITL pause)

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact, ProposedFact};

use crate::spike_market::optimization::optimize_city_selection;
use crate::spike_market::scenario::candidate_cities;

// ── AggregationAgent ────────────────────────────────────────────────

/// Merges the 3 experiment analyses into a single composite view.
pub struct AggregationAgent;

impl Agent for AggregationAgent {
    fn name(&self) -> &str {
        "AggregationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        let has_all_experiments = [
            "analysis:market_demand",
            "analysis:competitive_landscape",
            "analysis:go_to_market_cost",
        ]
        .iter()
        .all(|id| hypotheses.iter().any(|f| f.id == *id));
        let has_aggregation = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "aggregated_scores");
        has_all_experiments && !has_aggregation
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        let cities = candidate_cities();

        // Extract scores from each experiment
        let mut city_scores: Vec<CityAggregatedScore> = cities
            .iter()
            .map(|c| CityAggregatedScore {
                name: c.name.clone(),
                base_weighted: c.weighted_score(),
                experiment_scores: Vec::new(),
                composite: c.weighted_score(),
            })
            .collect();

        for fact in hypotheses {
            if let Some(topic) = fact.id.strip_prefix("analysis:") {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    if let Some(scores) = parsed.get("scores").and_then(|s| s.as_object()) {
                        for cs in &mut city_scores {
                            if let Some(score) = scores.get(&cs.name).and_then(|v| v.as_f64()) {
                                cs.experiment_scores.push((topic.to_string(), score));
                            }
                        }
                    }
                }
            }
        }

        // Compute composite score: blend experiment scores with baseline.
        // With N experiments contributing, weight = N/3 experiment + (3-N)/3 baseline.
        // This ensures partial experiment data doesn't fully override the baseline.
        let total_experiments = 3.0_f64;
        for cs in &mut city_scores {
            let n = cs.experiment_scores.len() as f64;
            if n > 0.0 {
                let experiment_avg: f64 =
                    cs.experiment_scores.iter().map(|(_, s)| s).sum::<f64>() / n;
                let experiment_weight = n / total_experiments;
                let baseline_weight = 1.0 - experiment_weight;
                cs.composite =
                    experiment_weight * experiment_avg + baseline_weight * cs.base_weighted;
            } else {
                cs.composite = cs.base_weighted;
            }
        }

        let content = serde_json::to_string(&city_scores).unwrap_or_default();
        AgentEffect::with_fact(Fact::new(
            ContextKey::Strategies,
            "aggregated_scores",
            content,
        ))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CityAggregatedScore {
    name: String,
    base_weighted: f64,
    experiment_scores: Vec<(String, f64)>,
    #[serde(default)]
    composite: f64,
}

// ── VotingAgent ─────────────────────────────────────────────────────

/// Ranks cities by multi-criteria voting (Borda count over experiment scores).
pub struct VotingAgent;

impl Agent for VotingAgent {
    fn name(&self) -> &str {
        "VotingAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_aggregated = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "aggregated_scores");
        let has_ranking = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "city_ranking");
        has_aggregated && !has_ranking
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let aggregated = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|f| f.id == "aggregated_scores");

        let aggregated = match aggregated {
            Some(f) => f,
            None => return AgentEffect::empty(),
        };

        let scores: Vec<CityAggregatedScore> =
            serde_json::from_str(&aggregated.content).unwrap_or_default();

        // Borda count: rank cities by composite score, award points
        let mut ranked: Vec<(String, f64)> = scores
            .iter()
            .map(|s| (s.name.clone(), s.composite))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let n = ranked.len();
        let ranking: Vec<serde_json::Value> = ranked
            .iter()
            .enumerate()
            .map(|(rank, (name, score))| {
                serde_json::json!({
                    "rank": rank + 1,
                    "city": name,
                    "composite_score": score,
                    "borda_points": n - rank,
                })
            })
            .collect();

        let content = serde_json::to_string(&ranking).unwrap_or_default();
        AgentEffect::with_fact(Fact::new(ContextKey::Evaluations, "city_ranking", content))
    }
}

// ── OptimizationAgent ───────────────────────────────────────────────

/// Uses CP-SAT to select the optimal city within constraints.
pub struct OptimizationAgent {
    budget_k: u32,
    min_talent: u32,
}

impl OptimizationAgent {
    pub fn new(budget_k: u32, min_talent: u32) -> Self {
        Self {
            budget_k,
            min_talent,
        }
    }
}

impl Agent for OptimizationAgent {
    fn name(&self) -> &str {
        "OptimizationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_aggregated = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|f| f.id == "aggregated_scores");
        let has_ranking = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "city_ranking");
        let has_optimization = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "optimization_result");
        has_aggregated && has_ranking && !has_optimization
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let aggregated = ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|f| f.id == "aggregated_scores");

        let aggregated = match aggregated {
            Some(f) => f,
            None => return AgentEffect::empty(),
        };

        let scores: Vec<CityAggregatedScore> =
            serde_json::from_str(&aggregated.content).unwrap_or_default();
        let cities = candidate_cities();

        // Use composite scores as overrides for CP-SAT
        let overrides: Vec<f64> = cities
            .iter()
            .map(|c| {
                scores
                    .iter()
                    .find(|s| s.name == c.name)
                    .map_or(c.weighted_score(), |s| s.composite)
            })
            .collect();

        let result =
            optimize_city_selection(&cities, self.budget_k, self.min_talent, Some(&overrides));

        let content = serde_json::json!({
            "selected_city": result.city_name,
            "selected_index": result.selected_index,
            "objective_value": result.objective_value,
            "status": format!("{:?}", result.status),
            "wall_time_secs": result.wall_time,
            "budget_k": self.budget_k,
            "min_talent": self.min_talent,
            "city_details": {
                "entry_cost_k": cities[result.selected_index].entry_cost_k,
                "talent_score": cities[result.selected_index].talent_score,
                "market_access": cities[result.selected_index].market_access,
                "tax_incentive": cities[result.selected_index].tax_incentive,
            },
        })
        .to_string();

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            content,
        ))
    }
}

// ── RecommendationAgent ─────────────────────────────────────────────

/// Emits a ProposedFact with configurable confidence, potentially triggering HITL pause.
pub struct RecommendationAgent {
    confidence: f64,
    budget_k: u32,
    min_talent: u32,
}

impl RecommendationAgent {
    pub fn new(confidence: f64, budget_k: u32, min_talent: u32) -> Self {
        Self {
            confidence,
            budget_k,
            min_talent,
        }
    }
}

impl Agent for RecommendationAgent {
    fn name(&self) -> &str {
        "RecommendationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_optimization = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "optimization_result");
        let has_recommendation = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|f| f.id == "recommendation");
        // Also check if we were HITL-rejected (don't re-propose)
        let was_rejected = ctx
            .get(ContextKey::Diagnostic)
            .iter()
            .any(|f| f.id == "hitl-rejected:recommendation");
        has_optimization && !has_recommendation && !was_rejected
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let opt_result = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .find(|f| f.id == "optimization_result");

        let opt_result = match opt_result {
            Some(f) => f,
            None => return AgentEffect::empty(),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(&opt_result.content).unwrap_or_default();
        let city = parsed
            .get("selected_city")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let budget_k = self.budget_k;
        let min_talent = self.min_talent;

        let content = serde_json::json!({
            "recommendation": format!("Establish European R&D center in {city}"),
            "selected_city": city,
            "confidence": self.confidence,
            "rationale": format!(
                "CP-SAT optimization selected {city} as the optimal location within \
                 budget constraints (≤{budget_k}K EUR) and talent requirements \
                 (≥{min_talent}). The selection maximizes a weighted composite \
                 of talent, market access, tax incentives, and cost efficiency, \
                 informed by market demand, competitive landscape, and go-to-market \
                 cost analysis across all 3 research experiments."
            ),
            "optimization_details": parsed,
        })
        .to_string();

        AgentEffect::with_proposal(ProposedFact {
            key: ContextKey::Evaluations,
            id: "recommendation".into(),
            content,
            confidence: self.confidence,
            provenance: "consensus:optimization+voting+aggregation".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregation_requires_all_experiments() {
        let agent = AggregationAgent;
        let ctx = Context::new();
        assert!(
            !agent.accepts(&ctx),
            "should not accept with no experiments"
        );
    }

    #[test]
    fn aggregation_produces_strategy() {
        let agent = AggregationAgent;
        let mut ctx = Context::new();

        // Add all 3 experiment analyses
        for topic in [
            "market_demand",
            "competitive_landscape",
            "go_to_market_cost",
        ] {
            let scores = serde_json::json!({
                "scores": {
                    "Stockholm": 85,
                    "Berlin": 80,
                    "Amsterdam": 78,
                    "London": 90,
                    "Helsinki": 72,
                    "Copenhagen": 76,
                    "Zurich": 88,
                    "Dublin": 74,
                },
                "summary": format!("Analysis for {topic}"),
            });
            ctx.add_fact(Fact::new(
                ContextKey::Hypotheses,
                format!("analysis:{topic}"),
                scores.to_string(),
            ))
            .unwrap();
        }

        assert!(agent.accepts(&ctx));
        let effect = agent.execute(&ctx);
        assert_eq!(effect.facts.len(), 1);
        assert_eq!(effect.facts[0].id, "aggregated_scores");
    }

    #[test]
    fn voting_ranks_cities() {
        let mut ctx = Context::new();

        // Aggregated scores
        let scores = serde_json::json!([
            {"name": "Stockholm", "base_weighted": 80.0, "experiment_scores": [], "composite": 85.0},
            {"name": "Berlin", "base_weighted": 75.0, "experiment_scores": [], "composite": 80.0},
        ]);
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "aggregated_scores",
            scores.to_string(),
        ))
        .unwrap();

        let agent = VotingAgent;
        assert!(agent.accepts(&ctx));

        let effect = agent.execute(&ctx);
        assert_eq!(effect.facts.len(), 1);
        assert_eq!(effect.facts[0].id, "city_ranking");

        let ranking: Vec<serde_json::Value> =
            serde_json::from_str(&effect.facts[0].content).unwrap();
        assert_eq!(ranking[0]["city"], "Stockholm");
        assert_eq!(ranking[0]["rank"], 1);
    }

    #[test]
    fn recommendation_emits_proposal_with_configured_confidence() {
        let mut ctx = Context::new();

        let opt = serde_json::json!({
            "selected_city": "Dublin",
            "selected_index": 7,
            "status": "Optimal",
        });
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            opt.to_string(),
        ))
        .unwrap();

        let agent = RecommendationAgent::new(0.72, 500, 75);
        assert!(agent.accepts(&ctx));

        let effect = agent.execute(&ctx);
        assert!(effect.facts.is_empty(), "should use proposals, not facts");
        assert_eq!(effect.proposals.len(), 1);
        assert_eq!(effect.proposals[0].id, "recommendation");
        assert!(
            (effect.proposals[0].confidence - 0.72).abs() < f64::EPSILON,
            "confidence should be 0.72"
        );
    }

    #[test]
    fn recommendation_respects_hitl_rejection() {
        let mut ctx = Context::new();

        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "optimization_result",
            "{}",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Diagnostic,
            "hitl-rejected:recommendation",
            "rejected by human",
        ))
        .unwrap();

        let agent = RecommendationAgent::new(0.72, 500, 75);
        assert!(
            !agent.accepts(&ctx),
            "should not re-propose after HITL rejection"
        );
    }
}
