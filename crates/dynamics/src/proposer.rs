//! [`CatalogProposerSuggestor`] — proposes [`FormationDraft`] facts.
//!
//! Reads `ContextKey::Seeds`, calls
//! [`FormationCompiler::compile_k_candidates`] to produce up to `k`
//! distinct candidate rosters from a [`DiscoveryCatalog`], and
//! proposes one draft fact per candidate under
//! `ContextKey::Strategies` as JSON-in-TextPayload. Single-pass:
//! `accepts` returns true only when no draft fact has been emitted
//! yet under `Strategies`.
//!
//! This Suggestor *proposes* — Converge admits each proposal and
//! promotes those it accepts. The Suggestor itself does not promote.

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};
use organism_catalog::{DiscoveryCatalog, ProviderDescriptorCatalog};
use organism_runtime::{FormationCompileRequest, FormationCompiler};

use crate::extract::extract_drafts;
use crate::payload::FormationDraft;
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-catalog-proposer";

/// Catalog-backed deterministic proposer of [`FormationDraft`]s.
///
/// Holds its own copy of catalog + templates + providers + request +
/// k so the [`Suggestor`] interface (which only sees `&dyn Context`)
/// can produce drafts without those being available in context.
pub struct CatalogProposerSuggestor {
    catalog: DiscoveryCatalog,
    formation_templates: converge_kernel::formation::FormationCatalog,
    providers: ProviderDescriptorCatalog,
    request: FormationCompileRequest,
    k: usize,
    source_label: String,
}

impl CatalogProposerSuggestor {
    #[must_use]
    pub fn new(
        catalog: DiscoveryCatalog,
        formation_templates: converge_kernel::formation::FormationCatalog,
        providers: ProviderDescriptorCatalog,
        request: FormationCompileRequest,
        k: usize,
    ) -> Self {
        Self {
            catalog,
            formation_templates,
            providers,
            request,
            k,
            source_label: SUGGESTOR_NAME.to_string(),
        }
    }

    /// Override the source label recorded in emitted drafts and fact ids.
    ///
    /// This lets a design Formation host multiple catalog-backed
    /// proposers without colliding on `formation-draft-*` fact ids.
    #[must_use]
    pub fn with_source_label(mut self, source_label: impl Into<String>) -> Self {
        let source_label = source_label.into();
        self.source_label = if source_label.trim().is_empty() {
            SUGGESTOR_NAME.to_string()
        } else {
            source_label
        };
        self
    }
}

impl std::fmt::Debug for CatalogProposerSuggestor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatalogProposerSuggestor")
            .field("k", &self.k)
            .field("catalog_entries", &self.catalog.len())
            .field("source_label", &self.source_label)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Suggestor for CatalogProposerSuggestor {
    fn name(&self) -> &'static str {
        SUGGESTOR_NAME
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_DYNAMICS_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Wait for seeds, then fire once: no existing draft facts
        // under Strategies.
        ctx.has(ContextKey::Seeds) && extract_drafts(ctx, ContextKey::Strategies).is_empty()
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        let mut effect = AgentEffect::builder();
        let fact_prefix = fact_id_prefix(&self.source_label, &self.request.plan_id);

        match FormationCompiler::new().compile_k_candidates(
            &self.request,
            &self.formation_templates,
            &self.catalog,
            &self.providers,
            self.k,
        ) {
            Ok(candidates) => {
                for (index, candidate) in candidates.iter().enumerate() {
                    let descriptor_ids: Vec<String> = candidate
                        .plan
                        .roster
                        .iter()
                        .map(|r| r.suggestor_id.clone())
                        .collect();
                    // Stable draft_id = the fact id we'll use to
                    // emit. Single source of truth — the proposer
                    // writes the id into both the wire id and the
                    // payload field. Critics and the scorer route
                    // off the payload field.
                    let draft_id = format!("{fact_prefix}-{index}");
                    let draft = FormationDraft::new(
                        draft_id.clone(),
                        descriptor_ids,
                        format!(
                            "Catalog-derived candidate #{index} for template '{}'.",
                            candidate.plan.template_id
                        ),
                        self.source_label.clone(),
                    );
                    let json = match serde_json::to_string(&draft) {
                        Ok(s) => s,
                        Err(err) => {
                            effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                                ContextKey::Diagnostic,
                                format!("{fact_prefix}-serialize-error-{index}"),
                                TextPayload::new(format!(
                                    "{SUGGESTOR_NAME}: failed to serialize draft {index}: {err}"
                                )),
                            ));
                            continue;
                        }
                    };
                    effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                        ContextKey::Strategies,
                        draft_id,
                        TextPayload::new(json),
                    ));
                }
            }
            Err(failure) => {
                // Surface the failure as a diagnostic fact so the
                // design Formation can see why no drafts were
                // proposed. This is informational only; it does not
                // promote anything.
                effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                    ContextKey::Diagnostic,
                    format!("{fact_prefix}-failed"),
                    TextPayload::new(format!(
                        "{SUGGESTOR_NAME}: catalog cannot satisfy template — {}",
                        failure.error
                    )),
                ));
            }
        }

        effect.build()
    }
}

fn fact_id_prefix(source_label: &str, plan_id: &uuid::Uuid) -> String {
    format!("formation-draft-{}-{plan_id}", slug(source_label))
}

fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        SUGGESTOR_NAME.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_normalizes_source_label_for_fact_ids() {
        assert_eq!(slug("Proposer A / EU"), "proposer-a-eu");
        assert_eq!(slug(" "), SUGGESTOR_NAME);
    }
}
