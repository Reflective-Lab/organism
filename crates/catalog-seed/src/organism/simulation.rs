//! Descriptors for `organism-simulation::*` agents.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        policy_simulation(),
        outcome_simulation(),
        operational_simulation(),
        causal_simulation(),
        cost_simulation(),
    ]
}

#[must_use]
pub fn policy_simulation() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-policy-simulation",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["simulation", "policy", "what-if"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Simulate the effect of a proposed policy before committing.",
        use_when: "When a proposal changes policy and you want to see consequences first.",
        examples: vec![
            "what happens if we cap discounts at 10%",
            "simulate the impact of new admission rules",
        ],
        loop_contributions: vec![LoopContribution::Score, LoopContribution::Observe],
        produces: vec!["organism.simulation.policy-effect"],
    })
}

#[must_use]
pub fn outcome_simulation() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-outcome-simulation",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Proposals, ContextKey::Signals],
        domain_tags: vec!["simulation", "outcome", "forecast"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Forecast likely outcomes of a proposal given current signals.",
        use_when: "When you need a probabilistic outcome forecast for a candidate decision.",
        examples: vec![
            "what is the expected outcome of this plan",
            "probabilistic forecast over 12 months",
        ],
        loop_contributions: vec![LoopContribution::Score],
        produces: vec!["organism.simulation.outcome-forecast"],
    })
}

#[must_use]
pub fn operational_simulation() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-operational-simulation",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["simulation", "operations", "capacity"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Simulate operational impact (capacity, throughput, queueing) of a proposal.",
        use_when: "When a proposal changes operational load and you need to see if the system holds.",
        examples: vec![
            "can the team absorb this work given current load",
            "what happens to queue depth if we accept this contract",
        ],
        loop_contributions: vec![LoopContribution::Score, LoopContribution::Observe],
        produces: vec!["organism.simulation.operational-impact"],
    })
}

#[must_use]
pub fn causal_simulation() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-causal-simulation",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Proposals, ContextKey::Hypotheses],
        domain_tags: vec!["simulation", "causal", "counterfactual"],
        cost: CostClass::High,
        latency: LatencyClass::Batch,
        summary: "Counterfactual / causal simulation over a structural model.",
        use_when: "When 'what if we had not done X' or 'what causes Y' must be answered rigorously.",
        examples: vec![
            "would revenue have grown without the campaign",
            "trace which input drove this outcome",
        ],
        loop_contributions: vec![LoopContribution::Score, LoopContribution::Challenge],
        produces: vec!["organism.simulation.causal"],
    })
}

#[must_use]
pub fn cost_simulation() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-cost-simulation",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Proposals],
        domain_tags: vec!["simulation", "cost", "tco"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Total-cost-of-ownership simulation for a proposed decision.",
        use_when: "When a proposal has cost implications beyond the headline price.",
        examples: vec![
            "what is the 3-year TCO of this vendor",
            "include hidden costs in the estimate",
        ],
        loop_contributions: vec![LoopContribution::Score],
        produces: vec!["organism.simulation.cost"],
    })
}
