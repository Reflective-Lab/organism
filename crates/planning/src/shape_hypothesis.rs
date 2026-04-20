//! Shape-as-hypothesis — the collaboration shape itself is an object of learning.
//!
//! Multiple candidate shapes compete for the same intent. Each shape is
//! evaluated by evidence quality, convergence speed, or a balanced metric.
//! The winner is selected, and the learning layer calibrates priors about
//! which shapes work for which problem classes.
//!
//! Over time the system discovers collaboration patterns that no human
//! would design. Because learning feeds into planning priors (never
//! authority), the governance layer catches anything that shouldn't land.

use organism_intent::{IntentPacket, Reversibility};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::collaboration::{CollaborationCharter, CollaborationTopology};

/// A candidate collaboration shape being tested.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeCandidate {
    pub id: Uuid,
    pub charter: CollaborationCharter,
    pub rationale: String,
    pub prior_score: f64,
    pub evidence_quality: Option<f64>,
}

/// A competition between multiple candidate shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeCompetition {
    pub intent_id: Uuid,
    pub candidates: Vec<ShapeCandidate>,
    pub evaluation_metric: ShapeMetric,
    pub winner: Option<Uuid>,
}

/// What metric determines shape quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeMetric {
    EvidenceQuality,
    ConvergenceSpeed,
    ContradictionMinimization,
    Balanced,
}

/// Observation from a completed shape trial.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeObservation {
    pub candidate_id: Uuid,
    pub hypothesis_count: usize,
    pub avg_confidence: f64,
    pub contradiction_rate: f64,
    pub cycles_to_stability: u32,
    pub budget_used_fraction: f64,
}

/// Historical calibration of shape performance for a problem class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeCalibration {
    pub problem_class: String,
    pub topology: CollaborationTopology,
    pub prior_score: f64,
    pub posterior_score: f64,
    pub observation_count: u32,
}

/// Classify an intent into a problem class for calibration lookup.
pub fn classify_problem(intent: &IntentPacket) -> String {
    let reversibility = match intent.reversibility {
        Reversibility::Reversible => "reversible",
        Reversibility::Partial => "partial",
        Reversibility::Irreversible => "irreversible",
    };

    let complexity = if intent.constraints.len() >= 4 || intent.forbidden.len() >= 3 {
        "high"
    } else if intent.constraints.len() >= 2 || !intent.forbidden.is_empty() {
        "medium"
    } else {
        "low"
    };

    let authority = if intent.authority.len() >= 3 {
        "multi_authority"
    } else if !intent.authority.is_empty() {
        "single_authority"
    } else {
        "no_authority"
    };

    format!("{reversibility}_{complexity}_{authority}")
}

/// Generate candidate shapes for an intent.
///
/// Always produces at least 2 candidates: the derived charter plus
/// an alternative that explores a different point on the structure spectrum.
pub fn generate_candidates(
    intent: &IntentPacket,
    now: chrono::DateTime<chrono::Utc>,
    priors: &[ShapeCalibration],
) -> Vec<ShapeCandidate> {
    let derived = crate::charter_derivation::derive_charter(intent, now);
    let problem_class = classify_problem(intent);

    let mut candidates = vec![ShapeCandidate {
        id: Uuid::new_v4(),
        charter: derived.charter.clone(),
        rationale: format!("Derived from intent: {}", derived.rationale.topology_reason),
        prior_score: derived.confidence,
        evidence_quality: None,
    }];

    // Generate an alternative on the opposite end of the structure spectrum.
    let alt_topology = opposite_topology(derived.charter.topology);
    let alt_charter = match alt_topology {
        CollaborationTopology::Huddle => CollaborationCharter::huddle(),
        CollaborationTopology::DiscussionGroup => CollaborationCharter::discussion_group(),
        CollaborationTopology::Panel => CollaborationCharter::panel(),
        CollaborationTopology::SelfOrganizing => CollaborationCharter::self_organizing(),
    };

    let alt_prior = priors
        .iter()
        .find(|p| p.problem_class == problem_class && p.topology == alt_topology)
        .map_or(0.3, |p| p.posterior_score);

    candidates.push(ShapeCandidate {
        id: Uuid::new_v4(),
        charter: alt_charter,
        rationale: format!("Alternative: {alt_topology:?} explores the opposite structure point",),
        prior_score: alt_prior,
        evidence_quality: None,
    });

    // If priors suggest a third topology that performed well, include it.
    let best_prior = priors
        .iter()
        .filter(|p| {
            p.problem_class == problem_class
                && p.topology != derived.charter.topology
                && p.topology != alt_topology
                && p.observation_count >= 2
        })
        .max_by(|a, b| {
            a.posterior_score
                .partial_cmp(&b.posterior_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some(prior) = best_prior {
        let prior_charter = match prior.topology {
            CollaborationTopology::Huddle => CollaborationCharter::huddle(),
            CollaborationTopology::DiscussionGroup => CollaborationCharter::discussion_group(),
            CollaborationTopology::Panel => CollaborationCharter::panel(),
            CollaborationTopology::SelfOrganizing => CollaborationCharter::self_organizing(),
        };
        candidates.push(ShapeCandidate {
            id: Uuid::new_v4(),
            charter: prior_charter,
            rationale: format!(
                "Prior-informed: {:?} scored {:.2} over {} observations for '{}'",
                prior.topology, prior.posterior_score, prior.observation_count, problem_class
            ),
            prior_score: prior.posterior_score,
            evidence_quality: None,
        });
    }

    candidates
}

fn opposite_topology(topology: CollaborationTopology) -> CollaborationTopology {
    match topology {
        CollaborationTopology::SelfOrganizing => CollaborationTopology::Panel,
        CollaborationTopology::Panel => CollaborationTopology::SelfOrganizing,
        CollaborationTopology::Huddle => CollaborationTopology::DiscussionGroup,
        CollaborationTopology::DiscussionGroup => CollaborationTopology::Huddle,
    }
}

/// Score a shape observation against the chosen metric. Returns [0.0, 1.0].
#[allow(clippy::cast_precision_loss)]
pub fn score_observation(observation: &ShapeObservation, metric: ShapeMetric) -> f64 {
    match metric {
        ShapeMetric::EvidenceQuality => {
            let quantity = (observation.hypothesis_count as f64 / 50.0).min(1.0);
            let quality = observation.avg_confidence.clamp(0.0, 1.0);
            (quantity * 0.4 + quality * 0.6).clamp(0.0, 1.0)
        }
        ShapeMetric::ConvergenceSpeed => {
            let speed = 1.0 - (f64::from(observation.cycles_to_stability) / 20.0).min(1.0);
            let efficiency = 1.0 - observation.budget_used_fraction.clamp(0.0, 1.0);
            (speed * 0.7 + efficiency * 0.3).clamp(0.0, 1.0)
        }
        ShapeMetric::ContradictionMinimization => {
            (1.0 - observation.contradiction_rate.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        }
        ShapeMetric::Balanced => {
            let evidence = score_observation(observation, ShapeMetric::EvidenceQuality);
            let speed = score_observation(observation, ShapeMetric::ConvergenceSpeed);
            let contradictions =
                score_observation(observation, ShapeMetric::ContradictionMinimization);
            (evidence * 0.4 + speed * 0.3 + contradictions * 0.3).clamp(0.0, 1.0)
        }
    }
}

/// Select the winner from completed observations.
pub fn select_winner(
    competition: &ShapeCompetition,
    observations: &[ShapeObservation],
) -> Option<Uuid> {
    if observations.is_empty() {
        return None;
    }

    observations
        .iter()
        .filter(|obs| {
            competition
                .candidates
                .iter()
                .any(|c| c.id == obs.candidate_id)
        })
        .max_by(|a, b| {
            let score_a = score_observation(a, competition.evaluation_metric);
            let score_b = score_observation(b, competition.evaluation_metric);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|obs| obs.candidate_id)
}

/// Calibrate shape priors from an observation.
/// Feeds into planning priors, NEVER into authority.
pub fn calibrate_shape(
    problem_class: &str,
    topology: CollaborationTopology,
    score: f64,
    existing: &[ShapeCalibration],
) -> ShapeCalibration {
    let prior = existing
        .iter()
        .find(|c| c.problem_class == problem_class && c.topology == topology);

    let (prior_score, evidence) = match prior {
        Some(p) => (p.posterior_score, p.observation_count),
        None => (0.5, 0),
    };

    let observation_weight = 1.0 / (f64::from(evidence) + 2.0);
    let posterior = prior_score * (1.0 - observation_weight) + score * observation_weight;

    ShapeCalibration {
        problem_class: problem_class.to_string(),
        topology,
        prior_score,
        posterior_score: posterior.clamp(0.0, 1.0),
        observation_count: evidence + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn test_intent() -> IntentPacket {
        let now = Utc::now();
        IntentPacket::new("Test", now + Duration::days(7))
    }

    #[test]
    fn generate_candidates_produces_at_least_two() {
        let intent = test_intent();
        let candidates = generate_candidates(&intent, Utc::now(), &[]);
        assert!(candidates.len() >= 2);
    }

    #[test]
    fn generate_candidates_includes_prior_informed_third() {
        let intent = test_intent();
        let problem_class = classify_problem(&intent);
        let priors = vec![ShapeCalibration {
            problem_class,
            topology: CollaborationTopology::Huddle,
            prior_score: 0.5,
            posterior_score: 0.8,
            observation_count: 5,
        }];

        let candidates = generate_candidates(&intent, Utc::now(), &priors);
        assert!(candidates.len() >= 3);
        assert!(
            candidates
                .iter()
                .any(|c| c.rationale.contains("Prior-informed"))
        );
    }

    #[test]
    fn score_observation_evidence_quality() {
        let obs = ShapeObservation {
            candidate_id: Uuid::new_v4(),
            hypothesis_count: 50,
            avg_confidence: 0.9,
            contradiction_rate: 0.1,
            cycles_to_stability: 5,
            budget_used_fraction: 0.5,
        };

        let score = score_observation(&obs, ShapeMetric::EvidenceQuality);
        assert!(score > 0.7);
        assert!(score <= 1.0);
    }

    #[test]
    fn score_observation_convergence_speed() {
        let fast = ShapeObservation {
            candidate_id: Uuid::new_v4(),
            hypothesis_count: 10,
            avg_confidence: 0.5,
            contradiction_rate: 0.0,
            cycles_to_stability: 2,
            budget_used_fraction: 0.2,
        };
        let slow = ShapeObservation {
            candidate_id: Uuid::new_v4(),
            hypothesis_count: 10,
            avg_confidence: 0.5,
            contradiction_rate: 0.0,
            cycles_to_stability: 18,
            budget_used_fraction: 0.9,
        };

        assert!(
            score_observation(&fast, ShapeMetric::ConvergenceSpeed)
                > score_observation(&slow, ShapeMetric::ConvergenceSpeed)
        );
    }

    #[test]
    fn select_winner_picks_highest_score() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();

        let competition = ShapeCompetition {
            intent_id: Uuid::new_v4(),
            candidates: vec![
                ShapeCandidate {
                    id: id_a,
                    charter: CollaborationCharter::huddle(),
                    rationale: "A".into(),
                    prior_score: 0.5,
                    evidence_quality: None,
                },
                ShapeCandidate {
                    id: id_b,
                    charter: CollaborationCharter::panel(),
                    rationale: "B".into(),
                    prior_score: 0.5,
                    evidence_quality: None,
                },
            ],
            evaluation_metric: ShapeMetric::EvidenceQuality,
            winner: None,
        };

        let observations = vec![
            ShapeObservation {
                candidate_id: id_a,
                hypothesis_count: 10,
                avg_confidence: 0.5,
                contradiction_rate: 0.2,
                cycles_to_stability: 5,
                budget_used_fraction: 0.5,
            },
            ShapeObservation {
                candidate_id: id_b,
                hypothesis_count: 40,
                avg_confidence: 0.9,
                contradiction_rate: 0.05,
                cycles_to_stability: 3,
                budget_used_fraction: 0.4,
            },
        ];

        let winner = select_winner(&competition, &observations);
        assert_eq!(winner, Some(id_b));
    }

    #[test]
    fn classify_problem_consistent() {
        let intent = test_intent();
        let class1 = classify_problem(&intent);
        let class2 = classify_problem(&intent);
        assert_eq!(class1, class2);
    }

    #[test]
    fn classify_problem_varies_with_reversibility() {
        let now = Utc::now();
        let mut reversible = IntentPacket::new("A", now + Duration::days(7));
        reversible.reversibility = Reversibility::Reversible;

        let mut irreversible = IntentPacket::new("B", now + Duration::days(7));
        irreversible.reversibility = Reversibility::Irreversible;

        assert_ne!(
            classify_problem(&reversible),
            classify_problem(&irreversible)
        );
    }

    #[test]
    fn calibrate_shape_from_scratch() {
        let cal = calibrate_shape("test_class", CollaborationTopology::Huddle, 0.8, &[]);

        assert_eq!(cal.problem_class, "test_class");
        assert_eq!(cal.topology, CollaborationTopology::Huddle);
        assert!((cal.prior_score - 0.5).abs() < f64::EPSILON);
        assert_eq!(cal.observation_count, 1);
        assert!(cal.posterior_score > 0.5);
        assert!(cal.posterior_score < 0.8);
    }

    #[test]
    fn calibrate_shape_converges() {
        let mut calibrations: Vec<ShapeCalibration> = vec![];
        for _ in 0..10 {
            let cal = calibrate_shape("test", CollaborationTopology::Panel, 0.9, &calibrations);
            calibrations = vec![cal];
        }

        assert!(calibrations[0].posterior_score > 0.75);
        assert_eq!(calibrations[0].observation_count, 10);
    }

    // ── Negative tests ────────────────────────────────────────────

    #[test]
    fn select_winner_empty_observations() {
        let competition = ShapeCompetition {
            intent_id: Uuid::new_v4(),
            candidates: vec![],
            evaluation_metric: ShapeMetric::Balanced,
            winner: None,
        };
        assert!(select_winner(&competition, &[]).is_none());
    }

    #[test]
    fn score_zero_hypotheses() {
        let obs = ShapeObservation {
            candidate_id: Uuid::new_v4(),
            hypothesis_count: 0,
            avg_confidence: 0.0,
            contradiction_rate: 0.0,
            cycles_to_stability: 0,
            budget_used_fraction: 0.0,
        };
        let score = score_observation(&obs, ShapeMetric::Balanced);
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn score_extreme_values() {
        let obs = ShapeObservation {
            candidate_id: Uuid::new_v4(),
            hypothesis_count: 10_000,
            avg_confidence: 10.0,    // out of range, should clamp
            contradiction_rate: 5.0, // out of range
            cycles_to_stability: 1000,
            budget_used_fraction: 2.0, // out of range
        };

        for metric in [
            ShapeMetric::EvidenceQuality,
            ShapeMetric::ConvergenceSpeed,
            ShapeMetric::ContradictionMinimization,
            ShapeMetric::Balanced,
        ] {
            let score = score_observation(&obs, metric);
            assert!(score >= 0.0, "metric {metric:?} score {score} < 0");
            assert!(score <= 1.0, "metric {metric:?} score {score} > 1");
        }
    }

    #[test]
    fn generate_candidates_with_empty_priors() {
        let intent = test_intent();
        let candidates = generate_candidates(&intent, Utc::now(), &[]);
        assert!(candidates.len() >= 2);
        // No third candidate since no priors
        assert_eq!(candidates.len(), 2);
    }

    // ── Proptests ─────────────────────────────────────────────────

    #[allow(clippy::cast_precision_loss)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn score_always_bounded(
                hyp in 0_usize..200,
                conf in 0.0..=2.0_f64,
                contra in 0.0..=2.0_f64,
                cycles in 0_u32..100,
                budget in 0.0..=2.0_f64,
            ) {
                let obs = ShapeObservation {
                    candidate_id: Uuid::new_v4(),
                    hypothesis_count: hyp,
                    avg_confidence: conf,
                    contradiction_rate: contra,
                    cycles_to_stability: cycles,
                    budget_used_fraction: budget,
                };

                for metric in [
                    ShapeMetric::EvidenceQuality,
                    ShapeMetric::ConvergenceSpeed,
                    ShapeMetric::ContradictionMinimization,
                    ShapeMetric::Balanced,
                ] {
                    let score = score_observation(&obs, metric);
                    prop_assert!((0.0..=1.0).contains(&score), "metric {metric:?} score {score}");
                }
            }

            #[test]
            fn calibrate_posterior_bounded(
                score in 0.0..=1.0_f64,
                prior_score in 0.0..=1.0_f64,
                evidence in 0_u32..100,
            ) {
                let existing = vec![ShapeCalibration {
                    problem_class: "test".into(),
                    topology: CollaborationTopology::Huddle,
                    prior_score,
                    posterior_score: prior_score,
                    observation_count: evidence,
                }];

                let cal = calibrate_shape("test", CollaborationTopology::Huddle, score, &existing);
                prop_assert!(cal.posterior_score >= 0.0);
                prop_assert!(cal.posterior_score <= 1.0);
                prop_assert_eq!(cal.observation_count, evidence + 1);
            }

            #[test]
            fn calibrate_converges_toward_observation(
                score in 0.0..=1.0_f64,
                rounds in 1_usize..20,
            ) {
                let mut cals: Vec<ShapeCalibration> = vec![];
                for _ in 0..rounds {
                    let cal = calibrate_shape("test", CollaborationTopology::Huddle, score, &cals);
                    cals = vec![cal];
                }

                let posterior = cals[0].posterior_score;
                let distance = (posterior - score).abs();
                let initial_distance = (0.5 - score).abs();
                if rounds >= 3 && initial_distance > 0.05 {
                    prop_assert!(
                        distance < initial_distance,
                        "posterior {posterior} should be closer to score {score} than initial 0.5"
                    );
                }
            }
        }
    }
}
