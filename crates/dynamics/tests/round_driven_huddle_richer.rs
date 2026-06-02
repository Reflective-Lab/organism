//! Round-driven design huddle, richer composition slice.
//!
//! Composes the round-driven batch protocol with one **existing**
//! adversarial Suggestor and the platform `RoundSynthesizer`, all
//! inside one Converge Formation. No new traits, no LLMs, no Runtime
//! orchestrator — every participant is either a shipped Suggestor or
//! a tiny in-test fixture that exists only to close test-side glue
//! the platform deliberately does not own.
//!
//! Acceptance bar (verbatim):
//! - uses at least one existing adversarial Suggestor, not a new
//!   test-only critic
//! - the existing critic emits meaningful Constraints or Evaluations
//!   from draft/rationale context
//! - the huddle runs two rounds
//! - round 2 has a batch selected by `latest_completed_batch()`
//! - compile handoff only compiles that selected batch
//! - context/trace makes each participant's contribution visible
//!
//! `AssumptionBreakerAgent` is per-fact idempotent: its `accepts`
//! wakes whenever there is a strategy fact it has not yet judged
//! (one of `assumption-pass-<id>` / `…-warn-…` / `…-block-…` missing
//! from `Evaluations`/`Constraints`). The test asserts the breaker
//! covers drafts from both rounds — proof that adversarial scrutiny
//! is no longer single-shot.

use async_trait::async_trait;
use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
    ProfileSnapshot, StaticFormationTemplate, SuggestorCapability, SuggestorRole,
};
use converge_kernel::{AgentEffect, Context, ContextFact, ContextKey};
use converge_pack::{Provenance, ProvenanceSource, Suggestor, TextPayload};
use converge_provider::{CostClass, LatencyClass};
use organism_adversarial::AssumptionBreakerAgent;
use organism_catalog::{
    CatalogSuggestorDescriptor, DiscoveryCatalog, DiscoveryMetadata, LoopContribution,
    ProviderDescriptorCatalog, SuggestorDescriptor,
};
use organism_dynamics::{
    BeautyContestSuggestor, CatalogProposerSuggestor, DraftValidatorCriticSuggestor, compile_draft,
    completed_batches, critic_pass_complete_marker, extract_drafts, extract_drafts_for_batch,
    latest_completed_batch, scorer_batch_complete_marker,
};
use organism_runtime::huddle::{
    RoundConventions, RoundStarter, RoundSynthesizer, SynthesisProducer,
};
use organism_runtime::{Formation, FormationCompileRequest, FormationCompiler};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Catalog + templates (same fixture shape as round_driven_huddle.rs).
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
        Uuid::from_u128(0xD101),
        Uuid::from_u128(0xD102),
        FormationTemplateQuery::new().with_keyword("work-template"),
    )
}

// ---------------------------------------------------------------------------
// Round-driven huddle conventions.
//
// `note_key` (Hypotheses) and `synthesis_key` (Strategies) are
// deliberately distinct: `RoundSynthesizer::next_round_needing_synthesis`
// counts notes by id-suffix `:N`, so synthesis facts must not land in
// the same key as notes or they would self-count. Synthesis facts in
// Strategies coexist with draft facts because the strict draft
// extractor skips facts with the wrong `kind`.
// ---------------------------------------------------------------------------

const ROUND_SIGNAL_PREFIX: &str = "design-round-";
const CONTINUE_PREFIX: &str = "design-round:continue:";
const NOTE_PREFIX: &str = "note:huddle:";
const SYNTHESIS_PREFIX: &str = "design-synthesis:";

fn design_huddle_conventions() -> RoundConventions {
    RoundConventions {
        round_signal_key: ContextKey::Signals,
        round_signal_prefix: ROUND_SIGNAL_PREFIX,
        continue_key: ContextKey::Constraints,
        continue_prefix: CONTINUE_PREFIX,
        note_key: ContextKey::Hypotheses,
        synthesis_key: ContextKey::Strategies,
        synthesis_prefix: SYNTHESIS_PREFIX,
    }
}

// ---------------------------------------------------------------------------
// Test-only fixtures: glue the platform deliberately does not own.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct TestProvenance;
impl ProvenanceSource for TestProvenance {
    fn as_str(&self) -> &'static str {
        "round-driven-huddle-richer-test"
    }
}

fn round_number_from_design_batch_id(batch_id: &str) -> Option<u8> {
    batch_id
        .strip_prefix(ROUND_SIGNAL_PREFIX)
        .and_then(|n| n.parse::<u8>().ok())
}

/// Closes the round loop: when the scorer signals completion for
/// `design-round-N`, emit `design-round:continue:N` so `RoundStarter`
/// advances to round N+1.
struct RoundAdvancer;

impl RoundAdvancer {
    fn pending(ctx: &dyn Context) -> Vec<u8> {
        let mut rounds: Vec<u8> = completed_batches(ctx)
            .into_iter()
            .filter_map(|b| round_number_from_design_batch_id(&b))
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
    fn provenance(&self) -> Provenance {
        TestProvenance.provenance()
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        !Self::pending(ctx).is_empty()
    }
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut effect = AgentEffect::builder();
        for round in Self::pending(ctx) {
            effect = effect.proposal(TestProvenance.proposed_fact(
                ContextKey::Constraints,
                format!("{CONTINUE_PREFIX}{round}"),
                TextPayload::new(format!("round {round} scoring complete; advance")),
            ));
        }
        effect.build()
    }
}

/// Turns each per-round scorer completion into one note fact that the
/// platform `RoundSynthesizer` can find. Without this, RoundSynthesizer
/// has nothing to synthesize because the platform does not produce
/// notes from drafts on its own — that mapping is a host policy.
struct ShortlistNoteEmitter;

impl ShortlistNoteEmitter {
    fn pending(ctx: &dyn Context) -> Vec<u8> {
        let mut rounds: Vec<u8> = completed_batches(ctx)
            .into_iter()
            .filter_map(|b| round_number_from_design_batch_id(&b))
            .filter(|round| {
                let id = format!("{NOTE_PREFIX}{round}");
                !ctx.get(ContextKey::Hypotheses)
                    .iter()
                    .any(|fact| fact.id().as_str() == id)
            })
            .collect();
        rounds.sort_unstable();
        rounds.dedup();
        rounds
    }
}

#[async_trait]
impl Suggestor for ShortlistNoteEmitter {
    fn name(&self) -> &'static str {
        "test-shortlist-note-emitter"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Diagnostic, ContextKey::Proposals]
    }
    fn provenance(&self) -> Provenance {
        TestProvenance.provenance()
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        !Self::pending(ctx).is_empty()
    }
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut effect = AgentEffect::builder();
        for round in Self::pending(ctx) {
            let batch_id = format!("{ROUND_SIGNAL_PREFIX}{round}");
            let shortlist = extract_drafts_for_batch(ctx, ContextKey::Proposals, &batch_id);
            let descriptors: Vec<String> = shortlist
                .first()
                .map(|d| d.descriptor_ids.iter().map(ToString::to_string).collect())
                .unwrap_or_default();
            effect = effect.proposal(TestProvenance.proposed_fact(
                ContextKey::Hypotheses,
                format!("{NOTE_PREFIX}{round}"),
                TextPayload::new(format!(
                    "huddle note for round {round}: shortlist=[{}]",
                    descriptors.join(",")
                )),
            ));
        }
        effect.build()
    }
}

/// Deterministic synthesis producer for the platform `RoundSynthesizer`.
struct HuddleSynthesisProducer;

#[async_trait]
impl SynthesisProducer for HuddleSynthesisProducer {
    async fn synthesize(
        &self,
        round: u8,
        notes: &[ContextFact],
        _ctx: &dyn Context,
    ) -> Result<String, String> {
        Ok(format!(
            "design huddle synthesis for round {round}: {} note(s)",
            notes.len()
        ))
    }
}

// ---------------------------------------------------------------------------
// Acceptance test
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::too_many_lines, clippy::similar_names)]
async fn richer_huddle_composes_real_adversarial_and_synthesizer_with_round_driven_batches() {
    let catalog = catalog();
    let templates = templates();
    let providers = ProviderDescriptorCatalog::new();
    let request = request();
    let conventions = design_huddle_conventions();

    let round_starter = RoundStarter::new(2).with_conventions(conventions);

    let proposer = CatalogProposerSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
        2,
    )
    .with_round_signals(ROUND_SIGNAL_PREFIX);

    let critic = DraftValidatorCriticSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
    );

    let adversarial = AssumptionBreakerAgent::new();
    let scorer = BeautyContestSuggestor::new_critic_gated(1);
    let synthesizer =
        RoundSynthesizer::new(1, HuddleSynthesisProducer).with_conventions(conventions);

    let huddle = Formation::new("richer-design-huddle")
        .agent_boxed(Box::new(round_starter))
        .agent_boxed(Box::new(proposer))
        .agent_boxed(Box::new(critic))
        .agent_boxed(Box::new(adversarial))
        .agent_boxed(Box::new(scorer))
        .agent_boxed(Box::new(ShortlistNoteEmitter))
        .agent_boxed(Box::new(synthesizer))
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

    // --- RoundStarter contribution: two round-start signals --------------
    let mut round_signals: Vec<&str> = context
        .get(ContextKey::Signals)
        .iter()
        .map(|f| f.id().as_str())
        .filter(|id| id.starts_with(ROUND_SIGNAL_PREFIX))
        .collect();
    round_signals.sort_unstable();
    assert_eq!(
        round_signals,
        vec!["design-round-1", "design-round-2"],
        "RoundStarter must own both round/batch ids"
    );

    // --- Proposer contribution: drafts in both batches --------------------
    let drafts_a = extract_drafts_for_batch(context, ContextKey::Strategies, "design-round-1");
    let drafts_b = extract_drafts_for_batch(context, ContextKey::Strategies, "design-round-2");
    assert_eq!(drafts_a.len(), 2, "round 1 must have 2 drafts");
    assert_eq!(drafts_b.len(), 2, "round 2 must have 2 drafts");
    assert!(
        drafts_a.iter().any(|d| d.draft_id == "candidate-0")
            && drafts_b.iter().any(|d| d.draft_id == "candidate-0"),
        "(draft_batch_id, draft_id) is the join key; both batches reuse candidate-0"
    );

    // --- DraftValidatorCritic contribution: per-batch sentinels + Pass ----
    let diagnostic_ids: Vec<&str> = context
        .get(ContextKey::Diagnostic)
        .iter()
        .map(|fact| fact.id().as_str())
        .collect();
    assert!(diagnostic_ids.contains(&critic_pass_complete_marker("design-round-1").as_str()));
    assert!(diagnostic_ids.contains(&critic_pass_complete_marker("design-round-2").as_str()));

    // --- AssumptionBreaker contribution: per-draft judgment per round -----
    // After the per-fact idempotency refactor, the breaker fires on
    // every unjudged strategy fact, in any round. The agent is
    // generic — it scrutinizes anything in `Strategies`, including
    // the synthesis facts that `RoundSynthesizer` emits there. We
    // filter to draft-shaped fact ids to assert per-batch coverage.
    let breaker_outputs: Vec<&str> = context
        .get(ContextKey::Evaluations)
        .iter()
        .chain(context.get(ContextKey::Constraints).iter())
        .map(|fact| fact.id().as_str())
        .filter(|id| id.starts_with("assumption-"))
        .collect();
    let breaker_outputs_for_drafts: Vec<&&str> = breaker_outputs
        .iter()
        .filter(|id| id.contains("formation-draft-"))
        .collect();
    assert_eq!(
        breaker_outputs_for_drafts.len(),
        4,
        "breaker must judge each draft across both rounds (2 drafts × 2 rounds); got {breaker_outputs_for_drafts:?}"
    );

    // Each batch must be represented in the breaker's outputs. The
    // breaker stamps its emit-id as `assumption-<verdict>-<strategy_fact_id>`,
    // and the proposer's strategy fact id for a draft is
    // `formation-draft-{encoded_batch_id}-{index}`. We can therefore
    // recover the batch by inspecting the strategy fact ids the
    // breaker judged.
    for batch_id in ["design-round-1", "design-round-2"] {
        let batch_draft_fact_ids: Vec<&str> = context
            .get(ContextKey::Strategies)
            .iter()
            .map(|f| f.id().as_str())
            .filter(|id| id.starts_with("formation-draft-"))
            .filter(|id| {
                // Strategy fact id encodes the batch in hex. Cross-check
                // by reading the draft payload to confirm the batch.
                extract_drafts(context, ContextKey::Strategies)
                    .iter()
                    .any(|d| {
                        d.draft_batch_id == batch_id
                            && id.ends_with(&format!(
                                "-{idx}",
                                idx = d.draft_id.trim_start_matches("candidate-")
                            ))
                    })
            })
            .collect();
        assert!(
            !batch_draft_fact_ids.is_empty(),
            "expected at least one strategy fact id for batch {batch_id}"
        );
        let covered = batch_draft_fact_ids.iter().all(|sid| {
            breaker_outputs_for_drafts
                .iter()
                .any(|bid| bid.ends_with(*sid))
        });
        assert!(
            covered,
            "breaker must scrutinize every draft in {batch_id}: strategy_ids={batch_draft_fact_ids:?}, breaker_outputs={breaker_outputs_for_drafts:?}"
        );
    }

    // --- BeautyContest contribution: per-batch sentinels + shortlists -----
    assert!(diagnostic_ids.contains(&scorer_batch_complete_marker("design-round-1").as_str()));
    assert!(diagnostic_ids.contains(&scorer_batch_complete_marker("design-round-2").as_str()));
    let shortlist = extract_drafts(context, ContextKey::Proposals);
    assert_eq!(shortlist.len(), 2, "top_n=1 over 2 batches → 2 shortlisted");
    let shortlist_batches: std::collections::BTreeSet<&str> = shortlist
        .iter()
        .map(|d| d.draft_batch_id.as_str())
        .collect();
    assert_eq!(
        shortlist_batches.into_iter().collect::<Vec<_>>(),
        vec!["design-round-1", "design-round-2"],
    );

    // --- RoundSynthesizer contribution: per-round synthesis facts ---------
    let synthesis_ids: Vec<&str> = context
        .get(ContextKey::Strategies)
        .iter()
        .map(|f| f.id().as_str())
        .filter(|id| id.starts_with(SYNTHESIS_PREFIX))
        .collect();
    let mut synthesis_sorted = synthesis_ids;
    synthesis_sorted.sort_unstable();
    assert_eq!(
        synthesis_sorted,
        vec!["design-synthesis:1", "design-synthesis:2"],
        "RoundSynthesizer must emit one synthesis per round"
    );

    // --- Compile handoff: latest_completed_batch → batch B; compile_draft -
    let latest = latest_completed_batch(context).expect("at least one batch completed");
    assert_eq!(
        latest, "design-round-2",
        "round 2 finishes after round 1, so latest_completed_batch picks batch B"
    );
    let batch_b_shortlist = extract_drafts_for_batch(context, ContextKey::Proposals, &latest);
    assert_eq!(
        batch_b_shortlist.len(),
        1,
        "compile handoff sees exactly batch B's shortlist"
    );

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
