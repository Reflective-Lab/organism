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

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};
use organism_catalog::{DiscoveryCatalog, ProviderDescriptorCatalog};
use organism_runtime::{FormationCompileRequest, FormationCompiler};

use crate::compile::compile_draft;
use crate::extract::extract_drafts;
use crate::payload::{DraftValidation, DraftVerdict};
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-draft-validator-critic";

/// Sentinel fact id the critic emits under `ContextKey::Diagnostic`
/// once it has produced verdicts for the current draft set. Downstream
/// Suggestors that need to wait for verdicts (e.g.
/// [`crate::BeautyContestSuggestor`] in critic-gated mode) check for
/// this fact to know "the critic has spoken; verdicts are now safe to
/// read." This is the join sentinel that turns concurrent
/// per-round firing into ordered phases.
pub const CRITIC_PASS_COMPLETE_MARKER: &str = "organism-dynamics-critic-pass-complete";

/// Catalog-aware critic that emits one [`DraftValidation`] verdict per
/// [`FormationDraft`] fact found under `ContextKey::Strategies`.
///
/// Single-pass: fires when at least one draft is present and no
/// validation has been emitted yet.
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

/// Per-source index counter so verdicts can join back to drafts by
/// `(draft_source, draft_index)`. The proposer assigns indices in the
/// order it emits drafts; the critic recovers them by scanning the
/// extracted drafts in order, restarting the count per source.
fn assign_indices(
    drafts: &[crate::payload::FormationDraft],
) -> Vec<(usize, &crate::payload::FormationDraft)> {
    use std::collections::HashMap;
    let mut next: HashMap<&str, usize> = HashMap::new();
    drafts
        .iter()
        .map(|d| {
            let idx = next.entry(d.source.as_str()).or_insert(0);
            let here = *idx;
            *idx += 1;
            (here, d)
        })
        .collect()
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
        // Fire once: drafts exist, no verdicts yet on either side.
        let has_drafts = !extract_drafts(ctx, ContextKey::Strategies).is_empty();
        let has_verdicts = has_validation_fact(ctx, ContextKey::Evaluations)
            || has_validation_fact(ctx, ContextKey::Constraints);
        has_drafts && !has_verdicts
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let drafts = extract_drafts(ctx, ContextKey::Strategies);
        let compiler = FormationCompiler::new();
        let mut effect = AgentEffect::builder();

        for (index, draft) in assign_indices(&drafts) {
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
            let validation =
                DraftValidation::new(&draft.source, index, verdict, reason, SUGGESTOR_NAME);
            let json = match serde_json::to_string(&validation) {
                Ok(s) => s,
                Err(err) => {
                    effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                        ContextKey::Diagnostic,
                        format!("draft-validation-serialize-error-{}-{index}", draft.source),
                        TextPayload::new(format!(
                            "{SUGGESTOR_NAME}: failed to serialize verdict for ({}, {index}): {err}",
                            draft.source
                        )),
                    ));
                    continue;
                }
            };
            effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                target_key,
                format!("draft-validation-{}-{index}", draft.source),
                TextPayload::new(json),
            ));
        }

        // Sentinel for downstream gating: the critic has finished its
        // verdict pass over the current draft set. Suggestors that
        // need ordered phases (scorer in critic-gated mode) read this
        // fact to know "verdicts are now safe to read."
        effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
            ContextKey::Diagnostic,
            CRITIC_PASS_COMPLETE_MARKER,
            TextPayload::new(format!(
                "{SUGGESTOR_NAME}: verdicts emitted for {n} draft(s)",
                n = drafts.len()
            )),
        ));

        effect.build()
    }
}

/// Returns true if the given `key` already holds at least one
/// [`DraftValidation`] fact. Used by `accepts` so the critic doesn't
/// re-fire after its single pass.
fn has_validation_fact(ctx: &dyn Context, key: ContextKey) -> bool {
    ctx.get(key).iter().any(|fact| {
        fact.payload::<TextPayload>()
            .and_then(|t| serde_json::from_str::<DraftValidation>(t.as_str()).ok())
            .is_some_and(|v| v.is_well_formed())
    })
}
