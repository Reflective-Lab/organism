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

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{Provenance, ProvenanceSource, Suggestor, TextPayload};

use crate::batch::{decode_batch_id, encode_batch_id};
use crate::extract::{extract_draft_validations, extract_drafts};
use crate::payload::{DraftBatchId, DraftId, DraftVerdict, FormationDraft};
use crate::provenance::ORGANISM_DYNAMICS_PROVENANCE;

const SUGGESTOR_NAME: &str = "organism-beauty-contest";

/// Prefix for the per-batch scorer-completion sentinel under
/// `ContextKey::Diagnostic`. The scorer emits this fact for every
/// batch it processes — including batches that produced an empty
/// shortlist because every draft was blocked. Without this sentinel,
/// the scorer's `accepts` gate ("batch has drafts but no shortlist
/// yet") would stay true forever on all-blocked batches, looping the
/// Formation. Use [`scorer_batch_complete_marker`] to build the full
/// id.
pub const SCORER_BATCH_COMPLETE_PREFIX: &str = "organism-dynamics-scorer-batch-complete";

/// Returns the diagnostic fact id for a completed scorer pass over
/// `draft_batch_id`.
#[must_use]
pub fn scorer_batch_complete_marker(draft_batch_id: &str) -> String {
    format!(
        "{SCORER_BATCH_COMPLETE_PREFIX}-{}",
        encode_batch_id(draft_batch_id)
    )
}

/// Scalar shortlist proposer over [`FormationDraft`] facts.
///
/// Two modes:
/// - **Immediate** ([`Self::new`]): fires as soon as drafts exist and
///   no shortlist exists. Does not wait for critic verdicts. Use when
///   no critic Suggestor is wired into the design Formation.
/// - **Critic-gated** ([`Self::new_critic_gated`]): also waits for the
///   [`crate::critic::critic_pass_complete_marker`] sentinel fact for
///   a draft batch before shortlisting that batch, so verdicts produced by
///   [`crate::DraftValidatorCriticSuggestor`] are guaranteed to be
///   visible for that batch. Without this sentinel, Converge fires
///   the critic and scorer concurrently in the same cycle and the
///   scorer sees stale (empty) verdicts.
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

    /// Critic-gated scorer. Waits for the critic's batch-scoped
    /// [`crate::critic::critic_pass_complete_marker`] sentinel fact
    /// before shortlisting a batch. Use when a
    /// [`crate::DraftValidatorCriticSuggestor`] (or any compatible
    /// critic that emits the same sentinel) is composed into the
    /// design Formation.
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

    fn provenance(&self) -> Provenance {
        Provenance::from(ORGANISM_DYNAMICS_PROVENANCE.as_str())
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Per-batch gate. The scorer accepts when at least one batch
        // has:
        //   - drafts under Strategies
        //   - no scorer-completion sentinel under Diagnostic for that
        //     batch yet (presence of shortlist drafts alone would
        //     loop on all-blocked batches that produce zero drafts)
        //   - (in critic-gated mode) the critic's per-batch sentinel
        //
        // The scorer can therefore process batch N+1 even after batch
        // N has already been scored — no cross-batch contamination,
        // no deadlock when only later batches are pending.
        let drafts = extract_drafts(ctx, ContextKey::Strategies);
        if drafts.is_empty() {
            return false;
        }
        let scored_batches = scored_batches(ctx);
        let pending_batches: HashSet<&str> = drafts
            .iter()
            .map(|d| d.draft_batch_id.as_str())
            .filter(|b| !scored_batches.contains(*b))
            .collect();
        if pending_batches.is_empty() {
            return false;
        }
        if self.wait_for_critic {
            // Require the per-batch sentinel for at least one pending
            // batch — otherwise the verdicts aren't safe to read for
            // any batch we'd score.
            return pending_batches.iter().any(|batch| {
                let marker = crate::critic::critic_pass_complete_marker(batch);
                ctx.get(ContextKey::Diagnostic)
                    .iter()
                    .any(|fact| fact.id().as_str() == marker)
            });
        }
        true
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let drafts = extract_drafts(ctx, ContextKey::Strategies);

        // Group eligible drafts by batch. A batch is eligible when
        // we haven't emitted a scorer-completion sentinel for it yet
        // AND (in critic-gated mode) the critic's sentinel for that
        // batch is present. Within an eligible batch, blocked drafts
        // are dropped before scoring.
        //
        // Empty-but-eligible batches still get processed below —
        // their scorer-completion sentinel is emitted with zero
        // shortlist facts so the gate flips false and the Formation
        // converges.
        let scored_batches = scored_batches(ctx);
        let blocked_ids: HashSet<(DraftBatchId, DraftId)> =
            extract_draft_validations(ctx, ContextKey::Constraints)
                .into_iter()
                .filter(|v| v.verdict == DraftVerdict::Block)
                .map(|v| (v.draft_batch_id, v.draft_id))
                .collect();

        let mut by_batch: BTreeMap<DraftBatchId, Vec<&FormationDraft>> = BTreeMap::new();
        for draft in &drafts {
            if scored_batches.contains(&draft.draft_batch_id) {
                continue;
            }
            if self.wait_for_critic {
                let marker = crate::critic::critic_pass_complete_marker(&draft.draft_batch_id);
                let has_sentinel = ctx
                    .get(ContextKey::Diagnostic)
                    .iter()
                    .any(|fact| fact.id().as_str() == marker);
                if !has_sentinel {
                    continue;
                }
            }
            // Even when this individual draft is blocked, ensure the
            // batch has an entry in by_batch so we still emit the
            // scorer-completion sentinel for it.
            let entry = by_batch.entry(draft.draft_batch_id.clone()).or_default();
            if blocked_ids.contains(&(draft.draft_batch_id.clone(), draft.draft_id.clone())) {
                continue;
            }
            entry.push(draft);
        }

        let mut effect = AgentEffect::builder();
        for (batch_id, eligible) in by_batch {
            // Score: v1 uses unique-descriptor count as the scalar —
            // more comprehensive rosters win ties. Stable secondary
            // sort by draft_id keeps order deterministic per batch.
            let mut scored: Vec<(usize, &FormationDraft)> =
                eligible.into_iter().map(|d| (score_draft(d), d)).collect();
            scored.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| a.1.draft_id.cmp(&b.1.draft_id))
                    .then_with(|| a.1.descriptor_ids.cmp(&b.1.descriptor_ids))
            });

            let encoded_batch = encode_batch_id(&batch_id);
            for (index, (_score, draft)) in scored.into_iter().take(self.top_n).enumerate() {
                // Preserve the original draft_id AND draft_batch_id
                // so downstream consumers can still join back to the
                // originating draft and the batch it came from.
                let shortlist = FormationDraft::new(
                    draft.draft_id.clone(),
                    draft.draft_batch_id.clone(),
                    draft.descriptor_ids.clone(),
                    format!(
                        "Shortlisted #{index}/{} by {SUGGESTOR_NAME} (batch: {batch_id}, source: {}).",
                        self.top_n, draft.source,
                    ),
                    draft.source.clone(),
                );
                let json = match serde_json::to_string(&shortlist) {
                    Ok(s) => s,
                    Err(err) => {
                        effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                            ContextKey::Diagnostic,
                            format!("shortlist-serialize-error-{encoded_batch}-{index}"),
                            TextPayload::new(format!(
                                "{SUGGESTOR_NAME}: failed to serialize shortlist {index} for batch {batch_id}: {err}"
                            )),
                        ));
                        continue;
                    }
                };
                effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                    ContextKey::Proposals,
                    format!("formation-draft-shortlist-{encoded_batch}-{index}"),
                    TextPayload::new(json),
                ));
            }

            // Per-batch scorer-completion sentinel. Emitted even when
            // zero drafts were shortlisted (every draft blocked, etc.)
            // so accepts() can flip false and the Formation converges.
            effect = effect.proposal(ORGANISM_DYNAMICS_PROVENANCE.proposed_fact(
                ContextKey::Diagnostic,
                scorer_batch_complete_marker(&batch_id),
                TextPayload::new(format!(
                    "{SUGGESTOR_NAME}: scoring complete for batch {batch_id}"
                )),
            ));
        }

        effect.build()
    }
}

fn scored_batches(ctx: &dyn Context) -> HashSet<DraftBatchId> {
    let mut batches: HashSet<DraftBatchId> = ctx
        .get(ContextKey::Diagnostic)
        .iter()
        .filter_map(|fact| {
            fact.id()
                .as_str()
                .strip_prefix(SCORER_BATCH_COMPLETE_PREFIX)
                .and_then(|rest| rest.strip_prefix('-'))
                .and_then(decode_batch_id)
                .map(DraftBatchId::from)
        })
        .collect();
    batches.extend(
        extract_drafts(ctx, ContextKey::Proposals)
            .into_iter()
            .map(|draft| draft.draft_batch_id),
    );
    batches
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
