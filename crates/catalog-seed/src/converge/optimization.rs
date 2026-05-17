//! Descriptors for `converge-optimization::suggestors::*`.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        formation_assembly(),
        assignment(),
        flow_optimization(),
        portfolio(),
        work_schedule(),
        greedy_scheduler(),
        nearest_neighbor_routing(),
    ]
}

#[must_use]
pub fn formation_assembly() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-formation-assembly",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["formation", "assembly", "optimization"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Assemble a formation roster that satisfies role and capability requirements.",
        use_when: "When a formation template's roles need to be filled from a candidate pool under constraints.",
        examples: vec![
            "build a roster covering analysis + planning + synthesis",
            "assemble the smallest team that satisfies these capabilities",
        ],
        loop_contributions: vec![LoopContribution::Optimize, LoopContribution::Synthesize],
        produces: vec!["converge.optimization.formation-assembly"],
    })
}

#[must_use]
pub fn assignment() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-assignment",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["assignment", "matching", "optimization"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Solve assignment / matching problems (Hungarian algorithm).",
        use_when: "When N agents must be paired with N tasks to minimize total cost.",
        examples: vec![
            "assign engineers to tickets minimizing context-switch cost",
            "match drivers to deliveries to minimize travel time",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["converge.optimization.assignment"],
    })
}

#[must_use]
pub fn flow_optimization() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-flow-optimization",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["flow", "min-cost-flow", "network", "optimization"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Solve min-cost-flow problems over a network with capacities.",
        use_when: "When goods/work/messages must move through a capacitated network at min cost.",
        examples: vec![
            "route units from warehouses to stores under truck-capacity limits",
            "schedule message flow across a bandwidth-constrained network",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["converge.optimization.flow"],
    })
}

#[must_use]
pub fn portfolio() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-portfolio",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["portfolio", "selection", "knapsack", "optimization"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Select a portfolio of items maximizing value within a budget.",
        use_when: "When you must pick a subset of candidates under a single shared budget cap.",
        examples: vec![
            "pick the best mix of projects under a $10M budget",
            "choose features to ship under a fixed engineering quarter",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["converge.optimization.portfolio"],
    })
}

#[must_use]
pub fn work_schedule() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-work-schedule",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["scheduling", "shift", "workforce", "optimization"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Schedule a workforce across shifts honoring coverage rules and preferences.",
        use_when: "When you must assign workers to time slots under coverage and labor constraints.",
        examples: vec![
            "build next month's nurse rota",
            "schedule support engineers across timezones",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["converge.optimization.work-schedule"],
    })
}

#[must_use]
pub fn greedy_scheduler() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-greedy-scheduler",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["scheduling", "greedy", "heuristic", "fast"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Fast greedy task scheduler — cheap heuristic for non-optimal-but-fine baseline.",
        use_when: "When a quick reasonable schedule beats a slow optimal one.",
        examples: vec![
            "give me a rough schedule in milliseconds",
            "baseline schedule before running the LP solver",
        ],
        loop_contributions: vec![LoopContribution::Optimize, LoopContribution::Propose],
        produces: vec!["converge.optimization.greedy-schedule"],
    })
}

#[must_use]
pub fn nearest_neighbor_routing() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "converge-nn-time-window-routing",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["routing", "vrp", "time-window", "nearest-neighbor"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Nearest-neighbor VRP solver with time windows. Heuristic, not optimal.",
        use_when: "When you need a fast routable plan for vehicles with delivery time windows.",
        examples: vec![
            "route this fleet through deliveries respecting opening hours",
            "fast VRPTW baseline before a tighter solver",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["converge.optimization.routing.nn-time-window"],
    })
}
