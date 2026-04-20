//! Organizational learning.
//!
//! Planning priors are calibrated by execution outcomes. Adversarial firings
//! become labeled training signals. Strategy adapts based on feedback.
//!
//! Learning signals must NEVER feed directly into authority — only into the
//! priors that planning consults.
//!
//! Cycle: Intent → Plan → Execute → Observe → Learn → Calibrate priors.
//!
//! # Adapter
//!
//! The [`adapter`] module bridges converge experience events to organism
//! learning types. After an engine run, call
//! [`adapter::build_episode_from_run`] with the final context plus queried
//! `ExperienceEventEnvelope`s, then [`adapter::calibrate_priors`] to update
//! planning priors.

pub mod adapter;
pub mod prior_agent;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use prior_agent::PlanningPriorAgent;

// ── Learning Episode ───────────────────────────────────────────────

/// Full record of a planning-to-outcome episode. Links intent, plan,
/// predicted outcomes, governed business outcomes, engine run status,
/// prediction errors, adversarial signals, and extracted lessons.
/// Every field traces to converge Facts or run envelopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEpisode {
    pub id: Uuid,
    pub intent_id: Uuid,
    pub plan_id: Uuid,
    pub predicted_outcome: String,
    pub actual_outcome: Option<String>,
    pub run_status: Option<String>,
    pub prediction_error: Option<PredictionError>,
    pub adversarial_signals: Vec<AdversarialContext>,
    pub lessons: Vec<Lesson>,
}

// ── Prediction Error ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionError {
    pub magnitude: f64,
    pub dimensions: Vec<ErrorDimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDimension {
    pub name: String,
    pub predicted: f64,
    pub actual: f64,
}

// ── Lesson ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    pub insight: String,
    pub context: String,
    pub confidence: f64,
    pub planning_adjustment: String,
}

// ── Prior Calibration ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorCalibration {
    pub assumption_type: String,
    pub context: String,
    pub prior_confidence: f64,
    pub posterior_confidence: f64,
    pub evidence_count: u32,
}

// ── Adversarial Context (for cross-referencing) ────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialContext {
    pub kind: String,
    pub failed_assumption: String,
    pub revision_summary: Option<String>,
}

// ── Signal (simplified for quick capture) ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSignal {
    pub kind: SignalKind,
    pub weight: f32,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    OutcomeMatchedExpectation,
    OutcomeBeatExpectation,
    OutcomeMissedExpectation,
    AdversarialBlocker,
    AdversarialWarning,
}
