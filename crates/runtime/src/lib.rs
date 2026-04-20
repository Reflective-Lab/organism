//! # Organism Runtime
//!
//! The formation guru. Given an intent, assembles teams of heterogeneous
//! agents and runs them in Converge Engine instances.
//!
//! There is ONE model: everything is a Suggestor. Adversarial review,
//! simulation, planning, policy, optimization — all participate in the
//! same convergence loop. No side-car pipelines.
//!
//! ```text
//! Intent → Admit → Form (pick Suggestors) → Engine.run() → Evaluate → Learn
//!                    ↑                                          ↓
//!                    └──── reform if needed ────────────────────┘
//! ```

pub mod collaboration;
pub mod formation;
pub mod readiness;
pub mod registry;

pub use collaboration::{
    CollaborationParticipant, CollaborationRunner, CollaborationRunnerError, TransitionRecord,
};
pub use formation::{Formation, FormationError, FormationResult, Seed};
pub use organism_pack::{
    CapabilityRequirement, DeclarativeBinding, IntentBinding, IntentResolver, PackRequirement,
    ResolutionLevel, ResolutionTrace,
};
pub use readiness::{
    BudgetProbe, CredentialProbe, GapSeverity, PackProbe, ReadinessConfirmation, ReadinessGap,
    ReadinessItem, ReadinessProbe, ReadinessReport, ResourceKind, check as check_readiness,
};
pub use registry::{RegisteredCapability, RegisteredPack, Registry, StructuralResolver};

use organism_intent::admission::{self, Admission};
use organism_pack::IntentPacket;

/// Outcome of the full organism pipeline.
#[derive(Debug)]
pub struct OrganismResult {
    /// The formation that produced the winning result.
    pub winning_formation: String,
    /// Converge result from the winning run.
    pub converge_result: converge_kernel::ConvergeResult,
}

/// Why the pipeline rejected an intent or formation.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("admission rejected: {0}")]
    Rejected(String),
    #[error("all formations failed: {0}")]
    AllFormationsFailed(String),
    #[error("formation error: {0}")]
    Formation(#[from] FormationError),
}

/// The formation guru.
///
/// Organism's runtime does exactly three things:
/// 1. Quick admission gate (is the intent even valid?)
/// 2. Run formations in Converge (each is a team of heterogeneous Suggestors)
/// 3. Pick the winner
///
/// Everything else — adversarial review, simulation, planning, policy checks —
/// happens INSIDE the formation as Suggestors in the convergence loop.
pub struct Runtime;

impl Runtime {
    pub fn new() -> Self {
        Self
    }

    /// Drive an intent through the pipeline.
    ///
    /// The caller is responsible for assembling formations (teams of Suggestors).
    /// That's the formation-guru logic — deciding which agents to include based
    /// on the intent's characteristics, available capabilities, and learned priors.
    ///
    /// Each formation may include any mix of:
    /// - LLM reasoning agents
    /// - Optimization solvers
    /// - Policy gates
    /// - Analytics/ML agents
    /// - Adversarial skeptics
    /// - Domain-specific pack agents
    ///
    /// All participate through the same `Suggestor` trait. Same contract,
    /// same governance, same convergence loop.
    pub async fn handle(
        &self,
        intent: IntentPacket,
        formations: Vec<Formation>,
    ) -> Result<OrganismResult, PipelineError> {
        // 1. Admission — the one imperative check that stays outside the loop.
        //    Is the intent structurally valid? Not expired? Not empty?
        match admission::admit(&intent) {
            Admission::Admit => {}
            Admission::Reject(err) => {
                return Err(PipelineError::Rejected(err.to_string()));
            }
        }

        // 2. Run formations (concurrently in the future; sequential for now).
        //    Each formation is a complete Converge Engine run with its own
        //    team of Suggestors. Adversarial agents, simulators, planners —
        //    they're all in there, converging together.
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for formation in formations {
            match formation.run().await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(e.to_string()),
            }
        }

        if results.is_empty() {
            return Err(PipelineError::AllFormationsFailed(errors.join("; ")));
        }

        // 3. Pick the winner.
        //    Future: evaluate competing results via learned quality metrics,
        //    convergence quality, cycle count, fact coverage.
        let winner = results.into_iter().next().unwrap();

        Ok(OrganismResult {
            winning_formation: winner.label,
            converge_result: winner.converge_result,
        })
    }
}
