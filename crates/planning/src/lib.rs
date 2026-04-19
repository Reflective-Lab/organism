//! Planning layer.
//!
//! Multi-model collaborative planning. A huddle runs several reasoning
//! systems in parallel, then the debate loop refines candidate plans
//! before they're handed to the simulation swarm.
//!
//! Plans flow through converge's PromotionGate like any other proposal.
//! No special bypass; standard convergence pipeline applies.

pub mod charter_derivation;
pub mod collaboration;
pub mod dd;
pub mod debate;
pub mod huddle;
pub mod kb;
pub mod shape_hypothesis;
pub mod suggestor;
pub mod topology_transition;

use organism_intent::IntentPacket;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use collaboration::{
    CollaborationCharter, CollaborationDiscipline, CollaborationMember, CollaborationRole,
    CollaborationTopology, CollaborationValidationError, ConsensusRule, TeamFormation,
    TeamFormationMode, TurnCadence,
};

// ── Plan ───────────────────────────────────────────────────────────

/// A candidate plan produced by reasoning. Plans are *proposals*, not
/// commitments — authority is recomputed at the commit boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub intent: Uuid,
    pub steps: Vec<PlanStep>,
    pub rationale: String,
    pub annotation: PlanAnnotation,
    pub contributor: ReasoningSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: String,
    pub expected_effect: String,
}

impl Plan {
    pub fn new(intent: &IntentPacket, rationale: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            intent: intent.id,
            steps: Vec::new(),
            rationale: rationale.into(),
            annotation: PlanAnnotation::default(),
            contributor: ReasoningSystem::LlmReasoning,
        }
    }
}

// ── Plan Annotation ────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanAnnotation {
    pub impacts: Vec<Impact>,
    pub costs: Vec<CostEstimate>,
    pub risks: Vec<Risk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impact {
    pub description: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub description: String,
    pub compute_cost: f64,
    pub time_cost: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub description: String,
    pub likelihood: Likelihood,
    pub impact: RiskImpact,
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Likelihood {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskImpact {
    Low,
    Medium,
    High,
    Critical,
}

// ── Reasoning Systems ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSystem {
    LlmReasoning,
    ConstraintSolver,
    MlPrediction,
    CausalAnalysis,
    CostEstimation,
    DomainModel,
}

// ── Huddle ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanContribution {
    pub system: ReasoningSystem,
    pub suggestions: Vec<String>,
    pub constraints: Vec<String>,
    pub risks: Vec<Risk>,
}

/// A reasoning capability participating in a huddle.
#[async_trait::async_trait]
pub trait Reasoner: Send + Sync {
    fn name(&self) -> &str;
    fn system_type(&self) -> ReasoningSystem;
    async fn propose(&self, intent: &IntentPacket) -> anyhow::Result<Plan>;
    fn contribute(&self, context: &serde_json::Value) -> PlanContribution;
}

// ── Plan Bundle (debate output) ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBundle {
    pub plans: Vec<Plan>,
    pub debate_rounds: u32,
}

// ── Hypothesis Tracking ───────────────────────────────────────────

/// Lifecycle state of a tracked hypothesis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum HypothesisOutcome {
    Open,
    Confirmed,
    Falsified { contradiction_id: String },
    Superseded,
    Unresolved,
}

/// A hypothesis tracked across convergence cycles.
///
/// Created from a `ContextKey::Hypotheses` fact. The tracker records
/// when it was first seen, its confidence trajectory, and its final outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedHypothesis {
    pub fact_id: String,
    pub domain: String,
    pub claim: String,
    pub confidence: f64,
    pub formed_cycle: u32,
    pub resolved_cycle: Option<u32>,
    pub outcome: HypothesisOutcome,
}
