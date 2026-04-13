// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Organizational learning — captures execution outcomes as training signals.
//!
//! The organism runtime learns from execution:
//!
//! 1. Intent → Plan → Execute → Observe outcome
//! 2. Compare predicted outcome with actual
//! 3. Update planning priors
//! 4. Calibrate adversarial agent sensitivity
//! 5. Feed back into next planning cycle
//!
//! Learning episodes reference converge-core types (IntentId, FactId)
//! for traceability. Every lesson traces back to the facts that produced it.

use serde::{Deserialize, Serialize};

use crate::adversarial::AdversarialSignal;

/// A learning episode — what was planned vs what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningEpisode {
    /// The intent that was executed (string form of converge-core IntentId).
    pub intent_id: String,
    /// The plan that was committed (string form of converge-core FactId).
    pub plan_fact_id: String,
    /// What was predicted.
    pub predicted_outcome: String,
    /// What actually happened.
    pub actual_outcome: String,
    /// The delta between prediction and reality.
    pub prediction_error: PredictionError,
    /// Adversarial signals generated during planning.
    pub adversarial_signals: Vec<AdversarialSignal>,
    /// Lessons extracted from this episode.
    pub lessons: Vec<Lesson>,
}

/// The difference between what was predicted and what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionError {
    /// Magnitude of error (0.0 = perfect, 1.0 = completely wrong).
    pub magnitude: f64,
    /// Which dimensions were most wrong.
    pub dimensions: Vec<ErrorDimension>,
}

/// A dimension along which prediction diverged from reality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDimension {
    pub name: String,
    pub predicted: f64,
    pub actual: f64,
}

/// A lesson extracted from a learning episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lesson {
    /// What was learned.
    pub insight: String,
    /// The context in which this lesson applies.
    pub context: String,
    /// Confidence in this lesson (0.0 to 1.0).
    pub confidence: f64,
    /// How this should affect future planning.
    pub planning_adjustment: String,
}

/// A calibration update for planning priors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorCalibration {
    /// Which assumption type is being calibrated.
    pub assumption_type: String,
    /// The context in which the calibration applies.
    pub context: String,
    /// Previous confidence in this assumption type.
    pub prior_confidence: f64,
    /// Updated confidence after learning.
    pub posterior_confidence: f64,
    /// Evidence count supporting this calibration.
    pub evidence_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learning_episode_roundtrips() {
        let episode = LearningEpisode {
            intent_id: "intent-001".into(),
            plan_fact_id: "fact:plan-001".into(),
            predicted_outcome: "15% revenue increase".into(),
            actual_outcome: "8% revenue increase".into(),
            prediction_error: PredictionError {
                magnitude: 0.47,
                dimensions: vec![ErrorDimension {
                    name: "revenue_growth".into(),
                    predicted: 0.15,
                    actual: 0.08,
                }],
            },
            adversarial_signals: vec![],
            lessons: vec![Lesson {
                insight: "Nordic enterprise sales cycles are 2x longer".into(),
                context: "nordic_enterprise_expansion".into(),
                confidence: 0.8,
                planning_adjustment: "Double time estimates for Nordic deals".into(),
            }],
        };

        let json = serde_json::to_string(&episode).unwrap();
        let _: LearningEpisode = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn prior_calibration_roundtrips() {
        let cal = PriorCalibration {
            assumption_type: "conversion_rate".into(),
            context: "enterprise_saas".into(),
            prior_confidence: 0.8,
            posterior_confidence: 0.5,
            evidence_count: 12,
        };

        let json = serde_json::to_string(&cal).unwrap();
        let _: PriorCalibration = serde_json::from_str(&json).unwrap();
    }
}
