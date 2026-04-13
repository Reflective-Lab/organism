// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Simulation swarm — parallel stress-testing before the commit boundary.
//!
//! The simulation swarm runs plans through multiple dimensions before
//! they reach the converge engine's PromotionGate. Proposals enter as
//! candidates and emerge as probability estimates, not assertions.
//!
//! Simulation results are emitted as converge Facts (under appropriate
//! ContextKeys) so the engine can use them in convergence decisions.
//!
//! This mirrors how aircraft are designed, trading systems operate,
//! and chip designs are validated — not one validation step, but many
//! simulations in parallel.

use converge_core::Context;
use serde::{Deserialize, Serialize};

/// Result of running a simulation swarm on a candidate plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Which plan was simulated (references a Proposal ID).
    pub plan_id: String,
    /// Number of simulation runs.
    pub runs: u32,
    /// Results per simulation dimension.
    pub dimensions: Vec<DimensionResult>,
    /// Overall confidence in the plan (0.0 to 1.0).
    pub overall_confidence: f64,
    /// Whether the plan is recommended for commitment.
    pub recommendation: SimulationRecommendation,
}

/// Result for a single simulation dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionResult {
    pub dimension: SimulationDimension,
    pub passed: bool,
    pub confidence: f64,
    pub findings: Vec<String>,
}

/// The dimensions along which plans are simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationDimension {
    /// Will the plan achieve its stated outcome?
    Outcome,
    /// Will the plan stay within budget and resource envelope?
    Cost,
    /// Does the plan comply with all policies and authority boundaries?
    Policy,
    /// Are the causal claims in the plan defensible?
    Causal,
    /// Is the plan operationally feasible given current capacity?
    Operational,
}

/// Simulation swarm recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationRecommendation {
    /// Plan is recommended for commitment.
    Proceed,
    /// Plan has issues that should be addressed first.
    ProceedWithCaution,
    /// Plan is not recommended.
    DoNotProceed,
}

/// Trait for simulation dimension runners.
///
/// Implementations run against the converge Context to access
/// the current state when evaluating a plan.
pub trait SimulationRunner: Send + Sync {
    /// Which dimension this runner simulates.
    fn dimension(&self) -> SimulationDimension;

    /// Run simulation against a candidate plan in the given context.
    fn simulate(&self, plan_id: &str, ctx: &Context, runs: u32) -> DimensionResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_result_roundtrips() {
        let result = SimulationResult {
            plan_id: "proposal:plan-001".into(),
            runs: 100,
            dimensions: vec![DimensionResult {
                dimension: SimulationDimension::Cost,
                passed: true,
                confidence: 0.95,
                findings: vec!["Within 5% of budget envelope".into()],
            }],
            overall_confidence: 0.87,
            recommendation: SimulationRecommendation::Proceed,
        };

        let json = serde_json::to_string(&result).unwrap();
        let _: SimulationResult = serde_json::from_str(&json).unwrap();
    }
}
