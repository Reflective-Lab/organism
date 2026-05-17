//! Descriptors for `organism-learning::*` agents.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![planning_prior()]
}

#[must_use]
pub fn planning_prior() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-planning-prior",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::ExperienceLearning],
        output_keys: vec![ContextKey::Hypotheses],
        reads: vec![ContextKey::Seeds, ContextKey::Signals],
        domain_tags: vec!["learning", "prior", "experience"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Inject historical priors learned from past formation runs into the current plan.",
        use_when: "When previous runs of a similar task can usefully bias the next run's starting point.",
        examples: vec![
            "warm-start this plan from last quarter's similar runs",
            "tighten the search with what we learned about this domain",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Propose],
        produces: vec!["organism.learning.prior-injection"],
    })
}
