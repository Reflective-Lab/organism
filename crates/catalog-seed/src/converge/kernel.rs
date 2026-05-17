//! Descriptors for kernel-facing Suggestors.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![provider_selection()]
}

#[must_use]
pub fn provider_selection() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-provider-selection",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints],
        domain_tags: vec!["provider", "selection", "backend-matching"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Pick the best provider backend for a Suggestor role given backend requirements.",
        use_when: "When a role needs a provider (LLM, policy engine, solver) and the catalog has multiple matches.",
        examples: vec![
            "match this LLM role to a provider with EU sovereignty + structured-output",
            "choose a cheap provider for a low-stakes summarization role",
        ],
        loop_contributions: vec![LoopContribution::Score],
        produces: vec!["converge.kernel.provider-selection"],
    })
}
