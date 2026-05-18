//! [`CatalogProposerSuggestor`] — proposes [`FormationDraft`] facts.
//!
//! Reads `ContextKey::Seeds`, calls
//! [`FormationCompiler::compile_k_candidates`] to produce up to `k`
//! distinct candidate rosters from a [`DiscoveryCatalog`], and
//! proposes one draft fact per candidate under
//! `ContextKey::Strategies` as JSON-in-TextPayload.
//!
//! ## Batch lifecycle
//!
//! Each emitted draft carries a `draft_batch_id`. Two modes pick that
//! id:
//!
//! - **Explicit** ([`Self::new`] / [`Self::with_batch_id`]): the
//!   proposer owns a single `draft_batch_id` and fires once for that
//!   batch when seeds are present. Use for one-shot tests or when the
//!   host wants total control of batching.
//! - **Round-driven** ([`Self::with_round_signals`]): the proposer
//!   watches [`ContextKey::Signals`] for facts whose ids start with a
//!   configured prefix and treats each fact id as the
//!   `draft_batch_id` for one open round. Pair with the
//!   `organism_runtime::huddle::RoundStarter` configured with a
//!   matching `round_signal_prefix` (and `round_signal_key` set to
//!   `Signals`) so the design huddle Formation owns its own batch
//!   lifecycle — round 1 produces batch A, round 2 produces batch B,
//!   and so on.
//!
//! In both modes the proposer's `accepts` is per-batch: drafts for one
//! batch never block drafts for another, and there is no global
//! "drafts exist anywhere" gate. The draft id stored in each draft's
//! payload is `candidate-{index}` and is **reusable across batches** —
//! `(draft_batch_id, draft_id)` is the join key. Fact ids on the wire
//! remain globally unique by hex-encoding the batch id.
//!
//! This Suggestor *proposes* — Converge admits each proposal and
//! promotes those it accepts. The Suggestor itself does not promote.

use std::collections::HashSet;

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};
use organism_catalog::{DiscoveryCatalog, ProviderDescriptorCatalog};
use organism_runtime::{FormationCompileRequest, FormationCompiler};

use crate::batch::encode_batch_id;
use crate::extract::extract_drafts;
use crate::payload::FormationDraft;
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-catalog-proposer";

/// How [`CatalogProposerSuggestor`] picks the `draft_batch_id` to
/// stamp on each emitted draft.
#[derive(Debug, Clone)]
enum BatchSource {
    /// Single fixed batch id; one-shot per proposer instance.
    Explicit(String),
    /// Watch [`ContextKey::Signals`] for facts whose id starts with
    /// `signal_prefix` and treat each fact id as an open batch id.
    /// Pair with `organism_runtime::huddle::RoundStarter` configured
    /// with the same `round_signal_prefix` and `round_signal_key =
    /// Signals`.
    RoundSignals { signal_prefix: &'static str },
}

/// Catalog-backed deterministic proposer of [`FormationDraft`]s.
pub struct CatalogProposerSuggestor {
    catalog: DiscoveryCatalog,
    formation_templates: converge_kernel::formation::FormationCatalog,
    providers: ProviderDescriptorCatalog,
    request: FormationCompileRequest,
    k: usize,
    source_label: String,
    batch_source: BatchSource,
}

// Declared statically so [`Suggestor::dependencies`] can return a
// borrow. Round-driven instances must wake when new round signals
// land under `ContextKey::Signals`, so it is declared unconditionally
// — declaring more keys than strictly needed is harmless (Converge
// just re-checks `accepts` more often).
const PROPOSER_DEPENDENCIES: &[ContextKey] = &[ContextKey::Seeds, ContextKey::Signals];

impl CatalogProposerSuggestor {
    #[must_use]
    pub fn new(
        catalog: DiscoveryCatalog,
        formation_templates: converge_kernel::formation::FormationCatalog,
        providers: ProviderDescriptorCatalog,
        request: FormationCompileRequest,
        k: usize,
    ) -> Self {
        let source_label = SUGGESTOR_NAME.to_string();
        let batch_id = default_batch_id(&source_label, &request.plan_id);
        Self {
            catalog,
            formation_templates,
            providers,
            request,
            k,
            source_label,
            batch_source: BatchSource::Explicit(batch_id),
        }
    }

    /// Override the source label recorded in emitted drafts.
    ///
    /// In explicit-batch mode this also resets the batch id to track
    /// the new label; call [`Self::with_batch_id`] after this to set a
    /// custom batch. In round-driven mode the source label is purely
    /// audit metadata.
    #[must_use]
    pub fn with_source_label(mut self, source_label: impl Into<String>) -> Self {
        let source_label = source_label.into();
        self.source_label = if source_label.trim().is_empty() {
            SUGGESTOR_NAME.to_string()
        } else {
            source_label
        };
        if matches!(self.batch_source, BatchSource::Explicit(_)) {
            self.batch_source =
                BatchSource::Explicit(default_batch_id(&self.source_label, &self.request.plan_id));
        }
        self
    }

    /// Switch to explicit-batch mode with the given `batch_id`. Use
    /// when running multiple proposer instances in the same design
    /// Formation: each instance gets a distinct `batch_id` so the
    /// critic and scorer can route per-batch without temporal
    /// contamination.
    #[must_use]
    pub fn with_batch_id(mut self, batch_id: impl Into<String>) -> Self {
        let batch_id = batch_id.into();
        if !batch_id.trim().is_empty() {
            self.batch_source = BatchSource::Explicit(batch_id);
        }
        self
    }

    /// Switch to round-driven mode. The proposer will read facts at
    /// [`ContextKey::Signals`] and, for every fact id that starts
    /// with `signal_prefix` and has no drafts yet, propose a draft
    /// batch using that fact id as the `draft_batch_id`.
    ///
    /// Pair with `organism_runtime::huddle::RoundStarter::with_conventions`
    /// configured so its `round_signal_key` is `Signals` and its
    /// `round_signal_prefix` matches `signal_prefix`. The design
    /// huddle Formation then owns its own batch lifecycle: round N
    /// produces batch N, with no caller-supplied batch ids.
    ///
    /// Only `Signals` is accepted as the source key — `Suggestor`
    /// dependencies are static so wiring an arbitrary key would make
    /// Converge silently skip wakeups for that key. Use a custom
    /// `round_signal_prefix` if you need to distinguish multiple
    /// round streams on the same key.
    #[must_use]
    pub fn with_round_signals(mut self, signal_prefix: &'static str) -> Self {
        self.batch_source = BatchSource::RoundSignals { signal_prefix };
        self
    }

    /// Returns the proposer's explicit batch id, or `None` if it is in
    /// round-driven mode (where batch ids are taken from incoming
    /// round signals).
    #[must_use]
    pub fn batch_id(&self) -> Option<&str> {
        match &self.batch_source {
            BatchSource::Explicit(id) => Some(id),
            BatchSource::RoundSignals { .. } => None,
        }
    }

    /// Batch ids that have a round signal but no drafts yet. Sorted
    /// for determinism. Returns the explicit id (if no drafts) in
    /// explicit mode.
    fn open_batches(&self, ctx: &dyn Context) -> Vec<String> {
        let existing: HashSet<String> = extract_drafts(ctx, ContextKey::Strategies)
            .into_iter()
            .map(|d| d.draft_batch_id)
            .collect();
        match &self.batch_source {
            BatchSource::Explicit(id) => {
                if existing.contains(id) {
                    vec![]
                } else {
                    vec![id.clone()]
                }
            }
            BatchSource::RoundSignals { signal_prefix } => {
                let mut open: Vec<String> = ctx
                    .get(ContextKey::Signals)
                    .iter()
                    .filter_map(|fact| {
                        let id = fact.id().as_str();
                        if id.starts_with(*signal_prefix) && !existing.contains(id) {
                            Some(id.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                open.sort();
                open.dedup();
                open
            }
        }
    }
}

fn default_batch_id(source_label: &str, plan_id: &uuid::Uuid) -> String {
    format!("{}-{plan_id}", slug(source_label))
}

impl std::fmt::Debug for CatalogProposerSuggestor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatalogProposerSuggestor")
            .field("k", &self.k)
            .field("catalog_entries", &self.catalog.len())
            .field("source_label", &self.source_label)
            .field("batch_source", &self.batch_source)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Suggestor for CatalogProposerSuggestor {
    fn name(&self) -> &'static str {
        SUGGESTOR_NAME
    }

    fn dependencies(&self) -> &[ContextKey] {
        PROPOSER_DEPENDENCIES
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_DYNAMICS_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Seeds) {
            return false;
        }
        !self.open_batches(ctx).is_empty()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut effect = AgentEffect::builder();
        let open_batches = self.open_batches(ctx);

        // Compile once per execute. The catalog/templates/request are
        // immutable on the proposer, so the candidate set is the same
        // for every batch in this call. The batch id is what
        // distinguishes them on the wire.
        let candidates = match FormationCompiler::new().compile_k_candidates(
            &self.request,
            &self.formation_templates,
            &self.catalog,
            &self.providers,
            self.k,
        ) {
            Ok(candidates) => candidates,
            Err(failure) => {
                // Surface the failure once per open batch so the audit
                // trail records which rounds saw no proposals.
                for batch_id in open_batches {
                    let encoded = encode_batch_id(&batch_id);
                    effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                        ContextKey::Diagnostic,
                        format!("{SUGGESTOR_NAME}-failed-{encoded}"),
                        TextPayload::new(format!(
                            "{SUGGESTOR_NAME}: catalog cannot satisfy template for batch {batch_id} — {}",
                            failure.error
                        )),
                    ));
                }
                return effect.build();
            }
        };

        for batch_id in open_batches {
            let encoded_batch = encode_batch_id(&batch_id);
            for (index, candidate) in candidates.iter().enumerate() {
                let descriptor_ids: Vec<String> = candidate
                    .plan
                    .roster
                    .iter()
                    .map(|r| r.suggestor_id.clone())
                    .collect();
                // Per-batch draft id — reusable across batches so
                // round-driven huddles can compare "round 1's
                // candidate-0" against "round 2's candidate-0". The
                // join key is the (draft_batch_id, draft_id) pair;
                // global uniqueness lives on the wire fact id.
                let draft_id = format!("candidate-{index}");
                let draft = FormationDraft::new(
                    draft_id.clone(),
                    batch_id.clone(),
                    descriptor_ids,
                    format!(
                        "Catalog-derived candidate #{index} for template '{}' (batch: {batch_id}).",
                        candidate.plan.template_id
                    ),
                    self.source_label.clone(),
                );
                let json = match serde_json::to_string(&draft) {
                    Ok(s) => s,
                    Err(err) => {
                        effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                            ContextKey::Diagnostic,
                            format!("formation-draft-serialize-error-{encoded_batch}-{index}"),
                            TextPayload::new(format!(
                                "{SUGGESTOR_NAME}: failed to serialize draft {index} for batch {batch_id}: {err}"
                            )),
                        ));
                        continue;
                    }
                };
                effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                    ContextKey::Strategies,
                    format!("formation-draft-{encoded_batch}-{index}"),
                    TextPayload::new(json),
                ));
            }
        }

        effect.build()
    }
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
