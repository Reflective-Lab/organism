//! Experience-to-learning adapter.
//!
//! Bridges converge's experience events to organism's learning system.
//! After a converge engine run completes, the adapter:
//!
//! 1. Reads the final context (facts across all keys)
//! 2. Builds a `LearningEpisode` capturing what was predicted vs observed
//! 3. Extracts `Lesson`s from contradictions and coverage gaps
//! 4. Produces `PriorCalibration` updates for future planning runs
//!
//! Learning signals NEVER feed into authority — only into planning priors.

use converge_pack::{Context, ContextKey, Fact};
use uuid::Uuid;

use crate::{
    AdversarialContext, ErrorDimension, LearningEpisode, LearningSignal, Lesson,
    PredictionError, PriorCalibration, SignalKind,
};

/// Build a learning episode from a completed converge engine context.
///
/// The `intent_id` and `plan_id` tie this episode back to the organism
/// planning seed that started the run. The `subject` is the entity
/// being researched (e.g., company name in DD).
pub fn build_episode(
    intent_id: Uuid,
    plan_id: Uuid,
    subject: &str,
    ctx: &dyn Context,
) -> LearningEpisode {
    let hypotheses = ctx.get(ContextKey::Hypotheses);
    let evaluations = ctx.get(ContextKey::Evaluations);
    let proposals = ctx.get(ContextKey::Proposals);
    let signals = ctx.get(ContextKey::Signals);
    let strategies = ctx.get(ContextKey::Strategies);

    // Extract category coverage from hypotheses
    let categories = extract_categories(&hypotheses);
    let contradiction_count = evaluations
        .iter()
        .filter(|f| f.content.contains("contradiction"))
        .count();

    // Predicted outcome: the synthesis proposal if present
    let predicted_outcome = proposals
        .first()
        .map(|p| p.content.clone())
        .unwrap_or_else(|| format!("No synthesis produced for {subject}"));

    // Compute coverage error — how many expected categories were missing
    let expected_categories = [
        "product",
        "customers",
        "technology",
        "competition",
        "market",
        "financials",
    ];
    let covered = expected_categories
        .iter()
        .filter(|c| categories.contains(&(**c).to_string()))
        .count();
    let coverage_ratio = covered as f64 / expected_categories.len() as f64;

    let prediction_error = PredictionError {
        magnitude: 1.0 - coverage_ratio,
        dimensions: vec![
            ErrorDimension {
                name: "category_coverage".into(),
                predicted: 1.0,
                actual: coverage_ratio,
            },
            ErrorDimension {
                name: "contradiction_rate".into(),
                predicted: 0.0,
                actual: if hypotheses.is_empty() {
                    0.0
                } else {
                    contradiction_count as f64 / hypotheses.len() as f64
                },
            },
            ErrorDimension {
                name: "signal_to_hypothesis_ratio".into(),
                predicted: 0.5,
                actual: if signals.is_empty() {
                    0.0
                } else {
                    hypotheses.len() as f64 / signals.len() as f64
                },
            },
        ],
    };

    // Extract lessons from what happened
    let mut lessons = Vec::new();

    if coverage_ratio < 1.0 {
        let missing: Vec<&&str> = expected_categories
            .iter()
            .filter(|c| !categories.contains(&(**c).to_string()))
            .collect();
        lessons.push(Lesson {
            insight: format!(
                "Coverage gap: missing categories {}",
                missing.iter().map(|c| **c).collect::<Vec<_>>().join(", ")
            ),
            context: subject.to_string(),
            confidence: 0.9,
            planning_adjustment: "Add targeted strategies for missing categories in initial seed".into(),
        });
    }

    if contradiction_count > 0 {
        lessons.push(Lesson {
            insight: format!(
                "{contradiction_count} contradictions detected — sources disagree on key claims"
            ),
            context: subject.to_string(),
            confidence: 0.8,
            planning_adjustment: "Increase depth searches on contradicted topics in follow-up runs".into(),
        });
    }

    let strategy_count = strategies.len();
    let initial_strategies = 4; // standard DD seed
    if strategy_count > initial_strategies + 2 {
        lessons.push(Lesson {
            insight: format!(
                "Gap detector fired {} times — initial strategies were insufficient",
                strategy_count - initial_strategies
            ),
            context: subject.to_string(),
            confidence: 0.7,
            planning_adjustment: "Consider broader initial strategy seed for this domain".into(),
        });
    }

    // Adversarial context from contradictions
    let adversarial_signals: Vec<AdversarialContext> = evaluations
        .iter()
        .filter(|f| f.content.contains("contradiction"))
        .filter_map(|f| {
            let v: serde_json::Value = serde_json::from_str(&f.content).ok()?;
            Some(AdversarialContext {
                kind: "contradiction".into(),
                failed_assumption: v["description"]
                    .as_str()
                    .unwrap_or("sources disagree")
                    .to_string(),
                revision_summary: None,
            })
        })
        .collect();

    LearningEpisode {
        id: Uuid::new_v4(),
        intent_id,
        plan_id,
        predicted_outcome,
        actual_outcome: None, // filled by human feedback later
        prediction_error: Some(prediction_error),
        adversarial_signals,
        lessons,
    }
}

/// Extract learning signals from a completed run — lightweight feedback
/// that can be captured without waiting for human outcome reporting.
pub fn extract_signals(ctx: &dyn Context) -> Vec<LearningSignal> {
    let hypotheses = ctx.get(ContextKey::Hypotheses);
    let evaluations = ctx.get(ContextKey::Evaluations);
    let proposals = ctx.get(ContextKey::Proposals);

    let mut signals = Vec::new();

    if !proposals.is_empty() {
        signals.push(LearningSignal {
            kind: SignalKind::OutcomeMatchedExpectation,
            weight: 1.0,
            note: "Synthesis produced — convergence loop completed".into(),
        });
    } else {
        signals.push(LearningSignal {
            kind: SignalKind::OutcomeMissedExpectation,
            weight: 0.8,
            note: "No synthesis produced — hypotheses may not have stabilized".into(),
        });
    }

    let contradictions = evaluations
        .iter()
        .filter(|f| f.content.contains("contradiction"))
        .count();

    if contradictions > 0 {
        signals.push(LearningSignal {
            kind: SignalKind::AdversarialWarning,
            weight: 0.6,
            note: format!("{contradictions} contradictions detected"),
        });
    }

    if hypotheses.len() > 50 {
        signals.push(LearningSignal {
            kind: SignalKind::OutcomeBeatExpectation,
            weight: 0.5,
            note: format!("Rich evidence base: {} hypotheses extracted", hypotheses.len()),
        });
    }

    signals
}

/// Check whether the converge context contains infrastructure failure
/// constraints that should prevent prior calibration.
///
/// Returns `true` if the run was compromised by infra issues (credits
/// exhausted, provider unavailable, rate limited) — meaning the outcome
/// reflects infrastructure state, not research quality.
pub fn has_infra_failure(ctx: &dyn Context) -> bool {
    ctx.get(ContextKey::Constraints)
        .iter()
        .any(|f| {
            f.content.contains("\"is_infra_failure\":true")
        })
}

/// Produce prior calibrations from a learning episode.
/// These feed into future planning runs — NOT into authority.
///
/// **Important:** Call [`has_infra_failure`] first. Do not calibrate
/// priors from runs where infrastructure failed — the outcomes reflect
/// provider state, not research quality.
pub fn calibrate_priors(
    episode: &LearningEpisode,
    existing_priors: &[PriorCalibration],
) -> Vec<PriorCalibration> {
    let mut priors = Vec::new();

    if let Some(error) = &episode.prediction_error {
        for dim in &error.dimensions {
            // Find existing prior or create new
            let existing = existing_priors
                .iter()
                .find(|p| p.assumption_type == dim.name);

            let (prior_conf, evidence) = match existing {
                Some(p) => (p.posterior_confidence, p.evidence_count),
                None => (0.5, 0),
            };

            // Simple Bayesian-ish update: blend prior with observation
            let observation_weight = 1.0 / (evidence as f64 + 2.0);
            let posterior = prior_conf * (1.0 - observation_weight)
                + dim.actual * observation_weight;

            priors.push(PriorCalibration {
                assumption_type: dim.name.clone(),
                context: episode
                    .lessons
                    .first()
                    .map(|l| l.context.clone())
                    .unwrap_or_default(),
                prior_confidence: prior_conf,
                posterior_confidence: posterior,
                evidence_count: evidence + 1,
            });
        }
    }

    priors
}

fn extract_categories(hypotheses: &[Fact]) -> Vec<String> {
    hypotheses
        .iter()
        .filter_map(|f| {
            let v: serde_json::Value = serde_json::from_str(&f.content).ok()?;
            v["category"].as_str().map(String::from)
        })
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibrate_priors_updates_from_episode() {
        let episode = LearningEpisode {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            predicted_outcome: "test".into(),
            actual_outcome: None,
            prediction_error: Some(PredictionError {
                magnitude: 0.3,
                dimensions: vec![ErrorDimension {
                    name: "coverage".into(),
                    predicted: 1.0,
                    actual: 0.7,
                }],
            }),
            adversarial_signals: vec![],
            lessons: vec![Lesson {
                insight: "test".into(),
                context: "test-company".into(),
                confidence: 0.8,
                planning_adjustment: "adjust".into(),
            }],
        };

        let priors = calibrate_priors(&episode, &[]);
        assert_eq!(priors.len(), 1);
        assert_eq!(priors[0].assumption_type, "coverage");
        assert_eq!(priors[0].evidence_count, 1);
        // First observation: blends 0.5 prior with 0.7 actual
        assert!(priors[0].posterior_confidence > 0.5);
        assert!(priors[0].posterior_confidence < 0.7);
    }

    #[test]
    fn calibrate_priors_converges_with_evidence() {
        let dim = ErrorDimension {
            name: "ratio".into(),
            predicted: 0.5,
            actual: 0.8,
        };

        let episode = LearningEpisode {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            predicted_outcome: "test".into(),
            actual_outcome: None,
            prediction_error: Some(PredictionError {
                magnitude: 0.3,
                dimensions: vec![dim],
            }),
            adversarial_signals: vec![],
            lessons: vec![],
        };

        // Simulate 5 rounds of calibration with same observation
        let mut priors = vec![];
        for _ in 0..5 {
            priors = calibrate_priors(&episode, &priors);
        }

        // Should converge toward 0.8 (the actual)
        assert!(priors[0].posterior_confidence > 0.65);
        assert_eq!(priors[0].evidence_count, 5);
    }
}
