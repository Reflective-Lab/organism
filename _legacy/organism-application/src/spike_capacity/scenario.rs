// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Scenario wiring and dataset loading for Spike 3.

use std::sync::Arc;

use converge_core::Engine;
use converge_optimization::packs::capacity_planning::{
    CapacityPlanningInput, DemandForecast, PlanningConstraints, ResourceType, Team,
};
use converge_provider::brave::BraveSearchProvider;
use converge_provider::provider_api::{AgentRequirements, CostClass, LlmProvider};
use converge_provider::{FallbackLlmProvider, ProviderRegistry, SelectionResult, create_provider};
use organism_core::intent::{OrganismIntent, Reversibility};

use crate::spike_capacity::agents::{
    ExperimentDataLoadAgent, HybridResearchAnalysisAgent, ResearchAnalysisAgent,
    SearchPlannerAgent, WebSearchAgent,
};
use crate::spike_capacity::consensus::{
    AnalyticsScoringAgent, CapacityOptimizationAgent, HuddleSynthesisAgent, PreplanningHuddleAgent,
    RecommendationAgent, SimulationAgent, VotingAgent,
};
use crate::spike_capacity::invariants::{
    DatasetProvenanceInvariant, FeasibleRecommendationInvariant, ResearchCoverageInvariant,
};

const DEMAND_FORECAST_JSON: &str = include_str!("../../data/spike_capacity/demand_forecast.json");
const DELIVERY_HISTORY_JSON: &str = include_str!("../../data/spike_capacity/delivery_history.json");
const TEAM_CAPACITY_JSON: &str = include_str!("../../data/spike_capacity/team_capacity.json");

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DemandDataset {
    pub dataset_version: String,
    pub records: Vec<DemandRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DemandRecord {
    pub period_id: String,
    pub skill: String,
    pub demand_units: f64,
    pub priority: u32,
    pub confidence: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeliveryHistoryDataset {
    pub dataset_version: String,
    pub records: Vec<DeliveryHistoryRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeliveryHistoryRecord {
    pub skill: String,
    pub completed_units: f64,
    pub lateness_rate: f64,
    pub spillover_rate: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamCapacityDataset {
    pub dataset_version: String,
    pub records: Vec<TeamCapacityRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamCapacityRecord {
    pub team_id: String,
    pub team_name: String,
    pub skills: Vec<String>,
    pub available_capacity: f64,
    pub max_utilization: f64,
    pub headcount: i32,
}

#[derive(Debug, Clone)]
pub struct CapacityBundle {
    pub demand: DemandDataset,
    pub history: DeliveryHistoryDataset,
    pub capacity: TeamCapacityDataset,
}

/// Capability-based live provider setup for Spike 3.
pub struct CapacityProviderSetup {
    pub planner: Arc<dyn LlmProvider>,
    pub planner_selection: SelectionResult,
    pub analyst: Arc<dyn LlmProvider>,
    pub analyst_selection: SelectionResult,
    pub huddle: Arc<dyn LlmProvider>,
    pub huddle_selection: SelectionResult,
    pub simulator: Arc<dyn LlmProvider>,
    pub simulator_selection: SelectionResult,
    pub search: Arc<BraveSearchProvider>,
}

impl std::fmt::Debug for CapacityProviderSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapacityProviderSetup")
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
            .field(
                "huddle",
                &format_args!(
                    "{}/{}",
                    self.huddle_selection.selected.provider, self.huddle_selection.selected.model
                ),
            )
            .field(
                "simulator",
                &format_args!(
                    "{}/{}",
                    self.simulator_selection.selected.provider,
                    self.simulator_selection.selected.model
                ),
            )
            .finish()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpikeConfig {
    pub planning_budget: f64,
    pub target_utilization: f64,
    pub min_overall_fulfillment: f64,
    pub resource_unit_cost: f64,
}

impl Default for SpikeConfig {
    fn default() -> Self {
        Self {
            planning_budget: 115_000.0,
            target_utilization: 0.8,
            min_overall_fulfillment: 0.92,
            resource_unit_cost: 120.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExperimentTopic {
    DemandResearch,
    DeliveryDeepResearch,
    WorkforceResearch,
}

impl ExperimentTopic {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::DemandResearch => "demand_research",
            Self::DeliveryDeepResearch => "delivery_deep_research",
            Self::WorkforceResearch => "workforce_research",
        }
    }

    #[must_use]
    pub fn research_question(self) -> &'static str {
        match self {
            Self::DemandResearch => {
                "What demand signals and market changes suggest pressure on backend, data, QA, platform, and ML capacity over the next two quarters?"
            }
            Self::DeliveryDeepResearch => {
                "What operational patterns indicate delivery lateness, spillover, and execution risk for software teams working across backend, data, QA, platform, and ML?"
            }
            Self::WorkforceResearch => {
                "What hiring, staffing, and skill-availability patterns shape realistic capacity planning choices for backend, data, QA, platform, and ML teams?"
            }
        }
    }
}

pub fn load_capacity_bundle() -> Result<CapacityBundle, String> {
    Ok(CapacityBundle {
        demand: parse_json("demand forecast", DEMAND_FORECAST_JSON)?,
        history: parse_json("delivery history", DELIVERY_HISTORY_JSON)?,
        capacity: parse_json("team capacity", TEAM_CAPACITY_JSON)?,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(label: &str, content: &str) -> Result<T, String> {
    serde_json::from_str(content).map_err(|e| format!("failed to parse {label}: {e}"))
}

pub fn build_experiment_engine(topic: ExperimentTopic) -> Engine {
    let mut engine = Engine::new();
    engine.register(ExperimentDataLoadAgent::new(topic));
    engine.register(ResearchAnalysisAgent::new(topic));
    engine
}

pub fn build_experiment_engine_with_providers(
    topic: ExperimentTopic,
    providers: &CapacityProviderSetup,
) -> Engine {
    let mut engine = Engine::new();
    engine.register(ExperimentDataLoadAgent::new(topic));
    engine.register(SearchPlannerAgent::new(
        topic,
        topic.research_question().to_string(),
        Arc::clone(&providers.planner),
    ));
    engine.register(WebSearchAgent::new(topic, Arc::clone(&providers.search)));
    engine.register(HybridResearchAnalysisAgent::new(
        topic,
        Arc::clone(&providers.analyst),
    ));
    engine
}

pub fn build_consensus_engine() -> Engine {
    build_consensus_engine_with_config(&SpikeConfig::default())
}

pub fn build_consensus_engine_with_config(config: &SpikeConfig) -> Engine {
    let mut engine = Engine::new();
    engine.register(PreplanningHuddleAgent::local());
    engine.register(HuddleSynthesisAgent);
    engine.register(VotingAgent);
    engine.register(AnalyticsScoringAgent);
    engine.register(SimulationAgent::new(config.clone()));
    engine.register(CapacityOptimizationAgent::new(config.clone()));
    engine.register(RecommendationAgent);

    engine.register_invariant(ResearchCoverageInvariant);
    engine.register_invariant(DatasetProvenanceInvariant);
    engine.register_invariant(FeasibleRecommendationInvariant);

    engine
}

pub fn build_consensus_engine_with_providers(
    config: &SpikeConfig,
    providers: &CapacityProviderSetup,
) -> Engine {
    let mut engine = Engine::new();
    engine.register(PreplanningHuddleAgent::live(Arc::clone(&providers.huddle)));
    engine.register(HuddleSynthesisAgent);
    engine.register(VotingAgent);
    engine.register(AnalyticsScoringAgent);
    engine.register(SimulationAgent::live(
        config.clone(),
        Arc::clone(&providers.simulator),
    ));
    engine.register(CapacityOptimizationAgent::new(config.clone()));
    engine.register(RecommendationAgent);

    engine.register_invariant(ResearchCoverageInvariant);
    engine.register_invariant(DatasetProvenanceInvariant);
    engine.register_invariant(FeasibleRecommendationInvariant);

    engine
}

pub fn discover_providers(registry: &ProviderRegistry) -> Result<CapacityProviderSetup, String> {
    let planner_reqs = AgentRequirements::fast_cheap().with_quality(0.7);
    let (planner, planner_selection) =
        select_and_verify(registry, &planner_reqs, "research_planner", true)?;

    let analyst_reqs = AgentRequirements::powerful();
    let (analyst, analyst_selection) =
        select_and_verify(registry, &analyst_reqs, "research_analyst", true)?;

    let huddle_reqs = AgentRequirements::new(CostClass::High, 10_000, true).with_quality(0.88);
    let (huddle, huddle_selection) =
        select_and_verify(registry, &huddle_reqs, "preplanning_huddle", true)?;

    let simulator_reqs = AgentRequirements::new(CostClass::Low, 6_000, true).with_quality(0.8);
    let (simulator, simulator_selection) =
        select_and_verify(registry, &simulator_reqs, "simulation_analyst", true)?;

    let search = Arc::new(
        BraveSearchProvider::from_env().map_err(|e| format!("Brave search unavailable: {e}"))?,
    );

    Ok(CapacityProviderSetup {
        planner,
        planner_selection,
        analyst,
        analyst_selection,
        huddle,
        huddle_selection,
        simulator,
        simulator_selection,
        search,
    })
}

fn select_and_verify(
    registry: &ProviderRegistry,
    requirements: &AgentRequirements,
    role: &str,
    defensive: bool,
) -> Result<(Arc<dyn LlmProvider>, SelectionResult), String> {
    let selection = registry
        .select_with_details(requirements)
        .map_err(|e| format!("No LLM for {role}: {e}"))?;

    let mut providers: Vec<Arc<dyn LlmProvider>> = Vec::new();
    let mut first_healthy_idx = None;

    for (i, (candidate, _fitness)) in selection.candidates.iter().enumerate() {
        let provider = match create_provider(&candidate.provider, &candidate.model) {
            Ok(provider) => provider,
            Err(_) => continue,
        };

        if defensive && first_healthy_idx.is_none() {
            match provider.health_check() {
                Ok(()) => {
                    first_healthy_idx = Some(providers.len());
                    providers.push(provider);
                }
                Err(_) => providers.push(provider),
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
        return Err(format!("All providers for {role} failed health checks"));
    }

    let start = first_healthy_idx.unwrap_or(0);
    let fallback = FallbackLlmProvider::new(providers);
    let primary_idx = start.min(selection.candidates.len() - 1);
    let (primary_candidate, primary_fitness) = &selection.candidates[primary_idx];
    let actual_selection = SelectionResult {
        selected: primary_candidate.clone(),
        fitness: primary_fitness.clone(),
        candidates: selection.candidates.clone(),
        rejected: selection.rejected.clone(),
    };

    Ok((Arc::new(fallback), actual_selection))
}

pub fn build_capacity_input(config: &SpikeConfig) -> Result<CapacityPlanningInput, String> {
    let bundle = load_capacity_bundle()?;
    Ok(CapacityPlanningInput {
        demand_forecasts: bundle
            .demand
            .records
            .iter()
            .map(|record| DemandForecast {
                period_id: record.period_id.clone(),
                resource_type: "engineering".to_string(),
                required_skill: record.skill.clone(),
                demand_units: record.demand_units,
                priority: record.priority,
                min_fulfillment_ratio: 0.7,
            })
            .collect(),
        resource_types: vec![ResourceType {
            id: "engineering".to_string(),
            name: "Engineering Capacity".to_string(),
            unit: "hours".to_string(),
            cost_per_unit: config.resource_unit_cost,
        }],
        teams: bundle
            .capacity
            .records
            .iter()
            .map(|record| Team {
                id: record.team_id.clone(),
                name: record.team_name.clone(),
                skills: record.skills.clone(),
                resource_types: vec!["engineering".to_string()],
                available_capacity: record.available_capacity,
                max_utilization: record.max_utilization,
                headcount: record.headcount,
            })
            .collect(),
        constraints: PlanningConstraints {
            target_utilization: config.target_utilization,
            max_budget: Some(config.planning_budget),
            min_overall_fulfillment: config.min_overall_fulfillment,
            allow_cross_team: true,
            strict_skill_matching: true,
        },
    })
}

pub fn test_capacity_intent() -> OrganismIntent {
    use converge_core::{
        Budgets, ConstraintSeverity, IntentConstraint, IntentId, IntentKind, Objective, RootIntent,
        Scope, SuccessCriteria,
    };

    let root = RootIntent {
        id: IntentId::new("capacity-planning-001"),
        kind: IntentKind::Custom,
        objective: Some(Objective::Custom(
            "Produce a feasible quarter plan that matches demand with team capacity".into(),
        )),
        scope: Scope::default(),
        constraints: vec![
            IntentConstraint {
                key: "planning_horizon".into(),
                value: "2026-Q3 to 2026-Q4".into(),
                severity: ConstraintSeverity::Hard,
            },
            IntentConstraint {
                key: "optimization".into(),
                value: "capacity-planning-pack".into(),
                severity: ConstraintSeverity::Hard,
            },
        ],
        success_criteria: SuccessCriteria::default(),
        budgets: Budgets::default(),
    };

    OrganismIntent::new(root).with_reversibility(Reversibility::Partial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_loads() {
        let bundle = load_capacity_bundle().expect("bundle should load");
        assert_eq!(bundle.demand.records.len(), 10);
        assert_eq!(bundle.history.records.len(), 5);
        assert_eq!(bundle.capacity.records.len(), 4);
    }

    #[test]
    fn build_input_works() {
        let input = build_capacity_input(&SpikeConfig::default()).expect("input should build");
        assert_eq!(input.demand_forecasts.len(), 10);
        assert_eq!(input.teams.len(), 4);
    }
}
