//! Descriptors for `organism-runtime::*` agents.
//!
//! ## Exclusion rule for generic Suggestors
//!
//! Suggestors that are generic over a host-supplied type parameter
//! AND whose `name()` is derived from the constructed instance (e.g.
//! `organism-planning::StabilitySuggestor<F>`) are intentionally
//! omitted from this seed. Their catalog identity is not stable until
//! the host monomorphizes them, and each host instantiation produces a
//! distinct effective Suggestor with a distinct name. Hosts that wire
//! such Suggestors should register a per-instance
//! [`CatalogSuggestorDescriptor`] alongside the factory at runtime.
//!
//! Generic Suggestors that DO have a stable static name (e.g.
//! `organism-round-synthesizer`, generic over `SynthesisProducer` but
//! returns a fixed `&'static str` from `name()`) are included here as
//! the descriptor identifies the surface, not the parameter.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        problem_classifier(),
        role_stall(),
        round_starter(),
        round_synthesizer(),
        disagreement_mapper(),
        consensus_evaluator(),
    ]
}

#[must_use]
pub fn problem_classifier() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-problem-classifier",
        role: SuggestorRole::Analysis,
        capabilities: vec![
            SuggestorCapability::LlmReasoning,
            SuggestorCapability::Analytics,
        ],
        output_keys: vec![ContextKey::Hypotheses, ContextKey::Diagnostic],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["runtime", "classification", "intake"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Classify an incoming intent into a problem class to pick the right formation template.",
        use_when: "At the top of the pipeline, before formation selection.",
        examples: vec![
            "is this a vendor selection or a hiring decision",
            "which formation template fits this intent",
        ],
        loop_contributions: vec![LoopContribution::Propose, LoopContribution::Observe],
        produces: vec!["organism.runtime.problem-class"],
    })
}

#[must_use]
pub fn role_stall() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-role-stall",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::HumanInTheLoop],
        output_keys: vec![ContextKey::Diagnostic],
        reads: vec![ContextKey::Hypotheses, ContextKey::Disagreements],
        domain_tags: vec!["runtime", "stall", "hitl", "telemetry"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Detect when a deliberation role is stalling and emit a UserCorrection event.",
        use_when: "When a huddle is not converging and a human nudge may be needed.",
        examples: vec![
            "is this huddle going in circles",
            "surface stall conditions for the operator",
        ],
        loop_contributions: vec![LoopContribution::Observe],
        produces: vec!["organism.runtime.role-stall"],
    })
}

#[must_use]
pub fn round_starter() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-round-starter",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Hypotheses],
        reads: vec![ContextKey::Seeds, ContextKey::Hypotheses],
        domain_tags: vec!["runtime", "huddle", "round"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Open a new huddle round with a prompt sized to the open questions.",
        use_when: "At the start of each deliberation round inside a huddle.",
        examples: vec![
            "open round 2 focused on the remaining unknowns",
            "frame the next round around the disagreement",
        ],
        loop_contributions: vec![LoopContribution::Propose],
        produces: vec!["organism.runtime.huddle-round"],
    })
}

#[must_use]
pub fn round_synthesizer() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-round-synthesizer",
        role: SuggestorRole::Synthesis,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Proposals, ContextKey::Hypotheses],
        reads: vec![ContextKey::Hypotheses],
        domain_tags: vec!["runtime", "huddle", "synthesis"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Synthesize the end-of-round outcome from accumulated hypotheses.",
        use_when: "When a huddle round closes and per-round hypotheses must be combined into a coherent step output.",
        examples: vec![
            "summarise this round's conclusions",
            "synthesize the team's hypotheses into a next step",
        ],
        loop_contributions: vec![LoopContribution::Synthesize],
        produces: vec!["organism.runtime.round-synthesis"],
    })
}

#[must_use]
pub fn disagreement_mapper() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-disagreement-mapper",
        role: SuggestorRole::Analysis,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Disagreements],
        reads: vec![ContextKey::Hypotheses, ContextKey::Evaluations],
        domain_tags: vec!["runtime", "huddle", "disagreement"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Cluster participant positions and map where they disagree.",
        use_when: "When a huddle has multiple participants and the divergence needs to be made explicit.",
        examples: vec![
            "where do the participants disagree",
            "map the positions into clusters",
        ],
        loop_contributions: vec![LoopContribution::Observe, LoopContribution::Synthesize],
        produces: vec!["organism.runtime.disagreement-map"],
    })
}

#[must_use]
pub fn consensus_evaluator() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-consensus-evaluator",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::ConsensusOutcomes],
        reads: vec![ContextKey::Votes, ContextKey::Disagreements],
        domain_tags: vec!["runtime", "huddle", "consensus", "voting"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Evaluate votes against a ConsensusRule and emit a deterministic consensus outcome.",
        use_when: "After votes have been cast and a binding consensus verdict is needed.",
        examples: vec![
            "did the team reach consensus under quorum rule",
            "tally votes under supermajority",
        ],
        loop_contributions: vec![LoopContribution::Score, LoopContribution::Synthesize],
        produces: vec!["organism.runtime.consensus-outcome"],
    })
}
