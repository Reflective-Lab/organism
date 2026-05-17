//! Descriptors for `converge-mnemos-knowledge` Suggestors.
//!
//! Authored against `converge-mnemos-knowledge = "1.2.2"`. Mnemos
//! exposes knowledge-base retrieval and storage primitives.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![knowledge_retrieval(), knowledge_store()]
}

#[must_use]
pub fn knowledge_retrieval() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "mnemos-knowledge-retrieval",
        role: SuggestorRole::Signal,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Signals, ContextKey::Hypotheses],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["knowledge", "retrieval", "kb"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Retrieve grounding facts from the configured knowledge base.",
        use_when: "When a decision needs grounding against the org's accumulated knowledge.",
        examples: vec![
            "what do we know about this vendor already",
            "pull prior decisions on similar topics",
        ],
        loop_contributions: vec![LoopContribution::Retrieve],
        produces: vec!["mnemos.knowledge.retrieval"],
    })
}

#[must_use]
pub fn knowledge_store() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "mnemos-knowledge-store",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Diagnostic],
        reads: vec![ContextKey::Proposals, ContextKey::ConsensusOutcomes],
        domain_tags: vec!["knowledge", "store", "kb", "memory"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Write a promoted decision back to the knowledge base for future grounding.",
        use_when: "After a decision lands and the org wants the rationale captured for future retrieval.",
        examples: vec![
            "record this decision in the KB",
            "store the rationale so we don't re-litigate next quarter",
        ],
        loop_contributions: vec![LoopContribution::Synthesize, LoopContribution::Observe],
        produces: vec!["mnemos.knowledge.store"],
    })
}
