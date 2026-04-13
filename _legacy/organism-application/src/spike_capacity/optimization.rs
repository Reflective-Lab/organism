// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Helpers for turning candidate plans into converge-optimization solves.

use std::thread;

use converge_optimization::gate::{GateDecision, ProblemSpec};
use converge_optimization::packs::Pack;
use converge_optimization::packs::capacity_planning::{
    CapacityPlanningOutput, CapacityPlanningPack, Team,
};

use crate::spike_capacity::consensus::{CandidatePlan, PlanAdjustment};
use crate::spike_capacity::scenario::{SpikeConfig, build_capacity_input, load_capacity_bundle};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeasiblePlanResult {
    pub plan_id: String,
    pub overall_fulfillment_ratio: f64,
    pub total_cost: f64,
    pub average_utilization: f64,
    pub teams_over_capacity: usize,
    pub unmet_demands: usize,
    pub gate_decision: String,
    pub gate_rationale: String,
    pub analytics_dataset_version: String,
    pub capacity_dataset_version: String,
    pub output: CapacityPlanningOutput,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum SimulationScenario {
    BaseCase,
    DemandSpike,
    AttritionShock,
    ExecutionRecovery,
}

impl SimulationScenario {
    #[must_use]
    pub fn all() -> [Self; 4] {
        [
            Self::BaseCase,
            Self::DemandSpike,
            Self::AttritionShock,
            Self::ExecutionRecovery,
        ]
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::BaseCase => "base_case",
            Self::DemandSpike => "demand_spike",
            Self::AttritionShock => "attrition_shock",
            Self::ExecutionRecovery => "execution_recovery",
        }
    }

    #[must_use]
    fn demand_multiplier(self) -> f64 {
        match self {
            Self::BaseCase => 1.0,
            Self::DemandSpike => 1.15,
            Self::AttritionShock => 1.0,
            Self::ExecutionRecovery => 0.95,
        }
    }

    #[must_use]
    fn capacity_multiplier(self) -> f64 {
        match self {
            Self::BaseCase => 1.0,
            Self::DemandSpike => 0.98,
            Self::AttritionShock => 0.88,
            Self::ExecutionRecovery => 1.05,
        }
    }

    #[must_use]
    fn budget_multiplier(self) -> f64 {
        match self {
            Self::BaseCase => 1.0,
            Self::DemandSpike => 1.0,
            Self::AttritionShock => 1.0,
            Self::ExecutionRecovery => 1.05,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimulationRun {
    pub plan_id: String,
    pub scenario: String,
    pub overall_fulfillment_ratio: f64,
    pub gate_decision: String,
    pub plausible: bool,
}

pub fn solve_candidate_plan(
    plan: &CandidatePlan,
    config: &SpikeConfig,
) -> Result<FeasiblePlanResult, String> {
    solve_candidate_plan_under_scenario(plan, config, SimulationScenario::BaseCase)
}

pub fn solve_candidate_plan_under_scenario(
    plan: &CandidatePlan,
    config: &SpikeConfig,
    scenario: SimulationScenario,
) -> Result<FeasiblePlanResult, String> {
    let mut input = build_capacity_input(config)?;
    apply_plan_adjustments(&mut input.teams, &plan.adjustments);
    apply_scenario_stress(&mut input, config, scenario);

    let pack = CapacityPlanningPack;
    let spec = ProblemSpec::builder(
        format!("spike-3-{}-{}", plan.plan_id, scenario.name()),
        "organism-application",
    )
    .objective(converge_optimization::gate::ObjectiveSpec::maximize(
        "fulfillment",
    ))
    .inputs(&input)
    .map_err(|e| e.to_string())?
    .seed(42)
    .build()
    .map_err(|e| e.to_string())?;

    let solve_result = pack.solve(&spec).map_err(|e| e.to_string())?;
    let invariant_results = pack
        .check_invariants(&solve_result.plan)
        .map_err(|e| e.to_string())?;
    let gate = pack.evaluate_gate(&solve_result.plan, &invariant_results);
    let output: CapacityPlanningOutput = solve_result.plan.plan_as().map_err(|e| e.to_string())?;
    let bundle = load_capacity_bundle()?;

    Ok(FeasiblePlanResult {
        plan_id: plan.plan_id.clone(),
        overall_fulfillment_ratio: output.summary.overall_fulfillment_ratio,
        total_cost: output.summary.total_cost,
        average_utilization: output.summary.average_utilization,
        teams_over_capacity: output.summary.teams_over_capacity,
        unmet_demands: output.summary.unmet_demands,
        gate_decision: gate_decision_name(gate.decision).to_string(),
        gate_rationale: gate.rationale,
        analytics_dataset_version: bundle.history.dataset_version,
        capacity_dataset_version: bundle.capacity.dataset_version,
        output,
    })
}

pub fn run_parallel_simulations(
    plans: &[CandidatePlan],
    config: &SpikeConfig,
) -> Result<Vec<SimulationRun>, String> {
    thread::scope(|scope| {
        let mut handles = Vec::new();

        for plan in plans {
            let plan = plan.clone();
            let config = config.clone();
            handles.push(scope.spawn(move || -> Result<Vec<SimulationRun>, String> {
                let mut runs = Vec::new();
                for scenario in SimulationScenario::all() {
                    let result = solve_candidate_plan_under_scenario(&plan, &config, scenario)?;
                    let gate_decision = result.gate_decision.clone();
                    runs.push(SimulationRun {
                        plan_id: plan.plan_id.clone(),
                        scenario: scenario.name().to_string(),
                        overall_fulfillment_ratio: result.overall_fulfillment_ratio,
                        gate_decision: gate_decision.clone(),
                        plausible: result.overall_fulfillment_ratio
                            >= config.min_overall_fulfillment * 0.95
                            && result.teams_over_capacity == 0
                            && gate_decision != "reject",
                    });
                }
                Ok(runs)
            }));
        }

        let mut all_runs = Vec::new();
        for handle in handles {
            let runs = handle
                .join()
                .map_err(|_| "simulation thread panicked".to_string())??;
            all_runs.extend(runs);
        }
        Ok(all_runs)
    })
}

fn apply_plan_adjustments(teams: &mut [Team], adjustments: &[PlanAdjustment]) {
    for adjustment in adjustments {
        if let Some(team) = teams.iter_mut().find(|team| team.id == adjustment.team_id) {
            team.available_capacity += adjustment.added_capacity;
            team.headcount += adjustment.added_headcount;
            for skill in &adjustment.extra_skills {
                if !team.skills.contains(skill) {
                    team.skills.push(skill.clone());
                }
            }
        }
    }
}

fn apply_scenario_stress(
    input: &mut converge_optimization::packs::capacity_planning::CapacityPlanningInput,
    config: &SpikeConfig,
    scenario: SimulationScenario,
) {
    for forecast in &mut input.demand_forecasts {
        forecast.demand_units *= scenario.demand_multiplier();
    }

    for team in &mut input.teams {
        team.available_capacity *= scenario.capacity_multiplier();
    }

    input.constraints.max_budget = Some(config.planning_budget * scenario.budget_multiplier());
}

fn gate_decision_name(decision: GateDecision) -> &'static str {
    match decision {
        GateDecision::Promote => "promote",
        GateDecision::Reject => "reject",
        GateDecision::Escalate => "escalate",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spike_capacity::consensus::CandidatePlan;
    use organism_core::planning::{CostEstimate, Impact, PlanAnnotation, ReasoningSystem};

    #[test]
    fn solve_balanced_plan() {
        let plan = CandidatePlan {
            plan_id: "balanced_growth".into(),
            name: "Balanced Growth".into(),
            strategic_thesis: "Balance bottlenecks".into(),
            adjustments: vec![
                PlanAdjustment::new("backend_platform", 70.0, 2, Vec::<String>::new()),
                PlanAdjustment::new("data_ml", 90.0, 3, Vec::<String>::new()),
                PlanAdjustment::new("experience_qa", 30.0, 1, Vec::<String>::new()),
            ],
            annotation: PlanAnnotation {
                description: "Balanced".into(),
                expected_impact: Impact {
                    description: "Higher fulfillment".into(),
                    confidence: 0.8,
                },
                cost_estimate: CostEstimate {
                    compute_cost: 0.0,
                    time_cost: Some("one quarter".into()),
                    unit: "USD".into(),
                },
                risks: vec![],
                contributors: vec![ReasoningSystem::ConstraintSolver],
            },
            incremental_spend: 280_000.0,
            planning_focus: vec!["backend".into(), "data".into()],
        };

        let result = solve_candidate_plan(&plan, &SpikeConfig::default()).unwrap();
        assert!(result.overall_fulfillment_ratio > 0.9);
    }
}
