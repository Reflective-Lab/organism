//! Descriptors for `organism-dynamics::*` — the design-huddle
//! Suggestors that turn deliberation into a Formation.
//!
//! These three are meta-Suggestors: they don't fill a work
//! Formation's slots directly. They compose the **design**
//! Formation that picks which work Formation to run. The catalog
//! still names them so an LLM-backed lookup or a host that wants
//! to assemble a design huddle programmatically can discover them
//! the same way it discovers any other Suggestor.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        catalog_proposer(),
        draft_validator_critic(),
        beauty_contest(),
    ]
}

/// [`organism_dynamics::CatalogProposerSuggestor`] — proposes
/// `FormationDraft` candidates by enumerating k-best rosters from a
/// [`organism_catalog::DiscoveryCatalog`]. The proposer slot in the
/// design huddle.
#[must_use]
pub fn catalog_proposer() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-catalog-proposer",
        role: SuggestorRole::Meta,
        capabilities: vec![
            SuggestorCapability::Optimization,
            SuggestorCapability::Analytics,
        ],
        output_keys: vec![ContextKey::Strategies, ContextKey::Diagnostic],
        reads: vec![ContextKey::Seeds, ContextKey::Signals],
        domain_tags: vec!["dynamics", "huddle", "proposer", "draft"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Propose FormationDraft candidates by compiling up to k diverse rosters from the discovery catalog.",
        use_when: "Inside a design huddle Formation, in either explicit-batch mode or round-driven mode where RoundStarter owns the batch lifecycle.",
        examples: vec![
            "give me 3 candidate work formations for this intent",
            "open a new batch of drafts for the next round",
        ],
        loop_contributions: vec![LoopContribution::Propose],
        produces: vec!["organism.dynamics.formation-draft"],
    })
}

/// [`organism_dynamics::DraftValidatorCriticSuggestor`] — the
/// admissibility gate on each emitted draft. Per-batch sentinel under
/// `Diagnostic` lets the scorer fire only on completed critic passes.
#[must_use]
pub fn draft_validator_critic() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-draft-validator-critic",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![
            ContextKey::Evaluations,
            ContextKey::Constraints,
            ContextKey::Diagnostic,
        ],
        reads: vec![ContextKey::Strategies],
        domain_tags: vec!["dynamics", "huddle", "critic", "validation"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Validate each FormationDraft against the catalog and emit a typed Pass/Block verdict per draft, plus a per-batch completion sentinel.",
        use_when: "Inside a design huddle, after the proposer has emitted drafts and before the scorer shortlists. Wires the temporal phase boundary between propose and score.",
        examples: vec![
            "is this draft admissible against the work template",
            "which drafts in the current batch should the scorer consider",
        ],
        loop_contributions: vec![LoopContribution::Observe, LoopContribution::Score],
        produces: vec!["organism.dynamics.draft-validation"],
    })
}

/// [`organism_dynamics::BeautyContestSuggestor`] — top-N shortlist
/// over the per-batch admissible draft pool, with optional critic-
/// gated mode for ordered phases.
#[must_use]
pub fn beauty_contest() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-beauty-contest",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Proposals, ContextKey::Diagnostic],
        reads: vec![
            ContextKey::Strategies,
            ContextKey::Evaluations,
            ContextKey::Constraints,
            ContextKey::Diagnostic,
        ],
        domain_tags: vec!["dynamics", "huddle", "scorer", "shortlist"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Shortlist the top-N FormationDrafts per batch using a deterministic scalar score; emit a per-batch completion sentinel even when every draft is blocked.",
        use_when: "Inside a design huddle, after critic verdicts are in. The critic-gated mode waits for the critic's per-batch sentinel before scoring that batch — pair with DraftValidatorCriticSuggestor.",
        examples: vec![
            "shortlist the top 2 drafts in this batch",
            "pick the best work formation candidate from the current round",
        ],
        loop_contributions: vec![LoopContribution::Score],
        produces: vec!["organism.dynamics.formation-draft"],
    })
}
