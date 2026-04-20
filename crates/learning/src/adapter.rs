//! Experience-to-learning adapter.
//!
//! Bridges converge's experience events to organism's learning system.
//! After a converge engine run completes, the adapter:
//!
//! 1. Reads the final context (facts across all keys)
//! 2. Builds a `LearningEpisode` capturing predicted vs governed business
//!    outcomes, plus the engine run status
//! 3. Reads the captured `ExperienceEventEnvelope`s for terminal outcomes
//!    and budget/audit signals
//! 4. Extracts `Lesson`s from contradictions and coverage gaps
//! 5. Produces `PriorCalibration` updates for future planning runs
//!
//! Learning signals NEVER feed into authority — only into planning priors.

use converge_kernel::{ExperienceEvent, ExperienceEventEnvelope};
use converge_pack::{Context, ContextKey, Fact};
use uuid::Uuid;

use crate::{
    AdversarialContext, ErrorDimension, LearningEpisode, LearningSignal, Lesson, PredictionError,
    PriorCalibration, SignalKind,
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
    build_episode_from_run(intent_id, plan_id, subject, ctx, &[])
}

/// Build a learning episode from a completed run using both context and
/// queried experience events from the Converge `ExperienceStore`.
#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
pub fn build_episode_from_run(
    intent_id: Uuid,
    plan_id: Uuid,
    subject: &str,
    ctx: &dyn Context,
    events: &[ExperienceEventEnvelope],
) -> LearningEpisode {
    let hypotheses = ctx.get(ContextKey::Hypotheses);
    let evaluations = ctx.get(ContextKey::Evaluations);
    let proposals = ctx.get(ContextKey::Proposals);
    let signals = ctx.get(ContextKey::Signals);
    let strategies = ctx.get(ContextKey::Strategies);

    // Extract category coverage from hypotheses
    let categories = extract_categories(hypotheses);
    let contradiction_count = evaluations
        .iter()
        .filter(|f| f.content.contains("contradiction"))
        .count();

    // Predicted outcome: the synthesis proposal if present.
    // Actual outcome: the final governed business result that survived promotion.
    // Run status: the terminal engine outcome captured in experience envelopes.
    let actual_outcome = latest_governed_outcome(proposals);
    let predicted_outcome = actual_outcome
        .clone()
        .unwrap_or_else(|| format!("No synthesis produced for {subject}"));
    let run_status = latest_run_status(events);

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
            planning_adjustment: "Add targeted strategies for missing categories in initial seed"
                .into(),
        });
    }

    if contradiction_count > 0 {
        lessons.push(Lesson {
            insight: format!(
                "{contradiction_count} contradictions detected — sources disagree on key claims"
            ),
            context: subject.to_string(),
            confidence: 0.8,
            planning_adjustment: "Increase depth searches on contradicted topics in follow-up runs"
                .into(),
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

    if let Some(outcome) = latest_outcome_event(events)
        && !outcome.passed
    {
        lessons.push(Lesson {
            insight: format!(
                "Run ended without a passing outcome{}",
                outcome
                    .stop_reason
                    .as_deref()
                    .map_or(String::new(), |reason| format!(" ({reason})"))
            ),
            context: subject.to_string(),
            confidence: 0.75,
            planning_adjustment:
                "Tighten the initial plan or widen search budget before re-running".into(),
        });
    }

    let budget_blocks = budget_exceeded_count(events);
    if budget_blocks > 0 {
        lessons.push(Lesson {
            insight: format!(
                "{budget_blocks} budget guard(s) fired during the run — the search loop hit engine limits"
            ),
            context: subject.to_string(),
            confidence: 0.85,
            planning_adjustment:
                "Seed fewer low-value branches or raise the explicit engine budget for this domain"
                    .into(),
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
        actual_outcome,
        run_status,
        prediction_error: Some(prediction_error),
        adversarial_signals,
        lessons,
    }
}

/// Extract learning signals from a completed run — lightweight feedback
/// that can be captured without waiting for human outcome reporting.
pub fn extract_signals(ctx: &dyn Context) -> Vec<LearningSignal> {
    extract_signals_from_run(ctx, &[])
}

/// Extract learning signals from a completed run using both context and
/// captured experience events.
pub fn extract_signals_from_run(
    ctx: &dyn Context,
    events: &[ExperienceEventEnvelope],
) -> Vec<LearningSignal> {
    let hypotheses = ctx.get(ContextKey::Hypotheses);
    let evaluations = ctx.get(ContextKey::Evaluations);
    let proposals = ctx.get(ContextKey::Proposals);

    let mut signals = Vec::new();

    if let Some(outcome) = latest_outcome_event(events) {
        signals.push(LearningSignal {
            kind: if outcome.passed {
                SignalKind::OutcomeMatchedExpectation
            } else {
                SignalKind::OutcomeMissedExpectation
            },
            weight: if outcome.passed { 1.0 } else { 0.9 },
            note: outcome_signal_note(outcome),
        });
    } else if !proposals.is_empty() {
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
            note: format!(
                "Rich evidence base: {} hypotheses extracted",
                hypotheses.len()
            ),
        });
    }

    if budget_exceeded_count(events) > 0 {
        signals.push(LearningSignal {
            kind: SignalKind::AdversarialBlocker,
            weight: 1.0,
            note: format!(
                "{} budget guard(s) fired during the run",
                budget_exceeded_count(events)
            ),
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
        .any(|f| f.content.contains("\"is_infra_failure\":true"))
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
            let observation_weight = 1.0 / (f64::from(evidence) + 2.0);
            let posterior =
                prior_conf * (1.0 - observation_weight) + dim.actual * observation_weight;

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

#[derive(Debug, Clone, Copy)]
struct OutcomeEventView<'a> {
    passed: bool,
    stop_reason: &'a Option<String>,
    backend: &'a Option<String>,
}

fn latest_outcome_event(events: &[ExperienceEventEnvelope]) -> Option<OutcomeEventView<'_>> {
    events.iter().rev().find_map(|event| {
        if let ExperienceEvent::OutcomeRecorded {
            passed,
            stop_reason,
            backend,
            ..
        } = &event.event
        {
            Some(OutcomeEventView {
                passed: *passed,
                stop_reason,
                backend,
            })
        } else {
            None
        }
    })
}

fn latest_governed_outcome(proposals: &[Fact]) -> Option<String> {
    proposals.last().map(|proposal| proposal.content.clone())
}

fn latest_run_status(events: &[ExperienceEventEnvelope]) -> Option<String> {
    latest_outcome_event(events).map(|outcome| {
        let status = if outcome.passed { "passed" } else { "failed" };
        let backend = outcome.backend.as_deref().unwrap_or("unknown-backend");
        let reason = outcome
            .stop_reason
            .as_deref()
            .unwrap_or("no stop reason recorded");
        format!("Run {status} via {backend} ({reason})")
    })
}

fn outcome_signal_note(outcome: OutcomeEventView<'_>) -> String {
    let status = if outcome.passed {
        "Outcome recorded as passing"
    } else {
        "Outcome recorded as failing"
    };
    match outcome.stop_reason.as_deref() {
        Some(reason) => format!("{status}: {reason}"),
        None => status.to_string(),
    }
}

fn budget_exceeded_count(events: &[ExperienceEventEnvelope]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event.event, ExperienceEvent::BudgetExceeded { .. }))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{Context, Engine, ExperienceEvent, ExperienceEventEnvelope};
    use converge_pack::ContextKey;
    use std::collections::HashMap;

    fn make_outcome_event(passed: bool, stop_reason: &str) -> ExperienceEventEnvelope {
        ExperienceEventEnvelope::new(
            "evt-outcome",
            ExperienceEvent::OutcomeRecorded {
                chain_id: "dd:test".into(),
                step: converge_kernel::DecisionStep::Planning,
                passed,
                stop_reason: Some(stop_reason.into()),
                latency_ms: None,
                tokens: None,
                cost_microdollars: None,
                backend: Some("converge-engine".into()),
                metadata: HashMap::default(),
            },
        )
    }

    fn promoted_context(entries: &[(ContextKey, &str, &str)]) -> Context {
        let mut ctx = Context::new();
        for (key, id, content) in entries {
            ctx.add_input(*key, *id, *content)
                .expect("should stage test input");
        }
        tokio::runtime::Runtime::new()
            .expect("should create runtime")
            .block_on(Engine::new().run(ctx))
            .expect("engine run should promote staged input")
            .context
    }

    #[test]
    fn calibrate_priors_updates_from_episode() {
        let episode = LearningEpisode {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            predicted_outcome: "test".into(),
            actual_outcome: None,
            run_status: None,
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
            run_status: None,
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

    #[test]
    fn build_episode_from_run_tracks_run_status_without_business_outcome() {
        let ctx = converge_kernel::Context::new();
        let episode = build_episode_from_run(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Acme",
            &ctx,
            &[make_outcome_event(false, "budget_exhausted")],
        );

        assert_eq!(episode.actual_outcome, None);
        assert_eq!(
            episode.run_status.as_deref(),
            Some("Run failed via converge-engine (budget_exhausted)")
        );
        assert!(episode.lessons.iter().any(|lesson| {
            lesson
                .insight
                .contains("Run ended without a passing outcome")
        }));
    }

    #[test]
    fn build_episode_from_run_uses_governed_outcome_for_actual_outcome() {
        let ctx = promoted_context(&[(
            ContextKey::Proposals,
            "proposal-1",
            r#"{"summary":"Acme is attractive","recommendation":"Proceed"}"#,
        )]);
        let episode = build_episode_from_run(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Acme",
            &ctx,
            &[make_outcome_event(true, "converged")],
        );

        assert_eq!(
            episode.actual_outcome.as_deref(),
            Some(r#"{"summary":"Acme is attractive","recommendation":"Proceed"}"#)
        );
        assert_eq!(
            episode.predicted_outcome,
            episode.actual_outcome.clone().unwrap()
        );
        assert_eq!(
            episode.run_status.as_deref(),
            Some("Run passed via converge-engine (converged)")
        );
    }

    #[test]
    fn extract_signals_from_run_prefers_recorded_outcome() {
        let ctx = converge_kernel::Context::new();
        let signals = extract_signals_from_run(&ctx, &[make_outcome_event(true, "converged")]);

        assert!(
            signals
                .iter()
                .any(|signal| matches!(signal.kind, SignalKind::OutcomeMatchedExpectation))
        );
        assert!(
            signals
                .iter()
                .any(|signal| signal.note.contains("converged"))
        );
    }

    // ── Negative & edge case tests ────────────────────────────────

    #[test]
    fn build_episode_empty_context_no_events() {
        let ctx = converge_kernel::Context::new();
        let episode = build_episode_from_run(Uuid::new_v4(), Uuid::new_v4(), "EmptyCo", &ctx, &[]);

        assert!(episode.actual_outcome.is_none());
        assert!(episode.run_status.is_none());
        assert!(episode.predicted_outcome.contains("No synthesis produced"));
        assert!(episode.prediction_error.is_some());
        let error = episode.prediction_error.unwrap();
        assert!((error.magnitude - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn extract_signals_empty_context_no_events_emits_missed() {
        let ctx = converge_kernel::Context::new();
        let signals = extract_signals_from_run(&ctx, &[]);

        assert!(
            signals
                .iter()
                .any(|s| matches!(s.kind, SignalKind::OutcomeMissedExpectation))
        );
    }

    #[test]
    fn extract_signals_failing_outcome_emits_missed() {
        let ctx = converge_kernel::Context::new();
        let signals =
            extract_signals_from_run(&ctx, &[make_outcome_event(false, "budget_exhausted")]);

        assert!(
            signals
                .iter()
                .any(|s| matches!(s.kind, SignalKind::OutcomeMissedExpectation))
        );
        assert!(signals.iter().any(|s| s.note.contains("budget_exhausted")));
    }

    #[test]
    fn budget_exceeded_events_produce_blocker_signal() {
        let ctx = converge_kernel::Context::new();
        let budget_event = ExperienceEventEnvelope::new(
            "evt-budget",
            ExperienceEvent::BudgetExceeded {
                chain_id: "dd:test".into(),
                resource: "tokens".into(),
                limit: "10000".into(),
                observed: Some("15000".into()),
            },
        );
        let signals = extract_signals_from_run(&ctx, &[budget_event]);

        assert!(
            signals
                .iter()
                .any(|s| matches!(s.kind, SignalKind::AdversarialBlocker))
        );
    }

    #[test]
    fn build_episode_budget_exceeded_produces_lesson() {
        let ctx = converge_kernel::Context::new();
        let budget_event = ExperienceEventEnvelope::new(
            "evt-budget",
            ExperienceEvent::BudgetExceeded {
                chain_id: "dd:test".into(),
                resource: "tokens".into(),
                limit: "10000".into(),
                observed: None,
            },
        );
        let episode = build_episode_from_run(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "BudgetCo",
            &ctx,
            &[budget_event],
        );

        assert!(
            episode
                .lessons
                .iter()
                .any(|l| l.insight.contains("budget guard"))
        );
    }

    #[test]
    fn calibrate_priors_no_prediction_error_returns_empty() {
        let episode = LearningEpisode {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            predicted_outcome: "test".into(),
            actual_outcome: None,
            run_status: None,
            prediction_error: None,
            adversarial_signals: vec![],
            lessons: vec![],
        };

        assert!(calibrate_priors(&episode, &[]).is_empty());
    }

    #[test]
    fn calibrate_priors_accumulates_from_existing() {
        let existing = vec![PriorCalibration {
            assumption_type: "coverage".into(),
            context: "test".into(),
            prior_confidence: 0.5,
            posterior_confidence: 0.6,
            evidence_count: 3,
        }];

        let episode = LearningEpisode {
            id: Uuid::new_v4(),
            intent_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            predicted_outcome: "test".into(),
            actual_outcome: None,
            run_status: None,
            prediction_error: Some(PredictionError {
                magnitude: 0.2,
                dimensions: vec![ErrorDimension {
                    name: "coverage".into(),
                    predicted: 1.0,
                    actual: 0.8,
                }],
            }),
            adversarial_signals: vec![],
            lessons: vec![],
        };

        let priors = calibrate_priors(&episode, &existing);
        assert_eq!(priors.len(), 1);
        assert_eq!(priors[0].evidence_count, 4);
        assert!((priors[0].prior_confidence - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn has_infra_failure_detects_infra_constraint() {
        let ctx = promoted_context(&[(
            ContextKey::Constraints,
            "infra-fail",
            r#"{"type":"error","is_infra_failure":true,"message":"credits exhausted"}"#,
        )]);
        assert!(has_infra_failure(&ctx));
    }

    #[test]
    fn has_infra_failure_false_for_non_infra_constraint() {
        let ctx = promoted_context(&[(
            ContextKey::Constraints,
            "parse-fail",
            r#"{"type":"error","is_infra_failure":false,"message":"parse error"}"#,
        )]);
        assert!(!has_infra_failure(&ctx));
    }

    #[test]
    fn has_infra_failure_false_for_empty_context() {
        let ctx = converge_kernel::Context::new();
        assert!(!has_infra_failure(&ctx));
    }

    // ── Proptests ─────────────────────────────────────────────────

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        fn arb_error_dimension() -> impl Strategy<Value = ErrorDimension> {
            ("[a-z_]{3,15}", proptest::num::f64::NORMAL, 0.0..=1.0_f64).prop_map(
                |(name, predicted, actual)| ErrorDimension {
                    name,
                    predicted,
                    actual,
                },
            )
        }

        proptest! {
            #[test]
            fn calibrate_priors_posterior_stays_bounded(
                actual in 0.0..=1.0_f64,
                prior_conf in 0.0..=1.0_f64,
                evidence in 0_u32..100,
            ) {
                let episode = LearningEpisode {
                    id: Uuid::new_v4(),
                    intent_id: Uuid::new_v4(),
                    plan_id: Uuid::new_v4(),
                    predicted_outcome: "test".into(),
                    actual_outcome: None,
                    run_status: None,
                    prediction_error: Some(PredictionError {
                        magnitude: (1.0 - actual).abs(),
                        dimensions: vec![ErrorDimension {
                            name: "dim".into(),
                            predicted: 1.0,
                            actual,
                        }],
                    }),
                    adversarial_signals: vec![],
                    lessons: vec![],
                };

                let existing = vec![PriorCalibration {
                    assumption_type: "dim".into(),
                    context: "test".into(),
                    prior_confidence: prior_conf,
                    posterior_confidence: prior_conf,
                    evidence_count: evidence,
                }];

                let priors = calibrate_priors(&episode, &existing);
                prop_assert_eq!(priors.len(), 1);
                let posterior = priors[0].posterior_confidence;
                prop_assert!(posterior >= 0.0, "posterior {} < 0", posterior);
                prop_assert!(posterior <= 1.0, "posterior {} > 1", posterior);
            }

            #[test]
            fn calibrate_priors_evidence_always_increments(
                dims in proptest::collection::vec(arb_error_dimension(), 1..5),
            ) {
                let episode = LearningEpisode {
                    id: Uuid::new_v4(),
                    intent_id: Uuid::new_v4(),
                    plan_id: Uuid::new_v4(),
                    predicted_outcome: "test".into(),
                    actual_outcome: None,
                    run_status: None,
                    prediction_error: Some(PredictionError {
                        magnitude: 0.5,
                        dimensions: dims,
                    }),
                    adversarial_signals: vec![],
                    lessons: vec![],
                };

                let priors = calibrate_priors(&episode, &[]);
                for prior in &priors {
                    prop_assert_eq!(prior.evidence_count, 1);
                }
            }

            #[test]
            fn calibrate_converges_toward_observation(
                actual in 0.0..=1.0_f64,
                rounds in 1_usize..20,
            ) {
                let episode = LearningEpisode {
                    id: Uuid::new_v4(),
                    intent_id: Uuid::new_v4(),
                    plan_id: Uuid::new_v4(),
                    predicted_outcome: "test".into(),
                    actual_outcome: None,
                    run_status: None,
                    prediction_error: Some(PredictionError {
                        magnitude: 0.5,
                        dimensions: vec![ErrorDimension {
                            name: "dim".into(),
                            predicted: 0.5,
                            actual,
                        }],
                    }),
                    adversarial_signals: vec![],
                    lessons: vec![],
                };

                let mut priors = vec![];
                for _ in 0..rounds {
                    priors = calibrate_priors(&episode, &priors);
                }

                let posterior = priors[0].posterior_confidence;
                let distance = (posterior - actual).abs();
                let initial_distance = (0.5 - actual).abs();
                if rounds >= 3 && initial_distance > 0.05 {
                    prop_assert!(
                        distance < initial_distance,
                        "posterior {} should be closer to actual {} than initial 0.5 (dist={}, initial_dist={})",
                        posterior, actual, distance, initial_distance
                    );
                }
            }
        }
    }
}
