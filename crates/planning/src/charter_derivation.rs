//! Charter derivation from intent.
//!
//! Reads an `IntentPacket`'s properties and produces a `CollaborationCharter`
//! with a transparent rationale explaining every choice. Not a lookup table —
//! a multi-signal derivation where each field is justified.
//!
//! Optionally integrates historical `ShapeCalibration` priors to bias the
//! derivation toward shapes that have worked for similar problem classes.

use chrono::{DateTime, Utc};
use organism_intent::{ExpiryAction, IntentPacket, Reversibility};
use serde::{Deserialize, Serialize};

use crate::collaboration::{
    CollaborationCharter, CollaborationDiscipline, CollaborationRole, CollaborationTopology,
    ConsensusRule, TeamFormationMode, TurnCadence,
};
use crate::shape_hypothesis::ShapeCalibration;

/// Quantified complexity signals extracted from the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentComplexity {
    pub constraint_pressure: f64,
    pub authority_breadth: f64,
    pub forbidden_density: f64,
    pub time_pressure: f64,
    pub reversibility_weight: f64,
    pub escalation_required: bool,
}

/// Why each charter field was chosen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivationRationale {
    pub topology_reason: String,
    pub discipline_reason: String,
    pub consensus_reason: String,
    pub cadence_reason: String,
    pub formation_reason: String,
    pub role_reasons: Vec<(String, String)>,
    pub flag_reasons: Vec<(String, bool, String)>,
}

/// Result of charter derivation from an intent.
#[derive(Debug, Clone)]
pub struct DerivedCharter {
    pub charter: CollaborationCharter,
    pub rationale: DerivationRationale,
    pub confidence: f64,
    pub intent_complexity: IntentComplexity,
}

fn compute_complexity(intent: &IntentPacket, now: DateTime<Utc>) -> IntentComplexity {
    let constraint_pressure = normalize_count(intent.constraints.len(), 6);
    let authority_breadth = normalize_count(intent.authority.len(), 4);
    let forbidden_density = normalize_count(intent.forbidden.len(), 5);

    let duration = intent.expires.signed_duration_since(now);
    #[allow(clippy::cast_precision_loss)]
    let hours = duration.num_hours().max(0) as f64;
    let time_pressure = 1.0 - (hours / 168.0).min(1.0); // 1 week = relaxed

    let reversibility_weight = match intent.reversibility {
        Reversibility::Reversible => 0.0,
        Reversibility::Partial => 0.5,
        Reversibility::Irreversible => 1.0,
    };

    let escalation_required = matches!(intent.expiry_action, ExpiryAction::Escalate);

    IntentComplexity {
        constraint_pressure,
        authority_breadth,
        forbidden_density,
        time_pressure,
        reversibility_weight,
        escalation_required,
    }
}

#[allow(clippy::cast_precision_loss)]
fn normalize_count(count: usize, saturation: usize) -> f64 {
    if saturation == 0 {
        return 0.0;
    }
    (count as f64 / saturation as f64).min(1.0)
}

/// Derive a collaboration charter from an intent's properties.
pub fn derive_charter(intent: &IntentPacket, now: DateTime<Utc>) -> DerivedCharter {
    let complexity = compute_complexity(intent, now);
    derive_from_complexity(&complexity)
}

/// Derive a charter, biased by historical shape calibration.
pub fn derive_charter_with_priors(
    intent: &IntentPacket,
    now: DateTime<Utc>,
    priors: &[ShapeCalibration],
) -> DerivedCharter {
    let mut derived = derive_charter(intent, now);

    if priors.is_empty() {
        return derived;
    }

    let problem_class = crate::shape_hypothesis::classify_problem(intent);
    let relevant: Vec<&ShapeCalibration> = priors
        .iter()
        .filter(|p| p.problem_class == problem_class && p.observation_count > 0)
        .collect();

    if relevant.is_empty() {
        return derived;
    }

    // Find the best-performing topology for this problem class.
    let best = relevant
        .iter()
        .max_by(|a, b| a.posterior_score.partial_cmp(&b.posterior_score).unwrap_or(std::cmp::Ordering::Equal));

    if let Some(best) = best
        && best.posterior_score > derived.confidence && best.observation_count >= 3 {
            let preset = topology_preset(best.topology);
            derived.rationale.topology_reason = format!(
                "Prior calibration favors {:?} for problem class '{}' (score {:.2}, {} observations)",
                best.topology, problem_class, best.posterior_score, best.observation_count
            );
            derived.charter = preset;
            derived.confidence = f64::midpoint(derived.confidence, best.posterior_score);
    }

    derived
}

fn topology_preset(topology: CollaborationTopology) -> CollaborationCharter {
    match topology {
        CollaborationTopology::Huddle => CollaborationCharter::huddle(),
        CollaborationTopology::DiscussionGroup => CollaborationCharter::discussion_group(),
        CollaborationTopology::Panel => CollaborationCharter::panel(),
        CollaborationTopology::SelfOrganizing => CollaborationCharter::self_organizing(),
    }
}

fn derive_from_complexity(c: &IntentComplexity) -> DerivedCharter {
    let stakes = c.reversibility_weight * 0.4
        + c.constraint_pressure * 0.2
        + c.forbidden_density * 0.2
        + c.authority_breadth * 0.2;

    let (topology, topology_reason) = derive_topology(c, stakes);
    let (discipline, discipline_reason) = derive_discipline(c, stakes);
    let (consensus, consensus_reason) = derive_consensus(c, stakes);
    let (cadence, cadence_reason) = derive_cadence(c, stakes);
    let (formation, formation_reason) = derive_formation(c);
    let (roles, role_reasons) = derive_roles(c, topology);
    let (flags, flag_reasons) = derive_flags(c, stakes);

    let minimum_members = match topology {
        CollaborationTopology::SelfOrganizing => 1,
        _ => 3,
    };

    let confidence = compute_confidence(c, stakes);

    let charter = CollaborationCharter {
        topology,
        formation_mode: formation,
        discipline,
        turn_cadence: cadence,
        consensus_rule: consensus,
        minimum_members,
        require_explicit_turns: flags[0].1,
        require_round_synthesis: flags[1].1,
        require_dissent_map: flags[2].1,
        require_done_gate: flags[3].1,
        require_report_owner: flags[4].1,
        expected_roles: roles.iter().map(|(r, _)| *r).collect(),
    };

    let rationale = DerivationRationale {
        topology_reason,
        discipline_reason,
        consensus_reason,
        cadence_reason,
        formation_reason,
        role_reasons: role_reasons
            .into_iter()
            .map(|(r, reason)| (r.label().to_string(), reason))
            .collect(),
        flag_reasons,
    };

    DerivedCharter {
        charter,
        rationale,
        confidence,
        intent_complexity: c.clone(),
    }
}

fn derive_topology(
    c: &IntentComplexity,
    stakes: f64,
) -> (CollaborationTopology, String) {
    if c.time_pressure >= 0.8 {
        (
            CollaborationTopology::Huddle,
            format!(
                "Tight deadline (time_pressure={:.2}) demands fast, structured collaboration",
                c.time_pressure
            ),
        )
    } else if c.authority_breadth >= 0.5 || stakes >= 0.7 {
        (
            CollaborationTopology::Panel,
            format!(
                "High stakes ({stakes:.2}) or multi-authority (breadth={:.2}) requires formal review with judges",
                c.authority_breadth
            ),
        )
    } else if stakes >= 0.4 {
        (
            CollaborationTopology::DiscussionGroup,
            format!(
                "Moderate stakes ({stakes:.2}) suits a moderated discussion with advisory output",
            ),
        )
    } else {
        (
            CollaborationTopology::SelfOrganizing,
            format!(
                "Low stakes ({stakes:.2}) and relaxed timeline (time_pressure={:.2}) — team can self-organize",
                c.time_pressure
            ),
        )
    }
}

fn derive_discipline(
    c: &IntentComplexity,
    stakes: f64,
) -> (CollaborationDiscipline, String) {
    if c.reversibility_weight >= 0.8 || stakes >= 0.7 {
        (
            CollaborationDiscipline::Enforced,
            format!(
                "Irreversible action (rev={:.1}) or high stakes ({stakes:.2}) requires enforced discipline",
                c.reversibility_weight
            ),
        )
    } else if stakes >= 0.3 {
        (
            CollaborationDiscipline::Moderated,
            format!(
                "Moderate stakes ({stakes:.2}) — soft guidance, formation mismatches tolerated"
            ),
        )
    } else {
        (
            CollaborationDiscipline::Loose,
            format!("Low stakes ({stakes:.2}) — minimal structure, maximum autonomy"),
        )
    }
}

fn derive_consensus(
    c: &IntentComplexity,
    stakes: f64,
) -> (ConsensusRule, String) {
    if c.reversibility_weight >= 1.0 && stakes >= 0.8 {
        (
            ConsensusRule::Unanimous,
            "Fully irreversible + highest stakes — all voters must agree".into(),
        )
    } else if c.reversibility_weight >= 0.8 || stakes >= 0.7 {
        (
            ConsensusRule::Supermajority,
            format!(
                "High reversibility weight ({:.1}) or stakes ({stakes:.2}) — supermajority required",
                c.reversibility_weight
            ),
        )
    } else if c.time_pressure >= 0.8 {
        (
            ConsensusRule::LeadDecides,
            format!(
                "Tight deadline (time_pressure={:.2}) — lead decides to avoid delay",
                c.time_pressure
            ),
        )
    } else if stakes >= 0.3 {
        (
            ConsensusRule::Majority,
            format!("Moderate stakes ({stakes:.2}) — simple majority sufficient"),
        )
    } else {
        (
            ConsensusRule::AdvisoryOnly,
            format!("Low stakes ({stakes:.2}) — advisory output, no binding vote"),
        )
    }
}

fn derive_cadence(
    c: &IntentComplexity,
    stakes: f64,
) -> (TurnCadence, String) {
    if c.time_pressure >= 0.8 {
        (
            TurnCadence::RoundRobin,
            "Tight deadline — strict rotation for maximum throughput".into(),
        )
    } else if c.authority_breadth >= 0.5 || stakes >= 0.7 {
        (
            TurnCadence::LeadThenRoundRobin,
            format!(
                "Formal setting (authority={:.2}, stakes={stakes:.2}) — lead frames, then rotation",
                c.authority_breadth
            ),
        )
    } else if stakes >= 0.3 {
        (
            TurnCadence::ModeratorThenRoundRobin,
            "Moderate stakes — moderator frames the discussion, then rotation".into(),
        )
    } else {
        (
            TurnCadence::FigureItOut,
            "Low stakes — agents self-coordinate".into(),
        )
    }
}

fn derive_formation(c: &IntentComplexity) -> (TeamFormationMode, String) {
    if c.authority_breadth >= 0.5 || c.reversibility_weight >= 0.8 {
        (
            TeamFormationMode::Curated,
            "High authority breadth or irreversibility — team must be hand-picked".into(),
        )
    } else if c.constraint_pressure >= 0.5 {
        (
            TeamFormationMode::CapabilityMatched,
            format!(
                "Moderate constraints (pressure={:.2}) — match capabilities to requirements",
                c.constraint_pressure
            ),
        )
    } else {
        (
            TeamFormationMode::OpenCall,
            "Low constraints — open participation".into(),
        )
    }
}

type RoleList = Vec<(CollaborationRole, String)>;

fn derive_roles(
    c: &IntentComplexity,
    topology: CollaborationTopology,
) -> (RoleList, RoleList) {
    let mut roles = Vec::new();

    match topology {
        CollaborationTopology::Panel => {
            roles.push((CollaborationRole::Lead, "Panel requires a lead to frame the discussion".into()));
            roles.push((CollaborationRole::Domain, "Domain expertise needed for substantive review".into()));
            roles.push((CollaborationRole::Critic, "Adversarial critic required for high-stakes decisions".into()));
            if c.authority_breadth >= 0.5 {
                roles.push((CollaborationRole::Judge, format!(
                    "Multi-authority (breadth={:.2}) needs independent judges",
                    c.authority_breadth
                )));
            }
        }
        CollaborationTopology::Huddle => {
            roles.push((CollaborationRole::Lead, "Huddle lead keeps the team focused".into()));
            roles.push((CollaborationRole::Domain, "Domain expertise grounds the discussion".into()));
            roles.push((CollaborationRole::Critic, "Critic provides necessary pushback".into()));
            roles.push((CollaborationRole::Synthesizer, "Synthesizer captures round outcomes".into()));
        }
        CollaborationTopology::DiscussionGroup => {
            roles.push((CollaborationRole::Moderator, "Discussion group needs a moderator".into()));
            roles.push((CollaborationRole::Domain, "Domain perspective required".into()));
            roles.push((CollaborationRole::Generalist, "Generalist provides breadth".into()));
        }
        CollaborationTopology::SelfOrganizing => {
            roles.push((CollaborationRole::Generalist, "Self-organizing team — generalists can fill any gap".into()));
        }
    }

    let reasons = roles.clone();
    (roles, reasons)
}

type FlagEntry = (String, bool, String);

fn derive_flags(c: &IntentComplexity, stakes: f64) -> (Vec<FlagEntry>, Vec<FlagEntry>) {
    let explicit_turns = stakes >= 0.3 || c.escalation_required;
    let round_synthesis = true; // always required
    let dissent_map = stakes >= 0.6 || c.reversibility_weight >= 0.8;
    let done_gate = stakes >= 0.3 || c.escalation_required;
    let report_owner = stakes >= 0.2;

    let flags = vec![
        (
            "require_explicit_turns".into(),
            explicit_turns,
            if c.escalation_required {
                "Escalation policy requires explicit turn tracking".into()
            } else if explicit_turns {
                format!("Stakes ({stakes:.2}) warrant explicit turn tracking")
            } else {
                "Low stakes — turns are informal".into()
            },
        ),
        (
            "require_round_synthesis".into(),
            round_synthesis,
            "Round synthesis always required — it's how the team captures progress".into(),
        ),
        (
            "require_dissent_map".into(),
            dissent_map,
            if dissent_map {
                format!(
                    "High stakes ({stakes:.2}) or irreversibility ({:.1}) — dissent must be visible",
                    c.reversibility_weight
                )
            } else {
                format!("Moderate stakes ({stakes:.2}) — dissent map optional")
            },
        ),
        (
            "require_done_gate".into(),
            done_gate,
            if c.escalation_required {
                "Escalation policy requires a formal done gate".into()
            } else if done_gate {
                format!("Stakes ({stakes:.2}) require a formal completion check")
            } else {
                "Low stakes — team decides when it's done".into()
            },
        ),
        (
            "require_report_owner".into(),
            report_owner,
            if report_owner {
                "Someone must own the final output".into()
            } else {
                "Very low stakes — no formal report needed".into()
            },
        ),
    ];

    let reasons = flags.clone();
    (flags, reasons)
}

fn compute_confidence(c: &IntentComplexity, stakes: f64) -> f64 {
    // High confidence when signals point strongly in one direction.
    // Low confidence when signals conflict (e.g. high stakes but tight deadline).
    let stake_clarity = (stakes - 0.5).abs() * 2.0; // 0=ambiguous, 1=clear
    let time_clarity = (c.time_pressure - 0.5).abs() * 2.0;
    let rev_clarity = c.reversibility_weight.abs(); // 0=reversible (clear), 1=irreversible (clear)

    let base = (stake_clarity + time_clarity + rev_clarity) / 3.0;

    // Conflicting signals reduce confidence.
    let conflict_penalty = if c.time_pressure >= 0.7 && stakes >= 0.7 {
        0.15 // tight deadline AND high stakes = conflicting pressure
    } else {
        0.0
    };

    (base - conflict_penalty).clamp(0.1, 0.95)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use organism_intent::ForbiddenAction;

    fn base_intent(now: DateTime<Utc>) -> IntentPacket {
        IntentPacket::new("Test outcome", now + Duration::days(7))
    }

    #[test]
    fn irreversible_intent_produces_enforced_supermajority() {
        let now = Utc::now();
        let mut intent = base_intent(now);
        intent.reversibility = Reversibility::Irreversible;
        intent.authority = vec!["board".into(), "legal".into()];
        intent.constraints = vec!["regulatory".into(), "compliance".into()];

        let derived = derive_charter(&intent, now);

        assert_eq!(derived.charter.discipline, CollaborationDiscipline::Enforced);
        assert!(matches!(
            derived.charter.consensus_rule,
            ConsensusRule::Supermajority | ConsensusRule::Unanimous
        ));
        assert!(derived.charter.require_dissent_map);
        assert!(derived.charter.require_done_gate);
    }

    #[test]
    fn reversible_low_stakes_produces_self_organizing() {
        let now = Utc::now();
        let intent = base_intent(now);

        let derived = derive_charter(&intent, now);

        assert_eq!(derived.charter.topology, CollaborationTopology::SelfOrganizing);
        assert_eq!(derived.charter.discipline, CollaborationDiscipline::Loose);
        assert_eq!(derived.charter.consensus_rule, ConsensusRule::AdvisoryOnly);
        assert_eq!(derived.charter.turn_cadence, TurnCadence::FigureItOut);
    }

    #[test]
    fn multi_authority_produces_panel_with_judges() {
        let now = Utc::now();
        let mut intent = base_intent(now);
        intent.authority = vec!["ceo".into(), "cfo".into(), "legal".into()];
        intent.reversibility = Reversibility::Partial;

        let derived = derive_charter(&intent, now);

        assert_eq!(derived.charter.topology, CollaborationTopology::Panel);
        assert!(
            derived
                .charter
                .expected_roles
                .contains(&CollaborationRole::Judge)
        );
        assert_eq!(derived.charter.turn_cadence, TurnCadence::LeadThenRoundRobin);
    }

    #[test]
    fn tight_deadline_produces_huddle() {
        let now = Utc::now();
        let intent = IntentPacket::new("Urgent decision", now + Duration::hours(2));

        let derived = derive_charter(&intent, now);

        assert_eq!(derived.charter.topology, CollaborationTopology::Huddle);
        assert_eq!(derived.charter.consensus_rule, ConsensusRule::LeadDecides);
        assert_eq!(derived.charter.turn_cadence, TurnCadence::RoundRobin);
    }

    #[test]
    fn escalate_expiry_forces_done_gate() {
        let now = Utc::now();
        let mut intent = base_intent(now);
        intent.expiry_action = ExpiryAction::Escalate;

        let derived = derive_charter(&intent, now);

        assert!(derived.charter.require_done_gate);
        assert!(derived.charter.require_explicit_turns);
    }

    #[test]
    fn moderate_complexity_produces_discussion_group() {
        let now = Utc::now();
        let mut intent = base_intent(now);
        intent.reversibility = Reversibility::Partial;
        intent.constraints = vec!["budget".into(), "timeline".into(), "scope".into()];
        intent.forbidden = vec![
            ForbiddenAction { action: "a".into(), reason: "r".into() },
            ForbiddenAction { action: "b".into(), reason: "r".into() },
            ForbiddenAction { action: "c".into(), reason: "r".into() },
        ];

        let derived = derive_charter(&intent, now);

        assert_eq!(derived.charter.topology, CollaborationTopology::DiscussionGroup);
        assert_eq!(derived.charter.discipline, CollaborationDiscipline::Moderated);
    }

    #[test]
    fn rationale_strings_are_non_empty() {
        let now = Utc::now();
        let intent = base_intent(now);

        let derived = derive_charter(&intent, now);

        assert!(!derived.rationale.topology_reason.is_empty());
        assert!(!derived.rationale.discipline_reason.is_empty());
        assert!(!derived.rationale.consensus_reason.is_empty());
        assert!(!derived.rationale.cadence_reason.is_empty());
        assert!(!derived.rationale.formation_reason.is_empty());
        assert!(!derived.rationale.role_reasons.is_empty());
        assert!(!derived.rationale.flag_reasons.is_empty());
    }

    #[test]
    fn confidence_higher_for_unambiguous_signals() {
        let now = Utc::now();

        let simple = base_intent(now);
        let simple_derived = derive_charter(&simple, now);

        let mut complex = base_intent(now);
        complex.reversibility = Reversibility::Irreversible;
        complex.authority = vec!["board".into(), "legal".into(), "cfo".into()];
        let complex_derived = derive_charter(&complex, now);

        assert!(simple_derived.confidence > 0.0);
        assert!(complex_derived.confidence > 0.0);
        assert!(complex_derived.confidence <= 0.95);
    }

    #[test]
    fn high_forbidden_density_increases_structure() {
        let now = Utc::now();
        let mut intent = base_intent(now);
        intent.forbidden = vec![
            ForbiddenAction { action: "a".into(), reason: "r".into() },
            ForbiddenAction { action: "b".into(), reason: "r".into() },
            ForbiddenAction { action: "c".into(), reason: "r".into() },
            ForbiddenAction { action: "d".into(), reason: "r".into() },
            ForbiddenAction { action: "e".into(), reason: "r".into() },
        ];
        intent.constraints = vec!["a".into(), "b".into(), "c".into(), "d".into()];
        intent.reversibility = Reversibility::Partial;

        let derived = derive_charter(&intent, now);

        assert!(matches!(
            derived.charter.topology,
            CollaborationTopology::Panel | CollaborationTopology::DiscussionGroup
        ));
    }

    #[test]
    fn derive_with_empty_priors_equals_derive_without() {
        let now = Utc::now();
        let intent = base_intent(now);

        let without = derive_charter(&intent, now);
        let with = derive_charter_with_priors(&intent, now, &[]);

        assert_eq!(without.charter.topology, with.charter.topology);
        assert_eq!(without.charter.discipline, with.charter.discipline);
    }

    #[test]
    fn derive_with_strong_priors_overrides_topology() {
        let now = Utc::now();
        let intent = base_intent(now); // would normally → SelfOrganizing

        let priors = vec![ShapeCalibration {
            problem_class: crate::shape_hypothesis::classify_problem(&intent),
            topology: CollaborationTopology::Huddle,
            prior_score: 0.5,
            posterior_score: 0.9,
            observation_count: 5,
        }];

        let derived = derive_charter_with_priors(&intent, now, &priors);

        assert_eq!(derived.charter.topology, CollaborationTopology::Huddle);
    }

    // ── Negative tests ────────────────────────────────────────────

    #[test]
    fn expired_intent_still_derives() {
        let now = Utc::now();
        let intent = IntentPacket::new("Past deadline", now - Duration::hours(1));

        let derived = derive_charter(&intent, now);

        // time_pressure should be 1.0 (maximally urgent)
        assert!((derived.intent_complexity.time_pressure - 1.0).abs() < f64::EPSILON);
        assert_eq!(derived.charter.topology, CollaborationTopology::Huddle);
    }

    #[test]
    fn zero_duration_intent() {
        let now = Utc::now();
        let intent = IntentPacket::new("Immediate", now);

        let derived = derive_charter(&intent, now);
        assert!(derived.intent_complexity.time_pressure >= 0.99);
    }

    // ── Proptests ─────────────────────────────────────────────────

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn derive_never_panics(
                n_constraints in 0_usize..20,
                n_authority in 0_usize..10,
                n_forbidden in 0_usize..10,
                hours_until_expiry in -24_i64..720,
                rev in 0_u8..3,
                expiry in 0_u8..3,
            ) {
                let now = Utc::now();
                let mut intent = IntentPacket::new(
                    "proptest",
                    now + Duration::hours(hours_until_expiry),
                );
                intent.constraints = (0..n_constraints).map(|i| format!("c{i}")).collect();
                intent.authority = (0..n_authority).map(|i| format!("a{i}")).collect();
                intent.forbidden = (0..n_forbidden).map(|i| ForbiddenAction {
                    action: format!("f{i}"),
                    reason: "test".into(),
                }).collect();
                intent.reversibility = match rev % 3 {
                    0 => Reversibility::Reversible,
                    1 => Reversibility::Partial,
                    _ => Reversibility::Irreversible,
                };
                intent.expiry_action = match expiry % 3 {
                    0 => ExpiryAction::Halt,
                    1 => ExpiryAction::Escalate,
                    _ => ExpiryAction::CompleteAndHalt,
                };

                let derived = derive_charter(&intent, now);

                prop_assert!(derived.confidence > 0.0);
                prop_assert!(derived.confidence <= 1.0);
                prop_assert!(!derived.rationale.topology_reason.is_empty());
                prop_assert!(!derived.charter.expected_roles.is_empty());
                prop_assert!(derived.charter.minimum_members >= 1);
            }

            #[test]
            fn complexity_values_bounded(
                n_constraints in 0_usize..100,
                n_authority in 0_usize..100,
                n_forbidden in 0_usize..100,
                hours in -100_i64..1000,
            ) {
                let now = Utc::now();
                let mut intent = IntentPacket::new("test", now + Duration::hours(hours));
                intent.constraints = (0..n_constraints).map(|i| format!("c{i}")).collect();
                intent.authority = (0..n_authority).map(|i| format!("a{i}")).collect();
                intent.forbidden = (0..n_forbidden).map(|i| ForbiddenAction {
                    action: format!("f{i}"),
                    reason: "t".into(),
                }).collect();

                let c = compute_complexity(&intent, now);

                prop_assert!(c.constraint_pressure >= 0.0 && c.constraint_pressure <= 1.0);
                prop_assert!(c.authority_breadth >= 0.0 && c.authority_breadth <= 1.0);
                prop_assert!(c.forbidden_density >= 0.0 && c.forbidden_density <= 1.0);
                prop_assert!(c.time_pressure >= 0.0 && c.time_pressure <= 1.0);
                prop_assert!(c.reversibility_weight >= 0.0 && c.reversibility_weight <= 1.0);
            }
        }
    }
}
