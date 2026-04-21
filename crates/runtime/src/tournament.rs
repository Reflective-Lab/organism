//! Formation tournament — run competing formations and learn from the comparison.
//!
//! Runs N `Formation`s on the same intent seed, scores each result using only
//! `ConvergeResult` fields (convergence, cycle efficiency, criteria coverage),
//! picks a winner, and produces `PriorCalibration` updates ready to feed into
//! the next `PlanningPriorAgent` seed.
//!
//! No new Converge types. No wrapper layers. Uses only public `converge-kernel`
//! fields and `organism-learning`'s adapter.

use converge_kernel::CriterionResult;
use uuid::Uuid;

use crate::formation::{Formation, FormationError, FormationResult};
use organism_learning::PriorCalibration;
use organism_learning::adapter::calibrate_priors;
use organism_learning::{ErrorDimension, LearningEpisode, PredictionError};

// ── Score ─────────────────────────────────────────────────────────────────────

/// Score derived entirely from `ConvergeResult` fields.
#[derive(Debug, Clone)]
pub struct FormationScore {
    pub label: String,
    /// Composite score in [0, 1]. Higher is better.
    pub score: f64,
    pub converged: bool,
    pub cycles: u32,
    /// Number of `CriterionResult::Met` outcomes (application-supplied criteria).
    pub criteria_met: usize,
    /// Total criteria evaluated.
    pub criteria_total: usize,
}

impl FormationScore {
    fn from_result(result: &FormationResult) -> Self {
        let cr = &result.converge_result;

        let criteria_total = cr.criteria_outcomes.len();
        let criteria_met = cr
            .criteria_outcomes
            .iter()
            .filter(|o| matches!(o.result, CriterionResult::Met { .. }))
            .count();

        // Convergence is the dominant signal.
        // Efficiency bonus: fewer cycles = higher score (cap at 50 cycles).
        // Criteria coverage: proportion of supplied criteria met.
        let convergence_score = if cr.converged { 1.0 } else { 0.0 };
        let efficiency_score = 1.0 - (f64::from(cr.cycles) / 50.0_f64).min(1.0);
        let criteria_score = if criteria_total == 0 {
            0.5 // neutral when no criteria registered
        } else {
            f64::from(u32::try_from(criteria_met).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(criteria_total).unwrap_or(u32::MAX))
        };

        // Weights: convergence 60%, efficiency 20%, criteria 20%.
        let score = convergence_score * 0.6 + efficiency_score * 0.2 + criteria_score * 0.2;

        Self {
            label: result.label.clone(),
            score,
            converged: cr.converged,
            cycles: cr.cycles,
            criteria_met,
            criteria_total,
        }
    }
}

// ── Tournament ────────────────────────────────────────────────────────────────

pub struct FormationTournament {
    formations: Vec<Formation>,
    intent_id: Uuid,
    plan_id: Uuid,
}

pub struct TournamentResult {
    pub winner: FormationScore,
    pub all_scores: Vec<FormationScore>,
    /// Calibrated priors ready to seed the next `PlanningPriorAgent` run.
    pub priors: Vec<PriorCalibration>,
}

#[derive(Debug, thiserror::Error)]
pub enum TournamentError {
    #[error("no formations provided")]
    NoFormations,
    #[error("all formations failed: {0}")]
    AllFailed(String),
    #[error("formation error: {0}")]
    Formation(#[from] FormationError),
}

impl FormationTournament {
    pub fn new(intent_id: Uuid, plan_id: Uuid, formations: Vec<Formation>) -> Self {
        Self {
            formations,
            intent_id,
            plan_id,
        }
    }

    /// Run all formations, score them, pick the winner, and calibrate priors.
    pub async fn run(self) -> Result<TournamentResult, TournamentError> {
        if self.formations.is_empty() {
            return Err(TournamentError::NoFormations);
        }

        let mut results: Vec<FormationResult> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        for formation in self.formations {
            match formation.run().await {
                Ok(r) => results.push(r),
                Err(e) => errors.push(e.to_string()),
            }
        }

        if results.is_empty() {
            return Err(TournamentError::AllFailed(errors.join("; ")));
        }

        // Score all results.
        let mut scores: Vec<FormationScore> =
            results.iter().map(FormationScore::from_result).collect();

        // Sort descending by score — first is winner.
        scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let winner = scores[0].clone();

        // Build a learning episode from the winner's score and calibrate priors.
        // The convergence and efficiency metrics become prediction-error dimensions
        // so `PlanningPriorAgent` can bias future runs toward configurations that
        // converged quickly and met their criteria.
        let priors = calibrate_priors(
            &episode_from_scores(&scores, self.intent_id, self.plan_id),
            &[],
        );

        Ok(TournamentResult {
            winner,
            all_scores: scores,
            priors,
        })
    }
}

/// Construct a minimal `LearningEpisode` from tournament scores so we can
/// feed it into `calibrate_priors` without duplicating the Bayesian logic.
fn episode_from_scores(
    scores: &[FormationScore],
    intent_id: Uuid,
    plan_id: Uuid,
) -> LearningEpisode {
    let winner = &scores[0];

    // convergence_rate: fraction of formations that converged.
    let converged_count = scores.iter().filter(|s| s.converged).count();
    let convergence_rate = f64::from(u32::try_from(converged_count).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(scores.len()).unwrap_or(u32::MAX));

    // criteria_coverage: winner's criteria coverage.
    let criteria_coverage = if winner.criteria_total == 0 {
        1.0
    } else {
        f64::from(u32::try_from(winner.criteria_met).unwrap_or(u32::MAX))
            / f64::from(u32::try_from(winner.criteria_total).unwrap_or(u32::MAX))
    };

    // cycle_efficiency: normalised inverse cycle count of winner.
    let cycle_efficiency = 1.0 - (f64::from(winner.cycles) / 50.0_f64).min(1.0);

    LearningEpisode {
        id: Uuid::new_v4(),
        intent_id,
        plan_id,
        predicted_outcome: format!("winner: {}", winner.label),
        actual_outcome: Some(format!(
            "score={:.3} converged={} cycles={}",
            winner.score, winner.converged, winner.cycles
        )),
        run_status: Some(if winner.converged {
            "converged".into()
        } else {
            "did-not-converge".into()
        }),
        prediction_error: Some(PredictionError {
            magnitude: 1.0 - winner.score,
            dimensions: vec![
                ErrorDimension {
                    name: "convergence_rate".into(),
                    predicted: 1.0,
                    actual: convergence_rate,
                },
                ErrorDimension {
                    name: "criteria_coverage".into(),
                    predicted: 1.0,
                    actual: criteria_coverage,
                },
                ErrorDimension {
                    name: "cycle_efficiency".into(),
                    predicted: 1.0,
                    actual: cycle_efficiency,
                },
            ],
        }),
        adversarial_signals: vec![],
        lessons: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{AgentEffect, Context, ContextKey, ProposedFact};
    use converge_pack::Suggestor;

    struct ConvergingAgent;

    #[async_trait::async_trait]
    impl Suggestor for ConvergingAgent {
        fn name(&self) -> &'static str {
            "converging"
        }

        fn dependencies(&self) -> &[ContextKey] {
            &[ContextKey::Seeds]
        }

        fn accepts(&self, ctx: &dyn Context) -> bool {
            ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
        }

        async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
            let seeds = ctx.get(ContextKey::Seeds);
            AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Hypotheses,
                format!("hyp-{}", seeds[0].id),
                "converged hypothesis",
                self.name(),
            ))
        }
    }

    fn make_formation(label: &str) -> Formation {
        Formation::new(label).agent(ConvergingAgent).seed(
            ContextKey::Seeds,
            "s1",
            "test content",
            "test",
        )
    }

    fn id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    // ── Scoring ───────────────────────────────────────────────────────────────

    #[test]
    fn score_converged_result_above_zero_point_six() {
        // A converged result with no criteria gets convergence(0.6) + efficiency(~0.2) + neutral(0.1)
        // roughly ≥ 0.7
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(make_formation("f1").run())
            .unwrap();
        let score = FormationScore::from_result(&result);
        assert!(score.converged);
        assert!(score.score > 0.6, "score was {}", score.score);
    }

    // ── Tournament ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn tournament_picks_winner_from_two_formations() {
        let t = FormationTournament::new(
            id(),
            id(),
            vec![make_formation("team-a"), make_formation("team-b")],
        );

        let result = t.run().await.unwrap();
        assert!(!result.winner.label.is_empty());
        assert_eq!(result.all_scores.len(), 2);
        // Winner must have the highest score
        for score in &result.all_scores {
            assert!(result.winner.score >= score.score);
        }
    }

    #[tokio::test]
    async fn tournament_produces_priors() {
        let t = FormationTournament::new(id(), id(), vec![make_formation("solo")]);
        let result = t.run().await.unwrap();
        // Should produce calibrations for convergence_rate, criteria_coverage, cycle_efficiency
        assert!(!result.priors.is_empty());
        assert!(
            result
                .priors
                .iter()
                .any(|p| p.assumption_type == "convergence_rate")
        );
        assert!(
            result
                .priors
                .iter()
                .any(|p| p.assumption_type == "criteria_coverage")
        );
        assert!(
            result
                .priors
                .iter()
                .any(|p| p.assumption_type == "cycle_efficiency")
        );
    }

    #[tokio::test]
    async fn tournament_error_on_no_formations() {
        let t = FormationTournament::new(id(), id(), vec![]);
        assert!(matches!(t.run().await, Err(TournamentError::NoFormations)));
    }

    #[tokio::test]
    async fn tournament_scores_sorted_descending() {
        let t = FormationTournament::new(
            id(),
            id(),
            vec![
                make_formation("a"),
                make_formation("b"),
                make_formation("c"),
            ],
        );
        let result = t.run().await.unwrap();
        let scores: Vec<f64> = result.all_scores.iter().map(|s| s.score).collect();
        for window in scores.windows(2) {
            assert!(window[0] >= window[1]);
        }
    }

    #[tokio::test]
    async fn tournament_winner_is_first_in_sorted_list() {
        let t =
            FormationTournament::new(id(), id(), vec![make_formation("a"), make_formation("b")]);
        let result = t.run().await.unwrap();
        assert_eq!(result.winner.label, result.all_scores[0].label);
    }

    // ── Prior calibration tightening ─────────────────────────────────────────

    #[tokio::test]
    async fn repeated_tournaments_tighten_priors() {
        // Run two rounds, pass first-round priors back into second round
        // via episode_from_scores. Evidence count must increment each round.
        let run_once = |existing: Vec<PriorCalibration>| async move {
            let t = FormationTournament::new(id(), id(), vec![make_formation("f")]);
            let result = t.run().await.unwrap();
            let episode = episode_from_scores(&result.all_scores, id(), id());
            calibrate_priors(&episode, &existing)
        };

        let round1 = run_once(vec![]).await;
        let round2 = run_once(round1.clone()).await;

        assert_eq!(round1[0].evidence_count, 1);
        assert_eq!(round2[0].evidence_count, 2);
    }

    // ── Priors are valid PlanningPriorAgent seeds ─────────────────────────────

    #[tokio::test]
    async fn priors_are_serializable_as_planning_prior_seeds() {
        let t = FormationTournament::new(id(), id(), vec![make_formation("f")]);
        let result = t.run().await.unwrap();

        for prior in &result.priors {
            let seed_content = serde_json::json!({
                "type": "prior_calibration",
                "calibration": prior,
            });
            // Must round-trip so PlanningPriorAgent can deserialize it
            let json = seed_content.to_string();
            let back: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(
                back["calibration"]["assumption_type"].as_str().unwrap(),
                prior.assumption_type
            );
        }
    }
}
