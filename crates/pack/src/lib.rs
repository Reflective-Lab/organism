//! # Organism Pack
//!
//! The public contract for Organism's planning loop.
//! One import — the full pipeline from intent to learning.
//!
//! ```text
//! IntentPacket → Admission → Plan → Challenge → Simulate → Learn → Commit
//! ```
//!
//! # Quick start
//!
//! ```rust,ignore
//! use organism_pack::*;
//!
//! // 1. Create an intent
//! let intent = IntentPacket::new("Approve $2,500 expense", expires);
//!
//! // 2. Check admission (4 dimensions)
//! let admission = my_controller.evaluate(&intent);
//!
//! // 3. Plan (multi-model huddle)
//! let plan = Plan::new(&intent, "route to 3 approvers");
//!
//! // 4. Challenge (5 skepticism kinds)
//! let challenge = Challenge::new(
//!     SkepticismKind::EconomicSkepticism,
//!     plan.id,
//!     "entertainment spend is high",
//!     Severity::Warning,
//! );
//!
//! // 5. Simulate (5 dimensions)
//! let result = DimensionResult {
//!     dimension: SimulationDimension::Cost,
//!     passed: true,
//!     confidence: 0.95,
//!     findings: vec!["within budget".into()],
//!     samples: vec![],
//! };
//!
//! // 6. Learn from outcomes
//! let lesson = Lesson {
//!     insight: "score 0.88 → approved".into(),
//!     context: "expense approval".into(),
//!     confidence: 0.9,
//!     planning_adjustment: "none".into(),
//! };
//! ```

// ── Intent ─────────────────────────────────────────────────────────
// The input: what the organization wants to achieve.

pub use organism_intent::{
    // Admission
    AdmissionController,
    AdmissionResult,
    ExpiryAction,
    FeasibilityAssessment,
    FeasibilityDimension,
    FeasibilityKind,
    ForbiddenAction,
    IntentError,
    IntentNode,
    IntentPacket,
    Reversibility,
    // Resolution
    resolution::{
        CapabilityRequirement, DeclarativeBinding, IntentBinding, IntentResolver, PackRequirement,
        ResolutionLevel, ResolutionTrace,
    },
};

// ── Planning ───────────────────────────────────────────────────────
// How the system reasons about the intent.

pub use organism_planning::{
    CostEstimate, Impact, Likelihood, Plan, PlanAnnotation, PlanBundle, PlanContribution, PlanStep,
    Reasoner, ReasoningSystem, Risk, RiskImpact,
};

// ── Adversarial ────────────────────────────────────────────────────
// Institutionalized disagreement before commit.

pub use organism_adversarial::{
    AdversarialSignal, Challenge, Finding, Severity, Skeptic, SkepticismKind,
};

// ── Simulation ─────────────────────────────────────────────────────
// Parallel stress-testing of candidate plans.

pub use organism_simulation::{
    DimensionResult, Sample, SimulationDimension, SimulationRecommendation, SimulationReport,
    SimulationResult, SimulationRunner,
};

// ── Learning ───────────────────────────────────────────────────────
// Calibrate priors from execution outcomes.

pub use organism_learning::{
    AdversarialContext, ErrorDimension, LearningEpisode, LearningSignal, Lesson, PredictionError,
    PriorCalibration, SignalKind,
};
