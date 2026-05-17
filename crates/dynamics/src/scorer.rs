//! [`BeautyContestSuggestor`] — proposes a top-N shortlist of
//! [`FormationDraft`] facts.
//!
//! Reads draft facts under `ContextKey::Strategies` (via the strict
//! [`crate::extract::extract_drafts`] parser, so non-draft facts on
//! the same key are ignored), scores them with a deterministic v1
//! heuristic, and proposes the top-N as drafts under
//! `ContextKey::Proposals` for downstream extraction.
//!
//! "Beauty contest" is a deliberate misnomer for v1 — the scoring is
//! a simple scalar based on declared roster size; later slices add
//! richer scoring (per-descriptor cost, governance class affinity,
//! domain-tag overlap, learned priors). The Suggestor *proposes*
//! shortlist facts; Converge admits and may promote.

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};

use crate::extract::extract_drafts;
use crate::payload::FormationDraft;
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-beauty-contest";

/// Scalar shortlist proposer over [`FormationDraft`] facts.
#[derive(Debug, Clone)]
pub struct BeautyContestSuggestor {
    top_n: usize,
}

impl BeautyContestSuggestor {
    /// Build a scorer that proposes the top `top_n` drafts.
    #[must_use]
    pub fn new(top_n: usize) -> Self {
        Self { top_n }
    }
}

#[async_trait]
impl Suggestor for BeautyContestSuggestor {
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
        // Fire once: at least one draft proposed under Strategies and
        // no shortlist draft yet under Proposals.
        !extract_drafts(ctx, ContextKey::Strategies).is_empty()
            && extract_drafts(ctx, ContextKey::Proposals).is_empty()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let drafts = extract_drafts(ctx, ContextKey::Strategies);

        // Score: v1 uses descriptor count as the scalar — more
        // comprehensive rosters win ties. Stable secondary sort by
        // source then by serialized id list keeps order deterministic.
        let mut scored: Vec<(usize, &FormationDraft)> =
            drafts.iter().map(|d| (score_draft(d), d)).collect();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.source.cmp(&b.1.source))
                .then_with(|| a.1.descriptor_ids.cmp(&b.1.descriptor_ids))
        });

        let mut effect = AgentEffect::builder();
        for (index, (_score, draft)) in scored.into_iter().take(self.top_n).enumerate() {
            // Rebuild with a fresh rationale that records the
            // shortlist position; the source stays as whoever
            // originally proposed the draft.
            let shortlist = FormationDraft::new(
                draft.descriptor_ids.clone(),
                format!(
                    "Shortlisted #{index}/{} by {SUGGESTOR_NAME} (source: {}).",
                    self.top_n, draft.source,
                ),
                draft.source.clone(),
            );
            let json = match serde_json::to_string(&shortlist) {
                Ok(s) => s,
                Err(err) => {
                    effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                        ContextKey::Diagnostic,
                        format!("shortlist-serialize-error-{index}"),
                        TextPayload::new(format!(
                            "{SUGGESTOR_NAME}: failed to serialize shortlist {index}: {err}"
                        )),
                    ));
                    continue;
                }
            };
            effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                ContextKey::Proposals,
                format!("formation-draft-shortlist-{index}"),
                TextPayload::new(json),
            ));
        }

        effect.build()
    }
}

/// v1 scoring: roster size. Wider coverage wins. Replace with a
/// richer heuristic when there is real data to learn from.
fn score_draft(draft: &FormationDraft) -> usize {
    draft.descriptor_ids.len()
}
