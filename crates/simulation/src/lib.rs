//! Simulation swarm.
//!
//! Parallel stress-testing of candidate plans before commit. Multiple
//! simulators run concurrently across five dimensions: outcome, cost,
//! policy, causal, operational. Each returns probability distributions,
//! not point estimates.
//!
//! Mirrors validation patterns from aircraft design, trading systems,
//! and chip design.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Simulation Result ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub plan_id: Uuid,
    pub runs: u32,
    pub dimensions: Vec<DimensionResult>,
    pub overall_confidence: f64,
    pub recommendation: SimulationRecommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionResult {
    pub dimension: SimulationDimension,
    pub passed: bool,
    pub confidence: f64,
    pub findings: Vec<String>,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationDimension {
    Outcome,
    Cost,
    Policy,
    Causal,
    Operational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationRecommendation {
    Proceed,
    ProceedWithCaution,
    DoNotProceed,
}

// ── Sample ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Sample {
    pub value: f64,
    pub probability: f64,
}

// ── Simulation Report (legacy compat) ──────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationReport {
    pub results: Vec<SimulationResult>,
}

// ── Simulator Trait ────────────────────────────────────────────────

pub trait SimulationRunner: Send + Sync {
    fn dimension(&self) -> SimulationDimension;
    fn simulate(&self, plan: &serde_json::Value) -> DimensionResult;
}
