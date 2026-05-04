//! Collaboration models for multi-agent planning and research teams.
//!
//! Organism's planning loop is broader than a generic huddle. Different
//! workflows need different collaboration contracts:
//! - a strict huddle with explicit turns and synthesis checkpoints
//! - a moderated discussion group with some structure but softer commitments
//! - a demanding panel where roles, dissent, and decision policy are explicit
//! - a very loose self-organizing swarm that is allowed to self-organize

use serde::{Deserialize, Serialize};

/// Re-export the canonical decision rule from `converge-pack`.
///
/// `ConsensusRule` is a governance primitive at the pack contract layer; this
/// crate keeps the existing import path stable while sourcing the type from
/// upstream so there is exactly one definition in the tree.
pub use converge_pack::ConsensusRule;

/// The overall collaboration shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationTopology {
    Huddle,
    DiscussionGroup,
    Panel,
    SelfOrganizing,
}

impl CollaborationTopology {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Huddle => "huddle",
            Self::DiscussionGroup => "discussion_group",
            Self::Panel => "panel",
            Self::SelfOrganizing => "self_organizing",
        }
    }
}

/// How a team is assembled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamFormationMode {
    Curated,
    CapabilityMatched,
    SelfSelected,
    OpenCall,
}

impl TeamFormationMode {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Curated => "curated",
            Self::CapabilityMatched => "capability_matched",
            Self::SelfSelected => "self_selected",
            Self::OpenCall => "open_call",
        }
    }
}

/// How demanding the collaboration contract is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationDiscipline {
    Enforced,
    Moderated,
    Loose,
}

impl CollaborationDiscipline {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Enforced => "enforced",
            Self::Moderated => "moderated",
            Self::Loose => "loose",
        }
    }
}

/// Role a collaborator plays in the team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationRole {
    Lead,
    Domain,
    Critic,
    Synthesizer,
    Judge,
    ReportWriter,
    Moderator,
    Generalist,
    Observer,
}

impl CollaborationRole {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Domain => "domain",
            Self::Critic => "critic",
            Self::Synthesizer => "synthesizer",
            Self::Judge => "judge",
            Self::ReportWriter => "report_writer",
            Self::Moderator => "moderator",
            Self::Generalist => "generalist",
            Self::Observer => "observer",
        }
    }

    #[must_use]
    pub const fn contributes_in_rounds(self) -> bool {
        matches!(
            self,
            Self::Lead | Self::Domain | Self::Critic | Self::Synthesizer | Self::Generalist
        )
    }

    #[must_use]
    pub const fn votes_on_done_gate(self) -> bool {
        matches!(
            self,
            Self::Lead | Self::Domain | Self::Critic | Self::Judge | Self::Generalist
        )
    }

    #[must_use]
    pub const fn can_write_report(self) -> bool {
        matches!(
            self,
            Self::ReportWriter | Self::Synthesizer | Self::Lead | Self::Generalist
        )
    }
}

/// How turns should be organized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnCadence {
    RoundRobin,
    LeadThenRoundRobin,
    ModeratorThenRoundRobin,
    SynthesisOnly,
    FigureItOut,
}

impl TurnCadence {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RoundRobin => "round_robin",
            Self::LeadThenRoundRobin => "lead_then_round_robin",
            Self::ModeratorThenRoundRobin => "moderator_then_round_robin",
            Self::SynthesisOnly => "synthesis_only",
            Self::FigureItOut => "figure_it_out",
        }
    }
}

/// A collaborator in a planning or research team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaborationMember {
    pub id: String,
    pub display_name: String,
    pub role: CollaborationRole,
    pub persona: Option<String>,
}

impl CollaborationMember {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        role: CollaborationRole,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            role,
            persona: None,
        }
    }

    #[must_use]
    pub fn with_persona(mut self, persona: impl Into<String>) -> Self {
        self.persona = Some(persona.into());
        self
    }
}

/// Team formation input for a collaboration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamFormation {
    pub mode: TeamFormationMode,
    pub members: Vec<CollaborationMember>,
}

impl TeamFormation {
    #[must_use]
    pub fn new(mode: TeamFormationMode, members: Vec<CollaborationMember>) -> Self {
        Self { mode, members }
    }

    #[must_use]
    pub fn curated(members: Vec<CollaborationMember>) -> Self {
        Self::new(TeamFormationMode::Curated, members)
    }

    #[must_use]
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    #[must_use]
    pub fn roles(&self) -> Vec<CollaborationRole> {
        self.members.iter().map(|member| member.role).collect()
    }

    #[must_use]
    pub fn contributors(&self) -> Vec<&CollaborationMember> {
        self.members
            .iter()
            .filter(|member| member.role.contributes_in_rounds())
            .collect()
    }

    #[must_use]
    pub fn voters(&self) -> Vec<&CollaborationMember> {
        self.members
            .iter()
            .filter(|member| member.role.votes_on_done_gate())
            .collect()
    }

    #[must_use]
    pub fn report_owner(&self) -> Option<&CollaborationMember> {
        self.members
            .iter()
            .find(|member| member.role.can_write_report())
    }
}

/// Explicit rules for how a team collaborates.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaborationCharter {
    pub topology: CollaborationTopology,
    pub formation_mode: TeamFormationMode,
    pub discipline: CollaborationDiscipline,
    pub turn_cadence: TurnCadence,
    pub consensus_rule: ConsensusRule,
    pub minimum_members: usize,
    pub require_explicit_turns: bool,
    pub require_round_synthesis: bool,
    pub require_dissent_map: bool,
    pub require_done_gate: bool,
    pub require_report_owner: bool,
    pub expected_roles: Vec<CollaborationRole>,
}

impl CollaborationCharter {
    #[must_use]
    pub fn huddle() -> Self {
        Self {
            topology: CollaborationTopology::Huddle,
            formation_mode: TeamFormationMode::CapabilityMatched,
            discipline: CollaborationDiscipline::Enforced,
            turn_cadence: TurnCadence::RoundRobin,
            consensus_rule: ConsensusRule::Majority,
            minimum_members: 3,
            require_explicit_turns: true,
            require_round_synthesis: true,
            require_dissent_map: true,
            require_done_gate: true,
            require_report_owner: true,
            expected_roles: vec![
                CollaborationRole::Lead,
                CollaborationRole::Domain,
                CollaborationRole::Critic,
                CollaborationRole::Synthesizer,
            ],
        }
    }

    #[must_use]
    pub fn discussion_group() -> Self {
        Self {
            topology: CollaborationTopology::DiscussionGroup,
            formation_mode: TeamFormationMode::CapabilityMatched,
            discipline: CollaborationDiscipline::Moderated,
            turn_cadence: TurnCadence::ModeratorThenRoundRobin,
            consensus_rule: ConsensusRule::AdvisoryOnly,
            minimum_members: 3,
            require_explicit_turns: true,
            require_round_synthesis: true,
            require_dissent_map: false,
            require_done_gate: false,
            require_report_owner: true,
            expected_roles: vec![
                CollaborationRole::Moderator,
                CollaborationRole::Domain,
                CollaborationRole::Generalist,
            ],
        }
    }

    #[must_use]
    pub fn panel() -> Self {
        Self {
            topology: CollaborationTopology::Panel,
            formation_mode: TeamFormationMode::Curated,
            discipline: CollaborationDiscipline::Enforced,
            turn_cadence: TurnCadence::LeadThenRoundRobin,
            consensus_rule: ConsensusRule::Majority,
            minimum_members: 3,
            require_explicit_turns: true,
            require_round_synthesis: true,
            require_dissent_map: true,
            require_done_gate: true,
            require_report_owner: true,
            expected_roles: vec![
                CollaborationRole::Lead,
                CollaborationRole::Domain,
                CollaborationRole::Critic,
            ],
        }
    }

    /// Very loose self-organizing collaboration.
    #[must_use]
    pub fn self_organizing() -> Self {
        Self {
            topology: CollaborationTopology::SelfOrganizing,
            formation_mode: TeamFormationMode::OpenCall,
            discipline: CollaborationDiscipline::Loose,
            turn_cadence: TurnCadence::FigureItOut,
            consensus_rule: ConsensusRule::AdvisoryOnly,
            minimum_members: 1,
            require_explicit_turns: false,
            require_round_synthesis: true,
            require_dissent_map: false,
            require_done_gate: true,
            require_report_owner: true,
            expected_roles: vec![CollaborationRole::Generalist],
        }
    }

    #[must_use]
    pub fn with_consensus_rule(mut self, rule: ConsensusRule) -> Self {
        self.consensus_rule = rule;
        self
    }

    #[must_use]
    pub fn with_turn_cadence(mut self, cadence: TurnCadence) -> Self {
        self.turn_cadence = cadence;
        self
    }

    #[must_use]
    pub fn with_discipline(mut self, discipline: CollaborationDiscipline) -> Self {
        self.discipline = discipline;
        self
    }

    #[must_use]
    pub fn with_topology(mut self, topology: CollaborationTopology) -> Self {
        self.topology = topology;
        self
    }

    #[must_use]
    pub fn with_minimum_members(mut self, n: usize) -> Self {
        self.minimum_members = n;
        self
    }

    #[must_use]
    pub fn with_formation_mode(mut self, mode: TeamFormationMode) -> Self {
        self.formation_mode = mode;
        self
    }

    #[must_use]
    pub fn with_expected_roles(mut self, roles: Vec<CollaborationRole>) -> Self {
        self.expected_roles = roles;
        self
    }

    pub fn validate(&self, team: &TeamFormation) -> Result<(), CollaborationValidationError> {
        if team.members.len() < self.minimum_members {
            return Err(CollaborationValidationError::TooFewMembers {
                required: self.minimum_members,
                actual: team.members.len(),
            });
        }

        if team.mode != self.formation_mode && self.discipline == CollaborationDiscipline::Enforced
        {
            return Err(CollaborationValidationError::FormationModeMismatch {
                expected: self.formation_mode,
                actual: team.mode,
            });
        }

        for expected in &self.expected_roles {
            if !team.members.iter().any(|member| member.role == *expected) {
                return Err(CollaborationValidationError::MissingRole { role: *expected });
            }
        }

        if self.require_done_gate && team.voters().is_empty() {
            return Err(CollaborationValidationError::NoVoters);
        }

        if self.require_report_owner && team.report_owner().is_none() {
            return Err(CollaborationValidationError::MissingReportOwner);
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CollaborationValidationError {
    #[error("team requires at least {required} members, found {actual}")]
    TooFewMembers { required: usize, actual: usize },
    #[error("expected formation mode {expected:?}, found {actual:?}")]
    FormationModeMismatch {
        expected: TeamFormationMode,
        actual: TeamFormationMode,
    },
    #[error("team is missing required role {role:?}")]
    MissingRole { role: CollaborationRole },
    #[error("collaboration requires at least one voter")]
    NoVoters,
    #[error("collaboration requires a report owner")]
    MissingReportOwner,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_panel_team() -> TeamFormation {
        TeamFormation::curated(vec![
            CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
            CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
            CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
            CollaborationMember::new("judge", "Judge", CollaborationRole::Judge),
            CollaborationMember::new("writer", "Writer", CollaborationRole::ReportWriter),
        ])
    }

    #[test]
    fn panel_requires_expected_roles() {
        let team = sample_panel_team();
        assert!(CollaborationCharter::panel().validate(&team).is_ok());
    }

    #[test]
    fn self_organizing_accepts_loose_team() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "generalist",
                "Generalist",
                CollaborationRole::Generalist,
            )],
        );
        assert!(
            CollaborationCharter::self_organizing()
                .validate(&team)
                .is_ok()
        );
    }

    #[test]
    fn enforced_panel_rejects_missing_critic() {
        let team = TeamFormation::curated(vec![
            CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
            CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
            CollaborationMember::new("judge", "Judge", CollaborationRole::Judge),
        ]);
        assert_eq!(
            CollaborationCharter::panel().validate(&team),
            Err(CollaborationValidationError::MissingRole {
                role: CollaborationRole::Critic,
            })
        );
    }

    // ── Negative tests ────────────────────────────────────────────

    #[test]
    fn huddle_rejects_empty_team() {
        let team = TeamFormation::new(TeamFormationMode::CapabilityMatched, vec![]);
        assert_eq!(
            CollaborationCharter::huddle().validate(&team),
            Err(CollaborationValidationError::TooFewMembers {
                required: 3,
                actual: 0,
            })
        );
    }

    #[test]
    fn huddle_rejects_undersized_team() {
        let team = TeamFormation::new(
            TeamFormationMode::CapabilityMatched,
            vec![
                CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
                CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
            ],
        );
        assert_eq!(
            CollaborationCharter::huddle().validate(&team),
            Err(CollaborationValidationError::TooFewMembers {
                required: 3,
                actual: 2,
            })
        );
    }

    #[test]
    fn enforced_charter_rejects_formation_mode_mismatch() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![
                CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
                CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
                CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
            ],
        );
        assert_eq!(
            CollaborationCharter::panel().validate(&team),
            Err(CollaborationValidationError::FormationModeMismatch {
                expected: TeamFormationMode::Curated,
                actual: TeamFormationMode::OpenCall,
            })
        );
    }

    #[test]
    fn moderated_charter_ignores_formation_mode_mismatch() {
        let team = TeamFormation::new(
            TeamFormationMode::SelfSelected,
            vec![
                CollaborationMember::new("mod", "Mod", CollaborationRole::Moderator),
                CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
                CollaborationMember::new("gen", "Gen", CollaborationRole::Generalist),
            ],
        );
        assert!(
            CollaborationCharter::discussion_group()
                .validate(&team)
                .is_ok()
        );
    }

    #[test]
    fn done_gate_requires_at_least_one_voter() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "writer",
                "Writer",
                CollaborationRole::ReportWriter,
            )],
        );
        let charter =
            CollaborationCharter::self_organizing().with_consensus_rule(ConsensusRule::Majority);

        let result = charter.validate(&team);
        assert_eq!(
            result,
            Err(CollaborationValidationError::MissingRole {
                role: CollaborationRole::Generalist,
            })
        );
    }

    #[test]
    fn team_of_only_observers_has_no_voters() {
        let mut charter = CollaborationCharter::self_organizing();
        charter.expected_roles = vec![];
        charter.require_report_owner = false;

        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![
                CollaborationMember::new("obs1", "Observer1", CollaborationRole::Observer),
                CollaborationMember::new("obs2", "Observer2", CollaborationRole::Observer),
            ],
        );
        assert_eq!(
            charter.validate(&team),
            Err(CollaborationValidationError::NoVoters)
        );
    }

    #[test]
    fn missing_report_owner_rejected() {
        let mut charter = CollaborationCharter::self_organizing();
        charter.expected_roles = vec![];
        charter.require_done_gate = false;

        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "critic",
                "Critic",
                CollaborationRole::Critic,
            )],
        );
        assert_eq!(
            charter.validate(&team),
            Err(CollaborationValidationError::MissingReportOwner)
        );
    }

    // ── ConsensusRule edge cases ──────────────────────────────────

    #[test]
    fn majority_needs_strict_majority() {
        assert!(!ConsensusRule::Majority.passes(2, 4));
        assert!(ConsensusRule::Majority.passes(3, 4));
        assert!(!ConsensusRule::Majority.passes(0, 1));
        assert!(ConsensusRule::Majority.passes(1, 1));
    }

    #[test]
    fn supermajority_threshold() {
        assert!(!ConsensusRule::Supermajority.passes(1, 3));
        assert!(ConsensusRule::Supermajority.passes(2, 3));
        assert!(ConsensusRule::Supermajority.passes(4, 6));
        assert!(!ConsensusRule::Supermajority.passes(3, 6));
    }

    #[test]
    fn unanimous_requires_all() {
        assert!(ConsensusRule::Unanimous.passes(5, 5));
        assert!(!ConsensusRule::Unanimous.passes(4, 5));
        assert!(!ConsensusRule::Unanimous.passes(0, 1));
    }

    #[test]
    fn lead_decides_needs_one_yes() {
        assert!(ConsensusRule::LeadDecides.passes(1, 100));
        assert!(!ConsensusRule::LeadDecides.passes(0, 100));
    }

    #[test]
    fn advisory_always_passes() {
        assert!(ConsensusRule::AdvisoryOnly.passes(0, 0));
        assert!(ConsensusRule::AdvisoryOnly.passes(0, 100));
    }

    #[test]
    fn consensus_with_zero_voters() {
        assert!(!ConsensusRule::Majority.passes(0, 0));
        assert!(ConsensusRule::Unanimous.passes(0, 0));
        assert!(!ConsensusRule::LeadDecides.passes(0, 0));
    }

    // ── Role capability matrix ────────────────────────────────────

    #[test]
    fn observer_cannot_contribute_vote_or_write() {
        assert!(!CollaborationRole::Observer.contributes_in_rounds());
        assert!(!CollaborationRole::Observer.votes_on_done_gate());
        assert!(!CollaborationRole::Observer.can_write_report());
    }

    #[test]
    fn report_writer_can_write_but_not_vote() {
        assert!(!CollaborationRole::ReportWriter.contributes_in_rounds());
        assert!(!CollaborationRole::ReportWriter.votes_on_done_gate());
        assert!(CollaborationRole::ReportWriter.can_write_report());
    }

    #[test]
    fn judge_can_vote_but_not_contribute_or_write() {
        assert!(!CollaborationRole::Judge.contributes_in_rounds());
        assert!(CollaborationRole::Judge.votes_on_done_gate());
        assert!(!CollaborationRole::Judge.can_write_report());
    }

    #[test]
    fn moderator_has_no_capabilities() {
        assert!(!CollaborationRole::Moderator.contributes_in_rounds());
        assert!(!CollaborationRole::Moderator.votes_on_done_gate());
        assert!(!CollaborationRole::Moderator.can_write_report());
    }

    #[test]
    fn generalist_can_do_everything() {
        assert!(CollaborationRole::Generalist.contributes_in_rounds());
        assert!(CollaborationRole::Generalist.votes_on_done_gate());
        assert!(CollaborationRole::Generalist.can_write_report());
    }

    // ── TeamFormation helpers ─────────────────────────────────────

    #[test]
    fn contributors_excludes_non_contributing_roles() {
        let team = sample_panel_team();
        let contributors = team.contributors();
        assert!(contributors.iter().all(|m| m.role.contributes_in_rounds()));
        assert!(!contributors.iter().any(
            |m| m.role == CollaborationRole::Judge || m.role == CollaborationRole::ReportWriter
        ));
    }

    #[test]
    fn voters_excludes_non_voting_roles() {
        let team = sample_panel_team();
        let voters = team.voters();
        assert!(voters.iter().all(|m| m.role.votes_on_done_gate()));
    }

    #[test]
    fn report_owner_picks_first_capable() {
        let team = TeamFormation::curated(vec![
            CollaborationMember::new("observer", "Observer", CollaborationRole::Observer),
            CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
            CollaborationMember::new("writer", "Writer", CollaborationRole::ReportWriter),
        ]);
        assert_eq!(team.report_owner().unwrap().id, "writer");
    }

    #[test]
    fn report_owner_none_when_no_capable() {
        let team = TeamFormation::curated(vec![
            CollaborationMember::new("observer", "Observer", CollaborationRole::Observer),
            CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
        ]);
        assert!(team.report_owner().is_none());
    }

    #[test]
    fn member_with_persona() {
        let member = CollaborationMember::new("critic", "Red Team", CollaborationRole::Critic)
            .with_persona("Aggressive skeptic who challenges every assumption");
        assert_eq!(
            member.persona.as_deref(),
            Some("Aggressive skeptic who challenges every assumption")
        );
    }

    // ── Charter preset invariants ─────────────────────────────────

    #[test]
    fn all_presets_require_round_synthesis() {
        assert!(CollaborationCharter::huddle().require_round_synthesis);
        assert!(CollaborationCharter::discussion_group().require_round_synthesis);
        assert!(CollaborationCharter::panel().require_round_synthesis);
        assert!(CollaborationCharter::self_organizing().require_round_synthesis);
    }

    #[test]
    fn self_organizing_is_the_only_single_member_preset() {
        assert_eq!(CollaborationCharter::self_organizing().minimum_members, 1);
        assert!(CollaborationCharter::huddle().minimum_members >= 3);
        assert!(CollaborationCharter::discussion_group().minimum_members >= 3);
        assert!(CollaborationCharter::panel().minimum_members >= 3);
    }

    // ── Proptest ──────────────────────────────────────────────────

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        fn arb_consensus_rule() -> impl Strategy<Value = ConsensusRule> {
            prop_oneof![
                Just(ConsensusRule::Majority),
                Just(ConsensusRule::Supermajority),
                Just(ConsensusRule::Unanimous),
                Just(ConsensusRule::LeadDecides),
                Just(ConsensusRule::AdvisoryOnly),
            ]
        }

        proptest! {
            #[test]
            fn consensus_yes_votes_never_exceed_total(
                rule in arb_consensus_rule(),
                total in 0_usize..100,
                yes in 0_usize..100,
            ) {
                if yes <= total {
                    let _ = rule.passes(yes, total);
                }
            }

            #[test]
            fn unanimous_passes_iff_all_vote_yes(
                total in 0_usize..50,
                yes in 0_usize..50,
            ) {
                prop_assume!(yes <= total);
                let result = ConsensusRule::Unanimous.passes(yes, total);
                prop_assert_eq!(result, yes == total);
            }

            #[test]
            fn advisory_always_passes_regardless_of_votes(
                total in 0_usize..100,
                yes in 0_usize..100,
            ) {
                prop_assert!(ConsensusRule::AdvisoryOnly.passes(yes, total));
            }

            #[test]
            fn majority_monotonic_in_yes_votes(
                total in 1_usize..50,
                yes1_frac in 0.0..=1.0_f64,
                yes2_frac in 0.0..=1.0_f64,
            ) {
                let (lo, hi) = if yes1_frac <= yes2_frac {
                    (yes1_frac, yes2_frac)
                } else {
                    (yes2_frac, yes1_frac)
                };
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
                let yes1 = (lo * total as f64) as usize;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
                let yes2 = (hi * total as f64) as usize;
                if ConsensusRule::Majority.passes(yes1, total) {
                    prop_assert!(ConsensusRule::Majority.passes(yes2, total));
                }
            }

            #[test]
            fn supermajority_is_stricter_than_majority(
                total in 1_usize..50,
                yes in 0_usize..50,
            ) {
                prop_assume!(yes <= total);
                if ConsensusRule::Supermajority.passes(yes, total) {
                    prop_assert!(ConsensusRule::Majority.passes(yes, total));
                }
            }
        }
    }
}
