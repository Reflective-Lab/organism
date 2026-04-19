//! Collaboration models for multi-agent planning and research teams.
//!
//! Organism's planning loop is broader than a generic huddle. Different
//! workflows need different collaboration contracts:
//! - a strict huddle with explicit turns and synthesis checkpoints
//! - a moderated discussion group with some structure but softer commitments
//! - a demanding panel where roles, dissent, and decision policy are explicit
//! - a very loose OpenClaw-style swarm that is allowed to self-organize

use serde::{Deserialize, Serialize};

/// The overall collaboration shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollaborationTopology {
    Huddle,
    DiscussionGroup,
    Panel,
    OpenClaw,
}

impl CollaborationTopology {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Huddle => "huddle",
            Self::DiscussionGroup => "discussion_group",
            Self::Panel => "panel",
            Self::OpenClaw => "open_claw",
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
        matches!(self, Self::ReportWriter | Self::Synthesizer | Self::Lead)
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

/// Decision rule used to decide when a team is done or aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusRule {
    Majority,
    Supermajority,
    Unanimous,
    LeadDecides,
    AdvisoryOnly,
}

impl ConsensusRule {
    #[must_use]
    pub fn passes(self, yes_votes: usize, total_voters: usize) -> bool {
        match self {
            Self::Majority => yes_votes * 2 > total_voters,
            Self::Supermajority => yes_votes * 3 >= total_voters * 2,
            Self::Unanimous => yes_votes == total_voters,
            Self::LeadDecides => yes_votes >= 1,
            Self::AdvisoryOnly => true,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Majority => "majority",
            Self::Supermajority => "supermajority",
            Self::Unanimous => "unanimous",
            Self::LeadDecides => "lead_decides",
            Self::AdvisoryOnly => "advisory_only",
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

    /// Very loose OpenClaw-style self-organization.
    #[must_use]
    pub fn open_claw() -> Self {
        Self {
            topology: CollaborationTopology::OpenClaw,
            formation_mode: TeamFormationMode::OpenCall,
            discipline: CollaborationDiscipline::Loose,
            turn_cadence: TurnCadence::FigureItOut,
            consensus_rule: ConsensusRule::AdvisoryOnly,
            minimum_members: 1,
            require_explicit_turns: false,
            require_round_synthesis: false,
            require_dissent_map: false,
            require_done_gate: false,
            require_report_owner: false,
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
    fn open_claw_accepts_loose_team() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "generalist",
                "Generalist",
                CollaborationRole::Generalist,
            )],
        );
        assert!(CollaborationCharter::open_claw().validate(&team).is_ok());
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
}
