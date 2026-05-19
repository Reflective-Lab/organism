//! Round-over-round exclusion policy — input to round-driven
//! [`CatalogProposerSuggestor`] so each round's drafts can differ from
//! the prior round's in a way that reflects what the critics already
//! rejected.
//!
//! The proposer is otherwise deterministic over its catalog and
//! request, so without an exclusion policy a round-driven huddle
//! produces the same drafts every round and rounds collapse to mere
//! repetition. With an exclusion policy, the proposer filters its
//! catalog per-batch before compiling: descriptors the policy returns
//! are dropped, and `FormationCompiler::compile_k_candidates` runs
//! against the filtered catalog.
//!
//! Implementations of [`RoundExclusionPolicy`] should be cheap — the
//! proposer calls them once per open batch on every fire. They MUST
//! be deterministic given the same context (so re-fires produce the
//! same exclusion set) and stateless across instances (state lives
//! in the Converge context, not in the policy struct).
//!
//! The platform ships [`BlockedMinusPassedPolicy`]: exclude any
//! descriptor that appeared in a `Block`-verdict draft of a prior
//! batch and was NOT also in a `Pass`-verdict draft of that same
//! batch. That is, "responsible for failure" in a way no surviving
//! roster vouches for. Keeping a descriptor that was blocked in one
//! roster but passed in another is deliberate: the issue was the
//! combination, not the descriptor.

use std::collections::BTreeSet;
use std::fmt;

use converge_kernel::{Context, ContextKey};
use organism_catalog::SuggestorDescriptorId;

use crate::extract::{extract_draft_validations, extract_drafts};

/// Compute the per-batch descriptor exclusion set for a round-driven
/// [`CatalogProposerSuggestor`].
///
/// Called once per open batch per proposer fire. The proposer filters
/// its catalog by removing every descriptor whose id appears in the
/// returned vec, then calls
/// [`organism_runtime::FormationCompiler::compile_k_candidates`]
/// against the filtered catalog.
///
/// Implementations should be:
/// - **Pure**: same inputs → same output across re-fires.
/// - **Cheap**: called per open batch on every Converge wakeup.
/// - **Stateless**: read from `ctx`, do not retain state on `self`.
pub trait RoundExclusionPolicy: Send + Sync + fmt::Debug {
    /// Human-readable name for audit (recorded in the proposer's
    /// per-batch diagnostic fact).
    fn name(&self) -> &'static str;

    /// Descriptor ids to exclude from the catalog for `batch_id`.
    /// Returning an empty vec leaves the catalog unchanged.
    fn exclusions(&self, ctx: &dyn Context, batch_id: &str) -> Vec<SuggestorDescriptorId>;
}

/// Excludes any descriptor that appeared in a `Block`-verdict draft of
/// a prior batch and was NOT in any draft of that same batch that
/// received a `Pass` verdict. "Responsible for failure" in a way no
/// surviving roster vouches for.
///
/// Reasoning: if descriptor X was in a roster that got Blocked AND in
/// another roster of the same batch that got Passed, the Block likely
/// reflects the combination, not X itself — so X stays in the
/// catalog. If X appeared only in Blocked rosters of a batch, every
/// roster including X failed and no surviving roster vouches for it
/// — X is dropped.
///
/// Drafts of the current batch are ignored (since they have not yet
/// been judged). Drafts of all prior batches contribute their union.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlockedMinusPassedPolicy;

impl BlockedMinusPassedPolicy {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RoundExclusionPolicy for BlockedMinusPassedPolicy {
    fn name(&self) -> &'static str {
        "blocked-minus-passed"
    }

    fn exclusions(&self, ctx: &dyn Context, batch_id: &str) -> Vec<SuggestorDescriptorId> {
        let drafts = extract_drafts(ctx, ContextKey::Strategies);
        let passes = extract_draft_validations(ctx, ContextKey::Evaluations);
        let blocks = extract_draft_validations(ctx, ContextKey::Constraints);

        let mut blocked: BTreeSet<String> = BTreeSet::new();
        let mut passed_only: BTreeSet<String> = BTreeSet::new();

        for draft in &drafts {
            if draft.draft_batch_id.as_str() == batch_id {
                continue;
            }
            let was_blocked = blocks.iter().any(|v| {
                v.draft_batch_id == draft.draft_batch_id.as_str()
                    && v.draft_id == draft.draft_id.as_str()
            });
            let was_passed = passes.iter().any(|v| {
                v.draft_batch_id == draft.draft_batch_id.as_str()
                    && v.draft_id == draft.draft_id.as_str()
            });
            if was_blocked {
                for id in &draft.descriptor_ids {
                    blocked.insert(id.as_str().to_string());
                }
            } else if was_passed {
                for id in &draft.descriptor_ids {
                    passed_only.insert(id.as_str().to_string());
                }
            }
        }

        blocked
            .into_iter()
            .filter(|id| !passed_only.contains(id))
            .map(SuggestorDescriptorId::from)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_name_is_stable() {
        assert_eq!(
            BlockedMinusPassedPolicy::new().name(),
            "blocked-minus-passed"
        );
    }
}
