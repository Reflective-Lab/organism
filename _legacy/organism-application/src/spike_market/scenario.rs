// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! City data structures and engine builders for the Nordic Market Expansion spike.

use std::sync::Arc;

use converge_core::gates::hitl::TimeoutPolicy;
use converge_core::{Engine, EngineHitlPolicy};
use converge_provider::brave::BraveSearchProvider;
use converge_provider::provider_api::{AgentRequirements, LlmProvider};
use converge_provider::{FallbackLlmProvider, ProviderRegistry, SelectionResult, create_provider};

use serde::{Deserialize, Serialize};

use crate::spike_market::agents::{ResearchAnalystAgent, SearchPlannerAgent, WebSearchAgent};
use crate::spike_market::consensus::{
    AggregationAgent, OptimizationAgent, RecommendationAgent, VotingAgent,
};
use crate::spike_market::invariants::{
    BudgetConstraintInvariant, ConsensusRequiredInvariant, MinimumScoreInvariant,
};

/// A candidate city for R&D center placement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateCity {
    pub name: String,
    /// Entry cost in thousands of EUR.
    pub entry_cost_k: u32,
    /// Talent score (0-100).
    pub talent_score: u32,
    /// Market access score (0-100).
    pub market_access: u32,
    /// Tax incentive score (0-100).
    pub tax_incentive: u32,
}

impl CandidateCity {
    /// Compute weighted score: 0.3×talent + 0.25×market + 0.25×tax + 0.2×cost_efficiency.
    /// Cost efficiency = 100 - (entry_cost_k / 5) clamped to [0, 100].
    #[must_use]
    pub fn weighted_score(&self) -> f64 {
        let cost_eff = (100.0 - f64::from(self.entry_cost_k) / 5.0).clamp(0.0, 100.0);
        0.30 * f64::from(self.talent_score)
            + 0.25 * f64::from(self.market_access)
            + 0.25 * f64::from(self.tax_incentive)
            + 0.20 * cost_eff
    }
}

/// Returns the 8 candidate cities with their static data.
#[must_use]
pub fn candidate_cities() -> Vec<CandidateCity> {
    vec![
        CandidateCity {
            name: "Stockholm".into(),
            entry_cost_k: 350,
            talent_score: 88,
            market_access: 75,
            tax_incentive: 80,
        },
        CandidateCity {
            name: "Berlin".into(),
            entry_cost_k: 280,
            talent_score: 82,
            market_access: 85,
            tax_incentive: 70,
        },
        CandidateCity {
            name: "Amsterdam".into(),
            entry_cost_k: 320,
            talent_score: 80,
            market_access: 90,
            tax_incentive: 65,
        },
        CandidateCity {
            name: "London".into(),
            entry_cost_k: 450,
            talent_score: 95,
            market_access: 95,
            tax_incentive: 50,
        },
        CandidateCity {
            name: "Helsinki".into(),
            entry_cost_k: 250,
            talent_score: 78,
            market_access: 60,
            tax_incentive: 85,
        },
        CandidateCity {
            name: "Copenhagen".into(),
            entry_cost_k: 330,
            talent_score: 84,
            market_access: 70,
            tax_incentive: 75,
        },
        CandidateCity {
            name: "Zurich".into(),
            entry_cost_k: 500,
            talent_score: 92,
            market_access: 80,
            tax_incentive: 60,
        },
        CandidateCity {
            name: "Dublin".into(),
            entry_cost_k: 300,
            talent_score: 76,
            market_access: 75,
            tax_incentive: 90,
        },
    ]
}

/// Budget constraint in thousands of EUR.
pub const MAX_BUDGET_K: u32 = 500;

/// Minimum talent score for a city to be eligible.
pub const MIN_TALENT_SCORE: u32 = 75;

/// Configuration for a spike run — all the knobs in one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpikeConfig {
    /// Maximum entry cost budget in thousands of EUR.
    pub budget_k: u32,
    /// Minimum talent score for a city to qualify.
    pub min_talent: u32,
    /// HITL confidence threshold: proposals at or below this pause for human approval.
    pub hitl_threshold: f64,
    /// Confidence the RecommendationAgent assigns to its output.
    pub recommendation_confidence: f64,
}

impl Default for SpikeConfig {
    fn default() -> Self {
        Self {
            budget_k: MAX_BUDGET_K,
            min_talent: MIN_TALENT_SCORE,
            hitl_threshold: 0.80,
            recommendation_confidence: 0.72,
        }
    }
}

/// Research topics for the 3 experiments.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExperimentTopic {
    MarketDemand,
    CompetitiveLandscape,
    GoToMarketCost,
}

impl ExperimentTopic {
    #[must_use]
    pub fn research_question(&self) -> &'static str {
        match self {
            Self::MarketDemand => {
                "What is the current demand for tech R&D talent and services in Nordic and European cities? Which cities have the strongest growing tech ecosystems?"
            }
            Self::CompetitiveLandscape => {
                "What major tech companies have R&D centers in European cities? What is the competitive landscape for tech talent in Stockholm, Berlin, Amsterdam, London, Helsinki, Copenhagen, Zurich, and Dublin?"
            }
            Self::GoToMarketCost => {
                "What are the costs of establishing an R&D center in European cities? Compare office costs, salaries, tax incentives, and regulatory environment in Stockholm, Berlin, Amsterdam, London, Helsinki, Copenhagen, Zurich, and Dublin."
            }
        }
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::MarketDemand => "market_demand",
            Self::CompetitiveLandscape => "competitive_landscape",
            Self::GoToMarketCost => "go_to_market_cost",
        }
    }
}

/// Build an experiment engine (SearchPlanner → WebSearch → ResearchAnalyst).
pub fn build_experiment_engine(
    topic: ExperimentTopic,
    llm_planner: Arc<dyn LlmProvider>,
    llm_analyst: Arc<dyn LlmProvider>,
    search_provider: Arc<BraveSearchProvider>,
) -> Engine {
    let mut engine = Engine::new();

    engine.register(SearchPlannerAgent::new(
        topic.name().to_string(),
        topic.research_question().to_string(),
        llm_planner,
    ));
    engine.register(WebSearchAgent::new(
        topic.name().to_string(),
        search_provider,
    ));
    engine.register(ResearchAnalystAgent::new(
        topic.name().to_string(),
        llm_analyst,
    ));

    engine
}

/// Result of capability-based provider discovery.
pub struct ProviderSetup {
    /// The planner LLM provider (fast/cheap).
    pub planner: Arc<dyn LlmProvider>,
    /// Selection details for the planner.
    pub planner_selection: SelectionResult,
    /// The analyst LLM provider (powerful).
    pub analyst: Arc<dyn LlmProvider>,
    /// Selection details for the analyst.
    pub analyst_selection: SelectionResult,
    /// The Brave search provider.
    pub search: Arc<BraveSearchProvider>,
}

impl std::fmt::Debug for ProviderSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderSetup")
            .field(
                "planner",
                &format_args!(
                    "{}/{}",
                    self.planner_selection.selected.provider, self.planner_selection.selected.model
                ),
            )
            .field(
                "analyst",
                &format_args!(
                    "{}/{}",
                    self.analyst_selection.selected.provider, self.analyst_selection.selected.model
                ),
            )
            .field("search", &self.search)
            .finish()
    }
}

/// Discover and create providers using capability-based matching.
///
/// Uses `ProviderRegistry` to find the best available LLM for each role:
/// - **Planner** (SearchPlannerAgent): `fast_cheap()` — generates search queries
/// - **Analyst** (ResearchAnalystAgent): `powerful()` — analyzes results and scores cities
/// - **Search**: Brave web search (requires `BRAVE_API_KEY`)
///
/// When `defensive` is true, each provider is health-checked after creation.
/// If the health check fails (quota exhausted, invalid key, etc.), the next
/// best candidate is tried. This costs one minimal API call per provider but
/// catches problems early instead of failing mid-convergence.
///
/// When `defensive` is false (optimistic mode), the best-scoring provider is
/// used without verification — faster startup, but errors surface later.
///
/// # Errors
///
/// Returns error if no provider satisfies a role's requirements, or if Brave is unavailable.
pub fn discover_providers(registry: &ProviderRegistry) -> Result<ProviderSetup, String> {
    discover_providers_with_mode(registry, true)
}

/// Discover providers with explicit health-check mode.
pub fn discover_providers_with_mode(
    registry: &ProviderRegistry,
    defensive: bool,
) -> Result<ProviderSetup, String> {
    let planner_reqs = AgentRequirements::fast_cheap();
    let (planner, planner_selection) =
        select_and_verify(registry, &planner_reqs, "planner", defensive)?;

    let analyst_reqs = AgentRequirements::powerful();
    let (analyst, analyst_selection) =
        select_and_verify(registry, &analyst_reqs, "analyst", defensive)?;

    // Brave search
    let search = Arc::new(
        BraveSearchProvider::from_env().map_err(|e| format!("Brave search unavailable: {e}"))?,
    );

    Ok(ProviderSetup {
        planner,
        planner_selection,
        analyst,
        analyst_selection,
        search,
    })
}

/// Select a provider with fallback support.
///
/// In defensive mode: health-checks the first viable candidate, then builds a
/// `FallbackLlmProvider` wrapping all remaining candidates for mid-run resilience.
///
/// In optimistic mode: wraps all candidates into a `FallbackLlmProvider` without
/// health checks — failures are handled transparently at call time.
fn select_and_verify(
    registry: &ProviderRegistry,
    requirements: &AgentRequirements,
    role: &str,
    defensive: bool,
) -> Result<(Arc<dyn LlmProvider>, SelectionResult), String> {
    let selection = registry
        .select_with_details(requirements)
        .map_err(|e| format!("No LLM for {role}: {e}"))?;

    // Create all viable providers (skip those that fail construction)
    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();
    let mut first_healthy_idx = None;

    for (i, (candidate, _fitness)) in selection.candidates.iter().enumerate() {
        let provider = match create_provider(&candidate.provider, &candidate.model) {
            Ok(p) => p,
            Err(e) => {
                eprintln!(
                    "  ⚠ {role}: {}/{} — creation failed: {e}",
                    candidate.provider, candidate.model
                );
                continue;
            }
        };

        if defensive && first_healthy_idx.is_none() {
            // Health-check to find the first working provider
            match provider.health_check() {
                Ok(()) => {
                    first_healthy_idx = Some(providers.len());
                    providers.push(provider);
                }
                Err(e) => {
                    eprintln!(
                        "  ⚠ {role}: {}/{} — health check failed: {e}",
                        candidate.provider, candidate.model
                    );
                    // Still add as a fallback — it might recover later
                    providers.push(provider);
                }
            }
        } else {
            providers.push(provider);
            if first_healthy_idx.is_none() && !defensive {
                first_healthy_idx = Some(i);
            }
        }
    }

    if providers.is_empty() {
        return Err(format!("No providers could be created for {role}"));
    }

    if defensive && first_healthy_idx.is_none() {
        return Err(format!(
            "All candidates for {role} failed health checks: {}",
            selection
                .candidates
                .iter()
                .map(|(c, _)| format!("{}/{}", c.provider, c.model))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Build the FallbackLlmProvider, starting at the first healthy candidate
    let start = first_healthy_idx.unwrap_or(0);
    let fallback = FallbackLlmProvider::new(providers);

    // Build a SelectionResult reflecting the primary pick
    let primary_idx = start.min(selection.candidates.len() - 1);
    let (primary_candidate, primary_fitness) = &selection.candidates[primary_idx];
    let actual_selection = SelectionResult {
        selected: primary_candidate.clone(),
        fitness: primary_fitness.clone(),
        candidates: selection.candidates.clone(),
        rejected: selection.rejected.clone(),
    };

    // Log the fallback chain
    let candidates_desc = fallback.describe_candidates();
    if candidates_desc.len() > 1 {
        eprintln!(
            "  ℹ {role}: fallback chain: {}",
            candidates_desc.join(" → ")
        );
    }

    Ok((Arc::new(fallback), actual_selection))
}

/// Build an experiment engine using capability-discovered providers.
pub fn build_experiment_engine_from_registry(
    topic: ExperimentTopic,
    providers: &ProviderSetup,
) -> Engine {
    build_experiment_engine(
        topic,
        Arc::clone(&providers.planner),
        Arc::clone(&providers.analyst),
        Arc::clone(&providers.search),
    )
}

/// Build the consensus engine (Aggregation → Voting → Optimization → Recommendation + HITL).
///
/// Uses default config. Prefer `build_consensus_engine_with_config` for custom settings.
pub fn build_consensus_engine() -> Engine {
    build_consensus_engine_with_config(&SpikeConfig::default())
}

/// Build the consensus engine with explicit configuration.
pub fn build_consensus_engine_with_config(config: &SpikeConfig) -> Engine {
    let mut engine = Engine::new();

    engine.register(AggregationAgent);
    engine.register(VotingAgent);
    engine.register(OptimizationAgent::new(config.budget_k, config.min_talent));
    engine.register(RecommendationAgent::new(
        config.recommendation_confidence,
        config.budget_k,
        config.min_talent,
    ));

    // Register invariants
    engine.register_invariant(BudgetConstraintInvariant::new(config.budget_k));
    engine.register_invariant(MinimumScoreInvariant::new(config.min_talent));
    engine.register_invariant(ConsensusRequiredInvariant);

    // HITL policy: proposals at or below threshold trigger pause
    engine.set_hitl_policy(EngineHitlPolicy {
        confidence_threshold: Some(config.hitl_threshold),
        gated_keys: vec![],
        timeout: TimeoutPolicy::default(),
    });

    engine
}

/// Create a test intent for market expansion.
pub fn test_market_intent() -> organism_core::intent::OrganismIntent {
    use converge_core::{
        Budgets, ConstraintSeverity, IntentConstraint, IntentId, IntentKind, Objective, RootIntent,
        Scope, SuccessCriteria,
    };
    use organism_core::intent::Reversibility;

    let root = RootIntent {
        id: IntentId::new("market-expansion-001"),
        kind: IntentKind::Custom,
        objective: Some(Objective::Custom(
            "Select optimal European R&D location".into(),
        )),
        scope: Scope::default(),
        constraints: vec![
            IntentConstraint {
                key: "budget".into(),
                value: format!("{MAX_BUDGET_K}K EUR"),
                severity: ConstraintSeverity::Hard,
            },
            IntentConstraint {
                key: "region".into(),
                value: "Europe".into(),
                severity: ConstraintSeverity::Hard,
            },
        ],
        success_criteria: SuccessCriteria::default(),
        budgets: Budgets::default(),
    };

    organism_core::intent::OrganismIntent::new(root).with_reversibility(Reversibility::Partial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_cities_returns_8() {
        assert_eq!(candidate_cities().len(), 8);
    }

    #[test]
    fn weighted_score_is_reasonable() {
        let cities = candidate_cities();
        for city in &cities {
            let score = city.weighted_score();
            assert!(score > 0.0 && score < 100.0, "{}: {}", city.name, score);
        }
    }

    #[test]
    fn consensus_engine_builds() {
        let _engine = build_consensus_engine();
    }

    #[test]
    fn test_intent_builds() {
        let _intent = test_market_intent();
    }
}
