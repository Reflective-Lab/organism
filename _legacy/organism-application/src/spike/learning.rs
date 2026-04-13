// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Post-convergence learning extraction for the Budget Approval Decision spike.
//!
//! Extracts a `LearningEpisode` from the converged context, capturing
//! adversarial signals, prediction errors, and lessons.

use converge_core::{Context, ContextKey};
use organism_core::adversarial::{AdversarialSignal, Challenge};
use organism_core::intent::OrganismIntent;
use organism_core::learning::{ErrorDimension, LearningEpisode, Lesson, PredictionError};

/// Extract a learning episode from the converged context.
pub fn extract_learning_episode(intent: &OrganismIntent, ctx: &Context) -> LearningEpisode {
    let adversarial_signals = extract_adversarial_signals(ctx);
    let lessons = extract_lessons(ctx, &adversarial_signals);

    // Find the decision for predicted outcome
    let decision = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|e| e.id.starts_with("decision:"))
        .map_or_else(|| "no decision".into(), |e| e.content.clone());

    LearningEpisode {
        intent_id: intent.id().0.as_str().to_string(),
        plan_fact_id: ctx
            .get(ContextKey::Strategies)
            .iter()
            .find(|s| s.id.starts_with("revised:"))
            .or_else(|| ctx.get(ContextKey::Strategies).first())
            .map_or_else(|| "none".into(), |s| s.id.clone()),
        predicted_outcome: decision,
        actual_outcome: "pending_execution".into(),
        prediction_error: PredictionError {
            magnitude: 0.0, // Not yet known — actual outcome pending
            dimensions: vec![ErrorDimension {
                name: "roi_estimate".into(),
                predicted: 1.8,
                actual: 0.0, // Will be filled post-execution
            }],
        },
        adversarial_signals,
        lessons,
    }
}

/// Extract adversarial signals from constraint facts.
fn extract_adversarial_signals(ctx: &Context) -> Vec<AdversarialSignal> {
    let constraints = ctx.get(ContextKey::Constraints);
    let strategies = ctx.get(ContextKey::Strategies);

    constraints
        .iter()
        .filter(|c| c.id.starts_with("challenge:"))
        .filter_map(|c| {
            let challenge: Challenge = serde_json::from_str(&c.content).ok()?;

            // Find the revision that addressed this challenge
            let revision_summary = strategies
                .iter()
                .find(|s| s.id.starts_with("revised:"))
                .map_or_else(|| "no revision".into(), |s| s.content.clone());

            Some(AdversarialSignal {
                skepticism: challenge.skepticism,
                assumption_type: challenge.target.clone(),
                failure_context: "budget_approval_q2_linkedin".into(),
                revision_summary,
                challenge,
            })
        })
        .collect()
}

/// Extract lessons from the adversarial signals.
fn extract_lessons(ctx: &Context, signals: &[AdversarialSignal]) -> Vec<Lesson> {
    let mut lessons = Vec::new();

    for signal in signals {
        lessons.push(Lesson {
            insight: format!(
                "Challenge on '{}': {}",
                signal.challenge.target, signal.challenge.description
            ),
            context: "budget_approval_linkedin_q2".into(),
            confidence: 0.8,
            planning_adjustment: signal
                .challenge
                .suggestion
                .clone()
                .unwrap_or_else(|| "Review ROI assumptions".into()),
        });
    }

    // Add a lesson from the decision outcome
    let decision = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|e| e.id.starts_with("decision:"));

    if let Some(d) = decision {
        if d.content.contains("approve_phased") {
            lessons.push(Lesson {
                insight:
                    "LinkedIn ROI assumptions were 2x too optimistic — phased approach adopted"
                        .into(),
                context: "linkedin_b2b_marketing".into(),
                confidence: 0.85,
                planning_adjustment: "Use 1.8x ROI baseline for LinkedIn B2B campaigns".into(),
            });
        }
    }

    lessons
}

#[cfg(test)]
mod tests {
    use crate::spike::run_budget_approval;
    use crate::spike::scenario::test_budget_intent;

    #[test]
    fn learning_episode_has_intent_id() {
        let intent = test_budget_intent(50_000);
        let (_, episode) = run_budget_approval(&intent).expect("should converge");
        assert_eq!(episode.intent_id, "budget-approval-001");
    }

    #[test]
    fn learning_episode_has_lessons() {
        let intent = test_budget_intent(50_000);
        let (_, episode) = run_budget_approval(&intent).expect("should converge");
        assert!(
            !episode.lessons.is_empty(),
            "expected lessons from adversarial signals"
        );
    }
}
