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

use std::collections::HashSet;

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};

use crate::extract::{extract_draft_validations, extract_drafts};
use crate::payload::{DraftVerdict, FormationDraft};
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-beauty-contest";

/// Scalar shortlist proposer over [`FormationDraft`] facts.
///
/// Two modes:
/// - **Immediate** ([`Self::new`]): fires as soon as drafts exist and
///   no shortlist exists. Does not wait for critic verdicts. Use when
///   no critic Suggestor is wired into the design Formation.
/// - **Critic-gated** ([`Self::new_critic_gated`]): also waits for the
///   [`crate::critic::CRITIC_PASS_COMPLETE_MARKER`] sentinel fact
///   before firing, so verdicts produced by
///   [`crate::DraftValidatorCriticSuggestor`] are guaranteed to be
///   visible. Without this sentinel, Converge fires the critic and
///   scorer concurrently in the same cycle and the scorer sees stale
///   (empty) verdicts.
#[derive(Debug, Clone)]
pub struct BeautyContestSuggestor {
    top_n: usize,
    wait_for_critic: bool,
}

impl BeautyContestSuggestor {
    /// Immediate-mode scorer. No verdict gating. Use when no critic is
    /// wired into the design Formation.
    #[must_use]
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n,
            wait_for_critic: false,
        }
    }

    /// Critic-gated scorer. Waits for the critic's
    /// [`crate::critic::CRITIC_PASS_COMPLETE_MARKER`] sentinel fact
    /// before firing. Use when a [`crate::DraftValidatorCriticSuggestor`]
    /// (or any compatible critic that emits the same sentinel) is
    /// composed into the design Formation.
    #[must_use]
    pub fn new_critic_gated(top_n: usize) -> Self {
        Self {
            top_n,
            wait_for_critic: true,
        }
    }
}

#[async_trait]
impl Suggestor for BeautyContestSuggestor {
    fn name(&self) -> &'static str {
        SUGGESTOR_NAME
    }

    fn dependencies(&self) -> &[ContextKey] {
        // We read drafts from Strategies, critic verdicts from
        // Evaluations and Constraints, and the critic's sentinel from
        // Diagnostic (when in critic-gated mode). All four must be
        // declared so Converge re-checks accepts() when any of them
        // changes — otherwise the scorer fires once based on stale
        // state and never wakes up after the critic emits.
        &[
            ContextKey::Strategies,
            ContextKey::Evaluations,
            ContextKey::Constraints,
            ContextKey::Diagnostic,
        ]
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_DYNAMICS_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let drafts_present = !extract_drafts(ctx, ContextKey::Strategies).is_empty();
        let no_shortlist = extract_drafts(ctx, ContextKey::Proposals).is_empty();
        if !drafts_present || !no_shortlist {
            return false;
        }
        if self.wait_for_critic {
            // Wait for the critic's sentinel before firing so verdicts
            // are guaranteed visible. Without this, Converge fires the
            // critic and scorer concurrently and the scorer reads stale
            // (empty) verdicts.
            return ctx
                .get(ContextKey::Diagnostic)
                .iter()
                .any(|fact| fact.id().as_str() == crate::critic::CRITIC_PASS_COMPLETE_MARKER);
        }
        true
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let drafts = extract_drafts(ctx, ContextKey::Strategies);

        // Build the set of draft_ids that the critic blocked. The
        // join key is `draft_id` — the proposer's stable identifier,
        // not a reconstructed index. Drafts without an explicit Block
        // verdict are eligible; this means: no critic wired → all
        // drafts eligible; critic wired and explicit Pass → eligible;
        // critic wired and explicit Block → excluded.
        let blocked_ids: HashSet<String> = extract_draft_validations(ctx, ContextKey::Constraints)
            .into_iter()
            .filter(|v| v.verdict == DraftVerdict::Block)
            .map(|v| v.draft_id)
            .collect();
        let eligible: Vec<&FormationDraft> = drafts
            .iter()
            .filter(|d| !blocked_ids.contains(&d.draft_id))
            .collect();

        // Score: v1 uses unique-descriptor count as the scalar — more
        // comprehensive rosters win ties. Stable secondary sort by
        // draft_id keeps order deterministic.
        let mut scored: Vec<(usize, &FormationDraft)> =
            eligible.into_iter().map(|d| (score_draft(d), d)).collect();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.draft_id.cmp(&b.1.draft_id))
                .then_with(|| a.1.descriptor_ids.cmp(&b.1.descriptor_ids))
        });

        let mut effect = AgentEffect::builder();
        for (index, (_score, draft)) in scored.into_iter().take(self.top_n).enumerate() {
            // Rebuild with a fresh rationale that records the
            // shortlist position; preserve the original proposer's
            // draft_id and source so downstream consumers can still
            // join back to the originating draft.
            let shortlist = FormationDraft::new(
                draft.draft_id.clone(),
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

/// v1 scoring: unique roster size. Wider coverage wins, but duplicate
/// descriptor ids never inflate the score. Replace with a richer
/// heuristic when there is real data to learn from.
fn score_draft(draft: &FormationDraft) -> usize {
    let mut unique = std::collections::BTreeSet::new();
    draft
        .descriptor_ids
        .iter()
        .filter(|id| unique.insert(id.as_str()))
        .count()
}
