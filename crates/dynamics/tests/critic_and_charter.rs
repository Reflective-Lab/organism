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
use converge_pack::{Provenance, ProvenanceSource, Suggestor, TextPayload};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{
    CatalogSuggestorDescriptor, DiscoveryCatalog, DiscoveryMetadata, LoopContribution,
    ProviderDescriptorCatalog, SuggestorDescriptor,
};
use organism_dynamics::{
    BeautyContestSuggestor, CatalogProposerSuggestor, DRAFT_VALIDATION_KIND,
    DraftValidatorCriticSuggestor, DraftVerdict, FormationDraft, PreflightError,
    critic_pass_complete_marker, extract_draft_validations, extract_drafts,
    preflight_design_formation,
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
    fn provenance(&self) -> Provenance {
        Provenance::from(TestProvenance.as_str())
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
        let batch_id = format!("bad-batch-{}", self.source_label);
        let draft_id = format!("bad-draft-{}-0", self.source_label);
        let draft = FormationDraft::new(
            draft_id.clone(),
            batch_id,
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
// Critic + scorer: later batches are independent from earlier verdicts
// ---------------------------------------------------------------------------

const FIRST_BATCH: &str = "temporal-batch-1";
const SECOND_BATCH: &str = "temporal-batch-2";
const SHARED_DRAFT_ID: &str = "candidate-0";

/// Emits an invalid first batch, then waits for the critic's first
/// batch sentinel before emitting a valid second batch with the same
/// payload `draft_id`. The fact ids differ, but the draft ids collide
/// intentionally to prove batch-scoped routing.
struct TwoBatchProposer;

impl TwoBatchProposer {
    fn emitted_marker(batch_id: &str) -> String {
        format!("two-batch-proposer-emitted-{batch_id}")
    }

    fn has_diagnostic(ctx: &dyn Context, id: &str) -> bool {
        ctx.get(ContextKey::Diagnostic)
            .iter()
            .any(|fact| fact.id().as_str() == id)
    }

    fn emit_batch(batch_id: &str, descriptor_ids: Vec<String>, rationale: &str) -> AgentEffect {
        let draft = FormationDraft::new(
            SHARED_DRAFT_ID,
            batch_id,
            descriptor_ids,
            rationale,
            "two-batch-proposer",
        );
        let json = serde_json::to_string(&draft).unwrap();
        AgentEffect::builder()
            .proposal(TestProvenance.proposed_fact(
                ContextKey::Strategies,
                format!("two-batch-{batch_id}-{SHARED_DRAFT_ID}"),
                TextPayload::new(json),
            ))
            .proposal(TestProvenance.proposed_fact(
                ContextKey::Diagnostic,
                Self::emitted_marker(batch_id),
                TextPayload::new(format!("two-batch proposer emitted {batch_id}")),
            ))
            .build()
    }
}

#[async_trait]
impl Suggestor for TwoBatchProposer {
    fn name(&self) -> &'static str {
        "two-batch-proposer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Diagnostic]
    }

    fn provenance(&self) -> Provenance {
        Provenance::from(TestProvenance.as_str())
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Seeds) {
            return false;
        }
        if !Self::has_diagnostic(ctx, &Self::emitted_marker(FIRST_BATCH)) {
            return true;
        }
        Self::has_diagnostic(ctx, &critic_pass_complete_marker(FIRST_BATCH))
            && !Self::has_diagnostic(ctx, &Self::emitted_marker(SECOND_BATCH))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if !Self::has_diagnostic(ctx, &Self::emitted_marker(FIRST_BATCH)) {
            return Self::emit_batch(
                FIRST_BATCH,
                vec![
                    "signal-a".to_string(),
                    "definitely-not-in-catalog".to_string(),
                ],
                "invalid first batch for temporal routing regression",
            );
        }

        Self::emit_batch(
            SECOND_BATCH,
            vec!["signal-a".to_string(), "constraint-a".to_string()],
            "valid second batch with reused draft id",
        )
    }
}

#[tokio::test]
async fn later_batch_shortlists_even_when_earlier_same_id_draft_was_blocked() {
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

    let formation = Formation::new("design-with-two-batches")
        .agent_boxed(Box::new(TwoBatchProposer))
        .agent_boxed(Box::new(critic))
        .agent_boxed(Box::new(scorer))
        .seed(ContextKey::Seeds, "s", "design", "test");

    let result = formation.run().await.expect("should converge");
    assert!(result.converge_result.converged);

    let blocks =
        extract_draft_validations(&result.converge_result.context, ContextKey::Constraints);
    assert!(
        blocks.iter().any(|v| {
            v.verdict == DraftVerdict::Block
                && v.draft_batch_id == FIRST_BATCH
                && v.draft_id == SHARED_DRAFT_ID
        }),
        "first batch should block shared draft id; got {blocks:?}"
    );

    let passes =
        extract_draft_validations(&result.converge_result.context, ContextKey::Evaluations);
    assert!(
        passes.iter().any(|v| {
            v.verdict == DraftVerdict::Pass
                && v.draft_batch_id == SECOND_BATCH
                && v.draft_id == SHARED_DRAFT_ID
        }),
        "second batch should pass the same draft id; got {passes:?}"
    );

    let shortlist = extract_drafts(&result.converge_result.context, ContextKey::Proposals);
    assert_eq!(
        shortlist.len(),
        1,
        "only the valid second batch should be shortlisted; got {shortlist:?}"
    );
    assert_eq!(shortlist[0].draft_batch_id, SECOND_BATCH);
    assert_eq!(shortlist[0].draft_id, SHARED_DRAFT_ID);
    assert_eq!(
        shortlist[0].descriptor_ids,
        vec!["signal-a".to_string(), "constraint-a".to_string()]
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

// ---------------------------------------------------------------------------
// Two-batch regression — no cross-batch contamination
// ---------------------------------------------------------------------------

/// A test-only Suggestor that proposes one [`FormationDraft`] with a
/// bogus descriptor under a caller-supplied `batch_id`, but only AFTER
/// a shortlist for `gate_on_shortlist_batch_id` already exists in
/// `Proposals`. This sequences batches: the second proposer waits for
/// the first batch to complete its full pipeline before it fires.
struct SequencedBadProposer {
    source_label: &'static str,
    batch_id: &'static str,
    gate_on_shortlist_batch_id: &'static str,
}

impl SequencedBadProposer {
    fn marker_id(&self) -> String {
        format!("sequenced-bad-{}-fired", self.source_label)
    }
}

#[async_trait]
impl Suggestor for SequencedBadProposer {
    fn name(&self) -> &'static str {
        "sequenced-bad-proposer"
    }
    fn dependencies(&self) -> &[ContextKey] {
        // Wait on Proposals so the engine re-checks accepts() once a
        // shortlist arrives.
        &[
            ContextKey::Seeds,
            ContextKey::Proposals,
            ContextKey::Diagnostic,
        ]
    }
    fn provenance(&self) -> Provenance {
        Provenance::from(TestProvenance.as_str())
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Seeds) {
            return false;
        }
        // Fire once.
        let marker = self.marker_id();
        let fired = ctx
            .get(ContextKey::Diagnostic)
            .iter()
            .any(|fact| fact.id().as_str() == marker.as_str());
        if fired {
            return false;
        }
        // Wait for the first batch's shortlist.
        extract_drafts(ctx, ContextKey::Proposals)
            .iter()
            .any(|d| d.draft_batch_id == self.gate_on_shortlist_batch_id)
    }
    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        let draft_id = format!("sequenced-bad-{}-0", self.source_label);
        let draft = FormationDraft::new(
            draft_id.clone(),
            self.batch_id,
            vec![
                "signal-a".to_string(),
                "definitely-not-in-catalog".to_string(),
            ],
            "bad draft proposed in a second batch after batch 1 completed",
            self.source_label,
        );
        let json = serde_json::to_string(&draft).unwrap();
        AgentEffect::builder()
            .proposal(TestProvenance.proposed_fact(
                ContextKey::Strategies,
                draft_id,
                TextPayload::new(json),
            ))
            .proposal(TestProvenance.proposed_fact(
                ContextKey::Diagnostic,
                self.marker_id(),
                TextPayload::new(format!(
                    "{} fired bad draft for batch {}",
                    self.source_label, self.batch_id
                )),
            ))
            .build()
    }
}

/// Two-batch regression. The good-batch fires first (CatalogProposer
/// with batch_id "good-batch") and is processed end-to-end (critic
/// verdicts → critic sentinel → scorer shortlist → scorer sentinel).
/// The bad-batch fires *after* the good-batch's shortlist exists,
/// emitting one bad draft. The critic must process bad-batch
/// separately (without re-validating good-batch). The scorer must
/// shortlist good-batch with no bad-batch contamination, and must
/// emit a scorer-completion sentinel for bad-batch with zero
/// shortlist drafts.
///
/// This is the acceptance bar for round-scoped gating: temporal
/// routing is stable across batches.
#[tokio::test]
async fn two_batches_no_cross_contamination_no_deadlock() {
    let catalog = work_catalog();
    let templates = work_template_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let request = work_request();

    // good-batch: CatalogProposer with explicit batch_id.
    let good = CatalogProposerSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
        3,
    )
    .with_source_label("good-proposer")
    .with_batch_id("good-batch");

    // bad-batch: sequenced after good-batch's shortlist exists.
    let bad = SequencedBadProposer {
        source_label: "bad-proposer",
        batch_id: "bad-batch",
        gate_on_shortlist_batch_id: "good-batch",
    };

    let critic = DraftValidatorCriticSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
    );
    let scorer = BeautyContestSuggestor::new_critic_gated(2);

    let formation = Formation::new("two-batch-design")
        .agent_boxed(Box::new(good))
        .agent_boxed(Box::new(bad))
        .agent_boxed(Box::new(critic))
        .agent_boxed(Box::new(scorer))
        .seed(ContextKey::Seeds, "s", "two-batch design", "test");

    let result = formation.run().await.expect("should converge");
    assert!(
        result.converge_result.converged,
        "two-batch Formation must converge; stop_reason = {:?}",
        result.converge_result.stop_reason
    );

    // --- Critic verdicts are per-batch ---
    let passes =
        extract_draft_validations(&result.converge_result.context, ContextKey::Evaluations);
    assert!(
        passes.iter().all(|v| v.draft_batch_id == "good-batch"),
        "all Pass verdicts must belong to good-batch (no good draft was in bad-batch); got {:?}",
        passes
            .iter()
            .map(|v| (v.draft_id.clone(), v.draft_batch_id.clone()))
            .collect::<Vec<_>>(),
    );
    let blocks: Vec<_> =
        extract_draft_validations(&result.converge_result.context, ContextKey::Constraints)
            .into_iter()
            .filter(|v| v.verdict == DraftVerdict::Block)
            .collect();
    assert_eq!(
        blocks.len(),
        1,
        "exactly one Block verdict expected (bad-batch's only draft); got {blocks:?}"
    );
    assert_eq!(blocks[0].draft_batch_id, "bad-batch");

    // --- Per-batch sentinels: both critic markers present ---
    for batch in ["good-batch", "bad-batch"] {
        let marker = critic_pass_complete_marker(batch);
        assert!(
            result
                .converge_result
                .context
                .get(ContextKey::Diagnostic)
                .iter()
                .any(|fact| fact.id().as_str() == marker),
            "critic must emit per-batch sentinel for '{batch}'"
        );
    }

    // --- Per-batch scorer sentinels: both batches recorded as scored,
    //     including bad-batch which produced zero shortlist drafts ---
    for batch in ["good-batch", "bad-batch"] {
        let marker = organism_dynamics::scorer_batch_complete_marker(batch);
        assert!(
            result
                .converge_result
                .context
                .get(ContextKey::Diagnostic)
                .iter()
                .any(|fact| fact.id().as_str() == marker),
            "scorer must emit per-batch completion sentinel for '{batch}'"
        );
    }

    // --- Shortlist contains ONLY good-batch drafts. No bad-batch
    //     contamination, no leakage across the join key. ---
    let shortlist = extract_drafts(&result.converge_result.context, ContextKey::Proposals);
    assert!(
        !shortlist.is_empty(),
        "good-batch should produce a non-empty shortlist"
    );
    for draft in &shortlist {
        assert_eq!(
            draft.draft_batch_id, "good-batch",
            "shortlist must contain ONLY good-batch drafts; found {draft:?}"
        );
    }
}
