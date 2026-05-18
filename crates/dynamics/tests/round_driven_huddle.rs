//! Round-driven design huddle: RoundStarter is the source of truth
//! for `draft_batch_id`; the proposer consumes the current open round;
//! the critic, scorer, and compile handoff all route per batch.
//!
//! Acceptance bar (verbatim from the design review):
//! - round 1 produces batch A and a shortlist
//! - round 2 produces batch B
//! - batch B reuses `draft_id = candidate-0`
//! - both batches have critic AND scorer completion sentinels
//! - `extract_drafts(Proposals)` contains multiple batches
//! - compile handoff deliberately picks batch B only
//! - no accidental "first proposal wins"
//!
//! No new traits, no LLM dependency, no Runtime orchestrator. The
//! Formation itself owns batch lifecycle through the standard
//! Suggestor surface; a tiny in-test `RoundAdvancer` fixture closes the
//! continue-marker loop because the platform-level scorer should not
//! be coupled to round conventions.

use async_trait::async_trait;
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
    BeautyContestSuggestor, CatalogProposerSuggestor, DraftValidatorCriticSuggestor, DraftVerdict,
    SCORER_BATCH_COMPLETE_PREFIX, compile_draft, completed_batches, critic_pass_complete_marker,
    extract_draft_validations, extract_drafts, extract_drafts_for_batch, latest_completed_batch,
    scorer_batch_complete_marker,
};
use organism_runtime::huddle::{RoundConventions, RoundStarter};
use organism_runtime::{Formation, FormationCompileRequest, FormationCompiler};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Catalog + templates: enough to compile candidate rosters of size 2.
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

fn catalog() -> DiscoveryCatalog {
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

fn templates() -> FormationCatalog {
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

fn request() -> FormationCompileRequest {
    FormationCompileRequest::new(
        Uuid::from_u128(0xD001),
        Uuid::from_u128(0xD002),
        FormationTemplateQuery::new().with_keyword("work-template"),
    )
}

// ---------------------------------------------------------------------------
// Round-driven huddle conventions
// ---------------------------------------------------------------------------

const ROUND_SIGNAL_PREFIX: &str = "design-round-";
const CONTINUE_PREFIX: &str = "design-round:continue:";

fn design_huddle_conventions() -> RoundConventions {
    RoundConventions {
        round_signal_key: ContextKey::Signals,
        round_signal_prefix: ROUND_SIGNAL_PREFIX,
        continue_key: ContextKey::Constraints,
        continue_prefix: CONTINUE_PREFIX,
        // Notes / synthesis are not exercised by this slice — leave
        // them on their defaults so an accidental ContextKey::Strategies
        // synthesis fact does not collide with our draft facts.
        note_key: ContextKey::Hypotheses,
        synthesis_key: ContextKey::Hypotheses,
        synthesis_prefix: "design-synthesis:",
    }
}

// ---------------------------------------------------------------------------
// RoundAdvancer: test-only fixture that closes the round loop.
//
// Watches the scorer's batch-completion sentinels under
// ContextKey::Diagnostic and, for each `design-round-N` whose scoring
// is finished, emits the matching `design-round:continue:N` marker
// under ContextKey::Constraints so RoundStarter advances to round
// N+1. Production hosts can compose this any way they like; the test
// uses the smallest possible glue that proves the lifecycle.
// ---------------------------------------------------------------------------

struct RoundAdvancer;

#[derive(Debug, Clone, Copy)]
struct TestProvenance;
impl ProvenanceSource for TestProvenance {
    fn as_str(&self) -> &'static str {
        "round-driven-huddle-test"
    }
}

impl RoundAdvancer {
    fn round_number_for_batch(batch_id: &str) -> Option<u8> {
        batch_id
            .strip_prefix(ROUND_SIGNAL_PREFIX)
            .and_then(|n| n.parse::<u8>().ok())
    }

    fn pending_advance(ctx: &dyn Context) -> Vec<u8> {
        let mut rounds: Vec<u8> = completed_batches(ctx)
            .into_iter()
            .filter_map(|b| Self::round_number_for_batch(&b))
            .filter(|round| {
                let marker = format!("{CONTINUE_PREFIX}{round}");
                !ctx.get(ContextKey::Constraints)
                    .iter()
                    .any(|fact| fact.id().as_str() == marker)
            })
            .collect();
        rounds.sort_unstable();
        rounds.dedup();
        rounds
    }
}

#[async_trait]
impl Suggestor for RoundAdvancer {
    fn name(&self) -> &'static str {
        "test-round-advancer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Diagnostic]
    }

    fn provenance(&self) -> &'static str {
        TestProvenance.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        !Self::pending_advance(ctx).is_empty()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut effect = AgentEffect::builder();
        for round in Self::pending_advance(ctx) {
            effect = effect.proposal(TestProvenance.proposed_fact(
                ContextKey::Constraints,
                format!("{CONTINUE_PREFIX}{round}"),
                TextPayload::new(format!("round {round} scoring complete; advance")),
            ));
        }
        effect.build()
    }
}

// ---------------------------------------------------------------------------
// Acceptance test
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::too_many_lines, clippy::similar_names)]
async fn round_starter_owns_batch_lifecycle_and_compile_handoff_picks_a_specific_batch() {
    let catalog = catalog();
    let templates = templates();
    let providers = ProviderDescriptorCatalog::new();
    let request = request();
    let conventions = design_huddle_conventions();

    // Cap at 2 rounds — exactly what the acceptance bar requires.
    let round_starter = RoundStarter::new(2).with_conventions(conventions);

    let proposer = CatalogProposerSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
        // k=2 so each batch has two candidates and a scorer top_n of 1
        // proves "shortlist is a strict subset," not "shortlist equals
        // every draft."
        2,
    )
    .with_round_signals(ROUND_SIGNAL_PREFIX);

    let critic = DraftValidatorCriticSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
    );

    let scorer = BeautyContestSuggestor::new_critic_gated(1);

    let huddle = Formation::new("round-driven-design-huddle")
        .agent_boxed(Box::new(round_starter))
        .agent_boxed(Box::new(proposer))
        .agent_boxed(Box::new(critic))
        .agent_boxed(Box::new(scorer))
        .agent_boxed(Box::new(RoundAdvancer))
        .seed(
            ContextKey::Seeds,
            "design-seed",
            "design the work formation",
            "test",
        );

    let result = huddle.run().await.expect("design huddle should converge");
    assert!(
        result.converge_result.converged,
        "huddle must converge; stop_reason = {:?}",
        result.converge_result.stop_reason
    );
    let context = &result.converge_result.context;

    // --- Two rounds fired with the right ids -----------------------------
    let mut round_signal_ids: Vec<&str> = context
        .get(ContextKey::Signals)
        .iter()
        .map(|fact| fact.id().as_str())
        .filter(|id| id.starts_with(ROUND_SIGNAL_PREFIX))
        .collect();
    round_signal_ids.sort_unstable();
    assert_eq!(
        round_signal_ids,
        vec!["design-round-1", "design-round-2"],
        "RoundStarter must own both batch ids"
    );

    // --- Drafts in Strategies: two per batch, same draft_id reused -------
    let drafts = extract_drafts(context, ContextKey::Strategies);
    let drafts_round_1 =
        extract_drafts_for_batch(context, ContextKey::Strategies, "design-round-1");
    let drafts_round_2 =
        extract_drafts_for_batch(context, ContextKey::Strategies, "design-round-2");
    assert_eq!(
        drafts.len(),
        4,
        "expected 2 drafts per round; got {drafts:?}"
    );
    assert_eq!(drafts_round_1.len(), 2);
    assert_eq!(drafts_round_2.len(), 2);

    // draft_id "candidate-0" appears in BOTH batches — the join key is
    // (draft_batch_id, draft_id), not draft_id alone.
    let round_1_ids: Vec<&str> = drafts_round_1.iter().map(|d| d.draft_id.as_str()).collect();
    let round_2_ids: Vec<&str> = drafts_round_2.iter().map(|d| d.draft_id.as_str()).collect();
    assert!(
        round_1_ids.contains(&"candidate-0") && round_2_ids.contains(&"candidate-0"),
        "batch B must reuse draft_id=candidate-0; round_1={round_1_ids:?} round_2={round_2_ids:?}"
    );

    // --- Critic verdicts: per-batch sentinels both present ---------------
    let critic_marker_round_1 = critic_pass_complete_marker("design-round-1");
    let critic_marker_round_2 = critic_pass_complete_marker("design-round-2");
    let diagnostic_ids: Vec<&str> = context
        .get(ContextKey::Diagnostic)
        .iter()
        .map(|fact| fact.id().as_str())
        .collect();
    assert!(
        diagnostic_ids.contains(&critic_marker_round_1.as_str()),
        "critic sentinel missing for design-round-1; diagnostic_ids={diagnostic_ids:?}"
    );
    assert!(
        diagnostic_ids.contains(&critic_marker_round_2.as_str()),
        "critic sentinel missing for design-round-2; diagnostic_ids={diagnostic_ids:?}"
    );

    // Verdicts are Pass for both batches (catalog satisfies template).
    let passes = extract_draft_validations(context, ContextKey::Evaluations);
    assert!(
        passes
            .iter()
            .any(|v| v.verdict == DraftVerdict::Pass && v.draft_batch_id == "design-round-1"),
        "expected at least one Pass verdict in design-round-1; got {passes:?}"
    );
    assert!(
        passes
            .iter()
            .any(|v| v.verdict == DraftVerdict::Pass && v.draft_batch_id == "design-round-2"),
        "expected at least one Pass verdict in design-round-2; got {passes:?}"
    );

    // --- Scorer sentinels: per-batch markers both present ----------------
    let scorer_marker_round_1 = scorer_batch_complete_marker("design-round-1");
    let scorer_marker_round_2 = scorer_batch_complete_marker("design-round-2");
    assert!(
        diagnostic_ids.contains(&scorer_marker_round_1.as_str()),
        "scorer sentinel missing for design-round-1"
    );
    assert!(
        diagnostic_ids.contains(&scorer_marker_round_2.as_str()),
        "scorer sentinel missing for design-round-2"
    );
    // Sanity-check the prefix-driven recovery path used by
    // completed_batches: every scorer marker we emitted starts with
    // the public prefix, so callers can scan without knowing the
    // exact id form.
    assert!(
        diagnostic_ids
            .iter()
            .any(|id| id.starts_with(SCORER_BATCH_COMPLETE_PREFIX)),
        "scorer sentinel prefix must be discoverable by callers"
    );

    // --- Proposals contain drafts from both batches ----------------------
    let shortlist = extract_drafts(context, ContextKey::Proposals);
    assert_eq!(shortlist.len(), 2, "top_n=1 per batch over 2 batches");
    let shortlist_batches: std::collections::BTreeSet<&str> = shortlist
        .iter()
        .map(|d| d.draft_batch_id.as_str())
        .collect();
    assert_eq!(
        shortlist_batches.into_iter().collect::<Vec<_>>(),
        vec!["design-round-1", "design-round-2"],
        "Proposals must surface a shortlist for both rounds (no 'first proposal wins')"
    );

    // --- Compile handoff: latest_completed_batch resolves to batch B -----
    let completed = completed_batches(context);
    assert_eq!(
        completed.iter().collect::<std::collections::HashSet<_>>(),
        ["design-round-1".to_string(), "design-round-2".to_string()]
            .iter()
            .collect::<std::collections::HashSet<_>>(),
        "both batches must be visible in completed_batches; got {completed:?}"
    );
    let latest = latest_completed_batch(context).expect("at least one batch completed");
    assert_eq!(
        latest, "design-round-2",
        "round 2 finishes after round 1 so latest_completed_batch must surface batch B"
    );

    // --- Explicit batch-B-only compile handoff ---------------------------
    let batch_b_shortlist = extract_drafts_for_batch(context, ContextKey::Proposals, &latest);
    assert_eq!(
        batch_b_shortlist.len(),
        1,
        "compile handoff must see exactly batch B's shortlist; got {batch_b_shortlist:?}"
    );
    assert_eq!(batch_b_shortlist[0].draft_batch_id, "design-round-2");

    // Confirm batch A is also accessible by id and that the routing
    // helper does not collapse the two batches (the payload draft_id
    // is deliberately reused; the per-batch slice is what
    // distinguishes them).
    let batch_a_shortlist =
        extract_drafts_for_batch(context, ContextKey::Proposals, "design-round-1");
    assert_eq!(batch_a_shortlist.len(), 1);
    assert_eq!(batch_a_shortlist[0].draft_batch_id, "design-round-1");
    assert_eq!(
        batch_a_shortlist[0].draft_id, batch_b_shortlist[0].draft_id,
        "deterministic proposer makes both batches reuse the same draft_id; \
         the routing key is (draft_batch_id, draft_id)"
    );

    // The chosen batch's draft must compile via the exact-roster
    // validator — proving the compile handoff is real, not just
    // bookkeeping.
    let compiler = FormationCompiler::new();
    let plan = compile_draft(
        &compiler,
        &request,
        &templates,
        &catalog,
        &providers,
        &batch_b_shortlist[0],
    )
    .expect("batch B's shortlisted draft must compile");
    assert_eq!(plan.template_id, "work-template");
    assert!(!plan.roster.is_empty());
}
