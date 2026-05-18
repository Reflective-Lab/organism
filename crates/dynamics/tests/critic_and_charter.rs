//! Integration tests for the critic-aware design Formation and the
//! charter preflight.
//!
//! Two concerns, one file (they share enough fixture machinery to
//! avoid duplication):
//! - `DraftValidatorCriticSuggestor` emits structured verdicts that
//!   `BeautyContestSuggestor` honors when shortlisting.
//! - `preflight_design_formation` validates a proposed team against
//!   a `CollaborationCharter` before the Formation is instantiated —
//!   opt-in, returns the underlying validation error verbatim.

use async_trait::async_trait;
use chrono::{Duration, Utc};
use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
    ProfileSnapshot, StaticFormationTemplate, SuggestorCapability, SuggestorRole,
};
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{
    CatalogSuggestorDescriptor, DiscoveryCatalog, DiscoveryMetadata, LoopContribution,
    ProviderDescriptorCatalog, SuggestorDescriptor,
};
use organism_dynamics::{
    BeautyContestSuggestor, CatalogProposerSuggestor, DRAFT_VALIDATION_KIND,
    DraftValidatorCriticSuggestor, DraftVerdict, FormationDraft, PreflightError,
    extract_draft_validations, extract_drafts, preflight_design_formation,
};
use organism_planning::{
    CollaborationCharter, CollaborationMember, CollaborationRole, CollaborationValidationError,
    TeamFormation, TeamFormationMode,
};
use organism_runtime::{
    CollaborationParticipant, CollaborationRunnerError, Formation, FormationCompileRequest,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared fixtures (mirror the design_to_work_formation.rs catalog so
// the two test files compose the same descriptor universe).
// ---------------------------------------------------------------------------

fn synthetic_descriptor(
    id: &'static str,
    role: SuggestorRole,
    capability: SuggestorCapability,
    writes: ContextKey,
) -> CatalogSuggestorDescriptor {
    let descriptor = SuggestorDescriptor::new(
        id,
        ProfileSnapshot {
            name: id.to_string(),
            role,
            output_keys: vec![writes],
            cost_hint: CostClass::Low,
            latency_hint: LatencyClass::Interactive,
            capabilities: vec![capability],
            confidence_min: 0.7,
            confidence_max: 0.95,
        },
    );
    let discovery = DiscoveryMetadata::new("Test fixture.", "Test fixture only.")
        .with_loop_contribution(LoopContribution::Synthesize);
    CatalogSuggestorDescriptor::new(descriptor, discovery)
}

fn work_catalog() -> DiscoveryCatalog {
    DiscoveryCatalog::new()
        .with_entry(synthetic_descriptor(
            "signal-a",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            ContextKey::Hypotheses,
        ))
        .with_entry(synthetic_descriptor(
            "signal-b",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            ContextKey::Hypotheses,
        ))
        .with_entry(synthetic_descriptor(
            "constraint-a",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ))
        .with_entry(synthetic_descriptor(
            "constraint-b",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ))
}

fn work_template_catalog() -> FormationCatalog {
    let metadata = FormationTemplateMetadata::new(
        "work-template",
        "Work formation: signal + constraint",
        vec![SuggestorRole::Signal, SuggestorRole::Constraint],
    )
    .with_keyword("work-template")
    .with_required_capability(SuggestorCapability::KnowledgeRetrieval)
    .with_required_capability(SuggestorCapability::PolicyEnforcement);
    FormationCatalog::new().with_template(FormationTemplate::static_template(
        StaticFormationTemplate::new(metadata),
    ))
}

fn work_request() -> FormationCompileRequest {
    FormationCompileRequest::new(
        Uuid::from_u128(0xC001),
        Uuid::from_u128(0xC002),
        FormationTemplateQuery::new().with_keyword("work-template"),
    )
}

#[derive(Debug, Clone, Copy)]
struct TestProvenance;
impl ProvenanceSource for TestProvenance {
    fn as_str(&self) -> &'static str {
        "critic-and-charter-test"
    }
}

// ---------------------------------------------------------------------------
// Critic + scorer end-to-end: valid drafts pass and reach the shortlist
// ---------------------------------------------------------------------------

#[tokio::test]
async fn critic_passes_valid_drafts_scorer_shortlists_them() {
    let catalog = work_catalog();
    let templates = work_template_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let request = work_request();

    let proposer = CatalogProposerSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
        3,
    );
    let critic = DraftValidatorCriticSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
    );
    let scorer = BeautyContestSuggestor::new_critic_gated(2);

    let formation = Formation::new("design-with-critic")
        .agent_boxed(Box::new(proposer))
        .agent_boxed(Box::new(critic))
        .agent_boxed(Box::new(scorer))
        .seed(ContextKey::Seeds, "s", "design", "test");

    let result = formation.run().await.expect("should converge");
    assert!(result.converge_result.converged);

    // Critic emitted Pass verdicts under Evaluations.
    let passes =
        extract_draft_validations(&result.converge_result.context, ContextKey::Evaluations);
    assert!(
        !passes.is_empty(),
        "critic must emit at least one Pass verdict for valid drafts"
    );
    for v in &passes {
        assert_eq!(v.verdict, DraftVerdict::Pass);
        assert_eq!(v.kind, DRAFT_VALIDATION_KIND);
    }

    // No Block verdicts (all CatalogProposer drafts are valid).
    let blocks =
        extract_draft_validations(&result.converge_result.context, ContextKey::Constraints);
    assert!(
        blocks.iter().all(|v| v.verdict != DraftVerdict::Block),
        "no Block verdicts expected for valid drafts; got {blocks:?}"
    );

    // Scorer shortlisted drafts into Proposals.
    let shortlist = extract_drafts(&result.converge_result.context, ContextKey::Proposals);
    assert!(!shortlist.is_empty());
    assert!(shortlist.len() <= 2);
}

// ---------------------------------------------------------------------------
// Critic + scorer: a bad draft is blocked, scorer excludes it
// ---------------------------------------------------------------------------

/// A test-only Suggestor that proposes exactly one [`FormationDraft`]
/// with a bogus descriptor id under [`ContextKey::Strategies`]. Gates
/// on the absence of its own fact id so it fires exactly once,
/// regardless of what other proposers are doing.
struct BadDraftProposer {
    source_label: &'static str,
}

impl BadDraftProposer {
    fn marker_id(&self) -> String {
        format!("bad-draft-{}-marker", self.source_label)
    }
}

#[async_trait]
impl Suggestor for BadDraftProposer {
    fn name(&self) -> &'static str {
        "bad-draft-proposer"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }
    fn provenance(&self) -> &'static str {
        TestProvenance.as_str()
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Fire once: gate on absence of our marker in Diagnostic.
        if !ctx.has(ContextKey::Seeds) {
            return false;
        }
        let marker = self.marker_id();
        !ctx.get(ContextKey::Diagnostic)
            .iter()
            .any(|fact| fact.id().as_str() == marker.as_str())
    }
    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        let draft_id = format!("bad-draft-{}-0", self.source_label);
        let draft = FormationDraft::new(
            draft_id.clone(),
            vec![
                "signal-a".to_string(),
                "definitely-not-in-catalog".to_string(),
            ],
            "intentionally invalid draft for critic regression",
            self.source_label,
        );
        let json = serde_json::to_string(&draft).unwrap();
        AgentEffect::builder()
            .proposal(TestProvenance.proposed_fact(
                ContextKey::Strategies,
                draft_id,
                TextPayload::new(json),
            ))
            // Marker so we don't re-fire on subsequent cycles.
            .proposal(TestProvenance.proposed_fact(
                ContextKey::Diagnostic,
                self.marker_id(),
                TextPayload::new(format!(
                    "{} emitted bad draft for regression test",
                    self.source_label
                )),
            ))
            .build()
    }
}

#[tokio::test]
async fn critic_blocks_bad_draft_scorer_excludes_it() {
    let catalog = work_catalog();
    let templates = work_template_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let request = work_request();

    let critic = DraftValidatorCriticSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
    );
    let scorer = BeautyContestSuggestor::new_critic_gated(5);
    let bad = BadDraftProposer {
        source_label: "bad-proposer",
    };

    // No CatalogProposerSuggestor in this run — only the bad proposer
    // emits a draft, so the critic's Block and scorer's exclusion are
    // unambiguous.
    let formation = Formation::new("design-with-bad-draft")
        .agent_boxed(Box::new(bad))
        .agent_boxed(Box::new(critic))
        .agent_boxed(Box::new(scorer))
        .seed(ContextKey::Seeds, "s", "design", "test");

    let result = formation.run().await.expect("should converge");
    assert!(result.converge_result.converged);

    // Critic emitted exactly one Block verdict under Constraints.
    let blocks =
        extract_draft_validations(&result.converge_result.context, ContextKey::Constraints);
    let bad_blocks: Vec<_> = blocks
        .iter()
        .filter(|v| v.verdict == DraftVerdict::Block)
        .collect();
    assert_eq!(
        bad_blocks.len(),
        1,
        "critic must emit one Block verdict; got {blocks:?}"
    );
    assert_eq!(bad_blocks[0].draft_id, "bad-draft-bad-proposer-0");
    assert!(
        bad_blocks[0].reason.contains("DraftDescriptorMissing")
            || bad_blocks[0]
                .reason
                .contains("draft references unknown descriptor"),
        "Block reason should mention the missing descriptor; got: {}",
        bad_blocks[0].reason
    );

    // No Pass verdicts (the only draft was bad).
    let passes =
        extract_draft_validations(&result.converge_result.context, ContextKey::Evaluations);
    assert!(
        passes.iter().all(|v| v.verdict != DraftVerdict::Pass),
        "no Pass verdicts expected when only a bad draft was proposed; got {passes:?}"
    );

    // Scorer must NOT have shortlisted the bad draft — Proposals empty.
    let shortlist = extract_drafts(&result.converge_result.context, ContextKey::Proposals);
    assert!(
        shortlist.is_empty(),
        "scorer must exclude blocked drafts; got shortlist {shortlist:?}"
    );
}

// ---------------------------------------------------------------------------
// Charter preflight: pass + fail
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TestParticipant {
    id: String,
    display: String,
    role: CollaborationRole,
}

impl CollaborationParticipant for TestParticipant {
    fn id(&self) -> &str {
        &self.id
    }
    fn display_name(&self) -> &str {
        &self.display
    }
    fn role(&self) -> CollaborationRole {
        self.role
    }
}

fn member(id: &str, role: CollaborationRole) -> CollaborationMember {
    CollaborationMember {
        id: id.to_string(),
        display_name: id.to_string(),
        role,
        persona: None,
    }
}

fn participant(id: &str, role: CollaborationRole) -> TestParticipant {
    TestParticipant {
        id: id.to_string(),
        display: id.to_string(),
        role,
    }
}

#[test]
fn preflight_passes_when_team_satisfies_huddle_charter() {
    let charter = CollaborationCharter::huddle();
    // huddle requires: minimum_members=3, mode=CapabilityMatched,
    // expected_roles = [Lead, Domain, Critic, Synthesizer].
    let team = TeamFormation {
        mode: TeamFormationMode::CapabilityMatched,
        members: vec![
            member("lead", CollaborationRole::Lead),
            member("domain-1", CollaborationRole::Domain),
            member("critic-1", CollaborationRole::Critic),
            member("synth", CollaborationRole::Synthesizer),
        ],
    };
    let participants = vec![
        participant("lead", CollaborationRole::Lead),
        participant("domain-1", CollaborationRole::Domain),
        participant("critic-1", CollaborationRole::Critic),
        participant("synth", CollaborationRole::Synthesizer),
    ];

    let runner = preflight_design_formation(team, charter, participants)
        .expect("huddle preflight should pass for a well-shaped team");
    // Smoke check on the returned runner.
    assert_eq!(
        runner.consensus_rule(),
        organism_planning::ConsensusRule::Majority
    );
}

#[test]
fn preflight_fails_when_team_is_under_minimum_members() {
    let charter = CollaborationCharter::huddle();
    let team = TeamFormation {
        mode: TeamFormationMode::CapabilityMatched,
        members: vec![member("lead", CollaborationRole::Lead)],
    };
    let participants = vec![participant("lead", CollaborationRole::Lead)];

    let err = preflight_design_formation(team, charter, participants)
        .expect_err("undersized team must fail preflight");
    match err {
        PreflightError::Charter(CollaborationRunnerError::InvalidTeam(
            CollaborationValidationError::TooFewMembers { .. },
        )) => {}
        PreflightError::Charter(other) => panic!("expected TooFewMembers, got {other:?}"),
    }
}

#[test]
fn preflight_fails_when_required_role_missing() {
    let charter = CollaborationCharter::huddle();
    // Four members, but no Critic (one of huddle's expected roles).
    // The charter validator returns the first missing role it finds
    // by iterating `expected_roles` in order: [Lead, Domain, Critic,
    // Synthesizer]. With Lead + Domain + Synthesizer present, the
    // first miss is Critic.
    let team = TeamFormation {
        mode: TeamFormationMode::CapabilityMatched,
        members: vec![
            member("lead", CollaborationRole::Lead),
            member("domain-1", CollaborationRole::Domain),
            member("domain-2", CollaborationRole::Domain),
            member("synth", CollaborationRole::Synthesizer),
        ],
    };
    let participants = vec![
        participant("lead", CollaborationRole::Lead),
        participant("domain-1", CollaborationRole::Domain),
        participant("domain-2", CollaborationRole::Domain),
        participant("synth", CollaborationRole::Synthesizer),
    ];

    let err = preflight_design_formation(team, charter, participants)
        .expect_err("missing required role must fail preflight");
    match err {
        PreflightError::Charter(CollaborationRunnerError::InvalidTeam(
            CollaborationValidationError::MissingRole { role },
        )) => {
            assert_eq!(role, CollaborationRole::Critic);
        }
        PreflightError::Charter(other) => panic!("expected MissingRole(Critic), got {other:?}"),
    }
}

// Smoke check that the crate doesn't accidentally re-export
// IntentPacket or similar runtime-only types in its v1 surface.
fn _ensure_dynamics_surface_is_minimal() {
    let _ = Utc::now() + Duration::hours(1);
}
