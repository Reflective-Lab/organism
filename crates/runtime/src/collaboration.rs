//! Shared collaboration runtime helpers.
//!
//! Products can define their own participant metadata while reusing Organism's
//! team formation and collaboration charter semantics.

use std::collections::HashMap;

use organism_pack::{
    CollaborationCharter, CollaborationRole, CollaborationTopology, CollaborationValidationError,
    ConsensusRule, TeamFormation, TurnCadence,
};

/// Runtime-side participant contract.
pub trait CollaborationParticipant: Clone {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn role(&self) -> CollaborationRole;
}

/// Record of a topology transition.
#[derive(Debug, Clone)]
pub struct TransitionRecord {
    pub from: CollaborationTopology,
    pub to: CollaborationTopology,
    pub reason: String,
    pub at_cycle: u32,
}

/// Validated runtime collaboration view.
#[derive(Debug, Clone)]
pub struct CollaborationRunner<P: CollaborationParticipant> {
    charter: CollaborationCharter,
    team: TeamFormation,
    members_by_id: HashMap<String, P>,
    contributors: Vec<P>,
    voters: Vec<P>,
    report_owner: Option<P>,
    transitions: Vec<TransitionRecord>,
}

impl<P: CollaborationParticipant> CollaborationRunner<P> {
    pub fn new(
        team: TeamFormation,
        charter: CollaborationCharter,
        participants: Vec<P>,
    ) -> Result<Self, CollaborationRunnerError> {
        charter
            .validate(&team)
            .map_err(CollaborationRunnerError::InvalidTeam)?;

        let mut members_by_id = HashMap::new();
        for participant in participants {
            members_by_id.insert(participant.id().to_string(), participant);
        }

        for member in &team.members {
            let Some(participant) = members_by_id.get(&member.id) else {
                return Err(CollaborationRunnerError::MissingParticipant {
                    id: member.id.clone(),
                    display_name: member.display_name.clone(),
                });
            };

            if participant.role() != member.role {
                return Err(CollaborationRunnerError::RoleMismatch {
                    id: member.id.clone(),
                    expected: member.role,
                    actual: participant.role(),
                });
            }
        }

        let contributors = members_by_id
            .values()
            .filter(|participant| participant.role().contributes_in_rounds())
            .cloned()
            .collect();
        let voters = members_by_id
            .values()
            .filter(|participant| participant.role().votes_on_done_gate())
            .cloned()
            .collect();
        let report_owner = members_by_id
            .values()
            .find(|participant| participant.role().can_write_report())
            .cloned();

        Ok(Self {
            charter,
            team,
            members_by_id,
            contributors,
            voters,
            report_owner,
            transitions: Vec::new(),
        })
    }

    #[must_use]
    pub fn team(&self) -> &TeamFormation {
        &self.team
    }

    #[must_use]
    pub fn charter(&self) -> &CollaborationCharter {
        &self.charter
    }

    #[must_use]
    pub fn member(&self, id: &str) -> Option<&P> {
        self.members_by_id.get(id)
    }

    #[must_use]
    pub fn contributors(&self) -> &[P] {
        &self.contributors
    }

    #[must_use]
    pub fn voters(&self) -> &[P] {
        &self.voters
    }

    #[must_use]
    pub fn report_owner(&self) -> Option<&P> {
        self.report_owner.as_ref()
    }

    #[must_use]
    pub fn require_round_synthesis(&self) -> bool {
        self.charter.require_round_synthesis
    }

    #[must_use]
    pub fn require_done_gate(&self) -> bool {
        self.charter.require_done_gate
    }

    #[must_use]
    pub fn require_dissent_map(&self) -> bool {
        self.charter.require_dissent_map
    }

    #[must_use]
    pub fn require_report_owner(&self) -> bool {
        self.charter.require_report_owner
    }

    #[must_use]
    pub fn consensus_rule(&self) -> ConsensusRule {
        self.charter.consensus_rule
    }

    #[must_use]
    pub fn turn_cadence(&self) -> TurnCadence {
        self.charter.turn_cadence
    }

    #[must_use]
    pub fn transitions(&self) -> &[TransitionRecord] {
        &self.transitions
    }

    /// Transition to a new charter with a new team and participants.
    /// Re-validates everything and rebuilds internal state.
    pub fn transition(
        &mut self,
        new_charter: CollaborationCharter,
        new_team: TeamFormation,
        new_participants: Vec<P>,
        reason: String,
        at_cycle: u32,
    ) -> Result<(), CollaborationRunnerError> {
        let from = self.charter.topology;
        let to = new_charter.topology;

        let rebuilt = Self::new(new_team, new_charter, new_participants)?;

        self.transitions.push(TransitionRecord {
            from,
            to,
            reason,
            at_cycle,
        });

        self.charter = rebuilt.charter;
        self.team = rebuilt.team;
        self.members_by_id = rebuilt.members_by_id;
        self.contributors = rebuilt.contributors;
        self.voters = rebuilt.voters;
        self.report_owner = rebuilt.report_owner;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CollaborationRunnerError {
    #[error(transparent)]
    InvalidTeam(#[from] CollaborationValidationError),
    #[error("missing runtime participant '{display_name}' ({id})")]
    MissingParticipant { id: String, display_name: String },
    #[error("participant '{id}' has role {actual:?}, expected {expected:?}")]
    RoleMismatch {
        id: String,
        expected: CollaborationRole,
        actual: CollaborationRole,
    },
}

#[cfg(test)]
mod tests {
    use organism_pack::{CollaborationMember, CollaborationRole, TeamFormation, TeamFormationMode};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestParticipant {
        id: String,
        display_name: String,
        role: CollaborationRole,
    }

    impl CollaborationParticipant for TestParticipant {
        fn id(&self) -> &str {
            &self.id
        }

        fn display_name(&self) -> &str {
            &self.display_name
        }

        fn role(&self) -> CollaborationRole {
            self.role
        }
    }

    #[test]
    fn runner_builds_contributors_and_voters() {
        let team = TeamFormation::curated(vec![
            CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
            CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
            CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
            CollaborationMember::new("writer", "Writer", CollaborationRole::ReportWriter),
        ]);
        let participants = vec![
            TestParticipant {
                id: "lead".into(),
                display_name: "Lead".into(),
                role: CollaborationRole::Lead,
            },
            TestParticipant {
                id: "domain".into(),
                display_name: "Domain".into(),
                role: CollaborationRole::Domain,
            },
            TestParticipant {
                id: "critic".into(),
                display_name: "Critic".into(),
                role: CollaborationRole::Critic,
            },
            TestParticipant {
                id: "writer".into(),
                display_name: "Writer".into(),
                role: CollaborationRole::ReportWriter,
            },
        ];

        let runner = CollaborationRunner::new(team, CollaborationCharter::panel(), participants)
            .expect("runner should build");

        assert_eq!(runner.contributors().len(), 3);
        assert_eq!(runner.voters().len(), 3);
        assert!(runner.report_owner().unwrap().role().can_write_report());
    }

    #[test]
    fn runner_rejects_missing_runtime_participant() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "generalist",
                "Generalist",
                CollaborationRole::Generalist,
            )],
        );
        let err = CollaborationRunner::<TestParticipant>::new(
            team,
            CollaborationCharter::self_organizing(),
            vec![],
        )
        .expect_err("runner should reject missing participant");

        assert!(matches!(
            err,
            CollaborationRunnerError::MissingParticipant { .. }
        ));
    }

    #[test]
    fn runner_rejects_role_mismatch() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "gen",
                "Generalist",
                CollaborationRole::Generalist,
            )],
        );
        let participants = vec![TestParticipant {
            id: "gen".into(),
            display_name: "Generalist".into(),
            role: CollaborationRole::Critic,
        }];
        let err = CollaborationRunner::new(
            team,
            CollaborationCharter::self_organizing(),
            participants,
        )
        .expect_err("runner should reject role mismatch");

        assert!(matches!(
            err,
            CollaborationRunnerError::RoleMismatch {
                expected: CollaborationRole::Generalist,
                actual: CollaborationRole::Critic,
                ..
            }
        ));
    }

    #[test]
    fn runner_propagates_charter_validation_errors() {
        let team = TeamFormation::curated(vec![]);
        let err =
            CollaborationRunner::<TestParticipant>::new(team, CollaborationCharter::panel(), vec![])
                .expect_err("runner should propagate validation error");

        assert!(matches!(err, CollaborationRunnerError::InvalidTeam(_)));
    }

    #[test]
    fn runner_member_lookup_returns_none_for_unknown_id() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![CollaborationMember::new(
                "gen",
                "Gen",
                CollaborationRole::Generalist,
            )],
        );
        let participants = vec![TestParticipant {
            id: "gen".into(),
            display_name: "Gen".into(),
            role: CollaborationRole::Generalist,
        }];
        let runner = CollaborationRunner::new(
            team,
            CollaborationCharter::self_organizing(),
            participants,
        )
        .unwrap();

        assert!(runner.member("gen").is_some());
        assert!(runner.member("nonexistent").is_none());
    }

    #[test]
    fn runner_delegates_charter_flags() {
        let charter = CollaborationCharter::huddle();
        let team = TeamFormation::new(
            TeamFormationMode::CapabilityMatched,
            vec![
                CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
                CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
                CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
                CollaborationMember::new("synth", "Synth", CollaborationRole::Synthesizer),
            ],
        );
        let participants = vec![
            TestParticipant {
                id: "lead".into(),
                display_name: "Lead".into(),
                role: CollaborationRole::Lead,
            },
            TestParticipant {
                id: "domain".into(),
                display_name: "Domain".into(),
                role: CollaborationRole::Domain,
            },
            TestParticipant {
                id: "critic".into(),
                display_name: "Critic".into(),
                role: CollaborationRole::Critic,
            },
            TestParticipant {
                id: "synth".into(),
                display_name: "Synth".into(),
                role: CollaborationRole::Synthesizer,
            },
        ];
        let runner = CollaborationRunner::new(team, charter, participants).unwrap();

        assert!(runner.require_round_synthesis());
        assert!(runner.require_done_gate());
        assert!(runner.require_dissent_map());
        assert!(runner.require_report_owner());
        assert_eq!(runner.consensus_rule(), ConsensusRule::Majority);
        assert_eq!(runner.turn_cadence(), TurnCadence::RoundRobin);
    }

    #[test]
    fn runner_report_owner_prefers_synthesizer_over_lead() {
        let team = TeamFormation::new(
            TeamFormationMode::OpenCall,
            vec![
                CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
                CollaborationMember::new("synth", "Synth", CollaborationRole::Synthesizer),
                CollaborationMember::new("gen", "Gen", CollaborationRole::Generalist),
            ],
        );
        let participants = vec![
            TestParticipant {
                id: "lead".into(),
                display_name: "Lead".into(),
                role: CollaborationRole::Lead,
            },
            TestParticipant {
                id: "synth".into(),
                display_name: "Synth".into(),
                role: CollaborationRole::Synthesizer,
            },
            TestParticipant {
                id: "gen".into(),
                display_name: "Gen".into(),
                role: CollaborationRole::Generalist,
            },
        ];
        let mut charter = CollaborationCharter::self_organizing();
        charter.expected_roles = vec![];
        let runner = CollaborationRunner::new(team, charter, participants).unwrap();

        let owner = runner.report_owner().unwrap();
        assert!(owner.role().can_write_report());
    }
}
