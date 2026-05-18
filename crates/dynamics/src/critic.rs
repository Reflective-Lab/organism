//! [`DraftValidatorCriticSuggestor`] — pre-validates [`FormationDraft`]
//! facts against the catalog and proposes a typed [`DraftValidation`]
//! verdict per draft.
//!
//! Runs once per draft set. For each draft, attempts the exact-roster
//! validator
//! ([`organism_runtime::FormationCompiler::compile_draft_from_catalog`])
//! and proposes a [`DraftValidation`] fact under
//! `ContextKey::Evaluations` (`Pass`) or `ContextKey::Constraints`
//! (`Block`). Converge admits and may promote.
//!
//! The critic is *additive* — it doesn't filter the draft pool
//! directly. It surfaces verdicts in the design Formation's promoted
//! context so the audit trail records why each draft was admitted or
//! rejected. Downstream consumers (e.g. [`crate::BeautyContestSuggestor`])
//! gate on these verdicts when picking a shortlist.

use std::collections::BTreeMap;

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};
use organism_catalog::{DiscoveryCatalog, ProviderDescriptorCatalog};
use organism_runtime::{FormationCompileRequest, FormationCompiler};

use crate::batch::encode_batch_id;
use crate::compile::compile_draft;
use crate::extract::extract_drafts;
use crate::payload::{DraftValidation, DraftVerdict};
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-draft-validator-critic";

/// Sentinel fact id prefix the critic emits under
/// `ContextKey::Diagnostic` once it has produced verdicts for a draft
/// batch. Downstream Suggestors that need to wait for verdicts (e.g.
/// [`crate::BeautyContestSuggestor`] in critic-gated mode) check the
/// batch-scoped marker from [`critic_pass_complete_marker`] to know
/// "the critic has spoken for this batch; verdicts are now safe to
/// read." This is the join sentinel that turns concurrent per-round
/// firing into ordered phases without blocking later batches.
pub const CRITIC_PASS_COMPLETE_MARKER: &str = "organism-dynamics-critic-pass-complete";

/// Returns the diagnostic fact id for a completed critic pass over
/// `draft_batch_id`.
#[must_use]
pub fn critic_pass_complete_marker(draft_batch_id: &str) -> String {
    format!(
        "{CRITIC_PASS_COMPLETE_MARKER}-{}",
        encode_batch_id(draft_batch_id)
    )
}

/// Catalog-aware critic that emits one [`DraftValidation`] verdict per
/// [`FormationDraft`] fact found under `ContextKey::Strategies`.
///
/// Batch-scoped: fires when at least one draft batch is present that
/// does not yet have a critic completion marker.
pub struct DraftValidatorCriticSuggestor {
    catalog: DiscoveryCatalog,
    formation_templates: converge_kernel::formation::FormationCatalog,
    providers: ProviderDescriptorCatalog,
    request: FormationCompileRequest,
}

impl DraftValidatorCriticSuggestor {
    #[must_use]
    pub fn new(
        catalog: DiscoveryCatalog,
        formation_templates: converge_kernel::formation::FormationCatalog,
        providers: ProviderDescriptorCatalog,
        request: FormationCompileRequest,
    ) -> Self {
        Self {
            catalog,
            formation_templates,
            providers,
            request,
        }
    }
}

impl std::fmt::Debug for DraftValidatorCriticSuggestor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DraftValidatorCriticSuggestor")
            .field("catalog_entries", &self.catalog.len())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Suggestor for DraftValidatorCriticSuggestor {
    fn name(&self) -> &'static str {
        SUGGESTOR_NAME
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_DYNAMICS_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        extract_drafts(ctx, ContextKey::Strategies)
            .iter()
            .any(|draft| !critic_batch_complete(ctx, &draft.draft_batch_id))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut drafts_by_batch: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for draft in extract_drafts(ctx, ContextKey::Strategies) {
            if !critic_batch_complete(ctx, &draft.draft_batch_id) {
                drafts_by_batch
                    .entry(draft.draft_batch_id.clone())
                    .or_default()
                    .push(draft);
            }
        }

        let compiler = FormationCompiler::new();
        let mut effect = AgentEffect::builder();

        // Route verdicts off the draft's own stable id. No
        // reconstructed indices, no source-label routing — the
        // proposer assigned the id and batch and we copy both verbatim.
        for (draft_batch_id, drafts) in &drafts_by_batch {
            let encoded_batch = encode_batch_id(draft_batch_id);
            for draft in drafts {
                let (target_key, verdict, reason) = match compile_draft(
                    &compiler,
                    &self.request,
                    &self.formation_templates,
                    &self.catalog,
                    &self.providers,
                    draft,
                ) {
                    Ok(plan) => (
                        ContextKey::Evaluations,
                        DraftVerdict::Pass,
                        format!(
                            "Validated against template '{}': {} descriptor(s) compiled.",
                            plan.plan.template_id,
                            plan.plan.roster.len()
                        ),
                    ),
                    Err(failure) => (
                        ContextKey::Constraints,
                        DraftVerdict::Block,
                        format!("Draft rejected by exact validator: {}", failure.error),
                    ),
                };
                let validation = DraftValidation::new(
                    &draft.draft_id,
                    &draft.draft_batch_id,
                    verdict,
                    reason,
                    SUGGESTOR_NAME,
                );
                let json = match serde_json::to_string(&validation) {
                    Ok(s) => s,
                    Err(err) => {
                        effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                            ContextKey::Diagnostic,
                            format!(
                                "draft-validation-serialize-error-{encoded_batch}-{}",
                                draft.draft_id
                            ),
                            TextPayload::new(format!(
                                "{SUGGESTOR_NAME}: failed to serialize verdict for {} in batch {draft_batch_id}: {err}",
                                draft.draft_id
                            )),
                        ));
                        continue;
                    }
                };
                effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                    target_key,
                    format!("draft-validation-{encoded_batch}-{}", draft.draft_id),
                    TextPayload::new(json),
                ));
            }

            // Sentinel for downstream gating: the critic has finished
            // its verdict pass over this draft batch. Suggestors that
            // need ordered phases (scorer in critic-gated mode) read
            // this fact to know "verdicts for this batch are now safe
            // to read."
            effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                ContextKey::Diagnostic,
                critic_pass_complete_marker(draft_batch_id),
                TextPayload::new(format!(
                    "{SUGGESTOR_NAME}: verdicts emitted for {n} draft(s) in batch {draft_batch_id}",
                    n = drafts.len()
                )),
            ));
        }

        effect.build()
    }
}

fn critic_batch_complete(ctx: &dyn Context, draft_batch_id: &str) -> bool {
    let marker = critic_pass_complete_marker(draft_batch_id);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|fact| fact.id().as_str() == marker)
}
