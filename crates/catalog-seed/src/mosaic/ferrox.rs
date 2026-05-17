//! Descriptors for `converge-ferrox-solver` Suggestors.
//!
//! Authored against `converge-ferrox-solver = "0.7.1"`. Ferrox exposes
//! OR/optimization solvers (LP, MIP, CP-SAT, MinCostFlow) behind a
//! shared Suggestor surface.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![glop_lp(), highs_mip(), cpsat(), min_cost_flow()]
}

#[must_use]
pub fn glop_lp() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "ferrox-glop-lp",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["optimization", "lp", "linear-programming", "or-tools"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Linear programming via Google OR-Tools' GLOP solver.",
        use_when: "When the problem is purely continuous and linear in variables and objective.",
        examples: vec![
            "minimize cost subject to linear capacity constraints",
            "fractional resource allocation",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["ferrox.solution.lp"],
    })
}

#[must_use]
pub fn highs_mip() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "ferrox-highs-mip",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["optimization", "mip", "mixed-integer", "highs"],
        cost: CostClass::High,
        latency: LatencyClass::Batch,
        summary: "Mixed-integer programming via the HiGHS solver.",
        use_when: "When integer/binary decision variables are unavoidable (yes/no, counts).",
        examples: vec![
            "choose a subset of projects with integer constraints",
            "assign integer quantities under linear constraints",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["ferrox.solution.mip"],
    })
}

#[must_use]
pub fn cpsat() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "ferrox-cpsat",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec![
            "optimization",
            "cp-sat",
            "constraint-programming",
            "or-tools",
        ],
        cost: CostClass::High,
        latency: LatencyClass::Batch,
        summary: "Constraint Programming / SAT solver via OR-Tools CP-SAT.",
        use_when: "When the problem has heavy combinatorial structure (scheduling, packing).",
        examples: vec![
            "schedule shifts under complex coverage rules",
            "pack tasks under no-overlap and precedence constraints",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["ferrox.solution.cpsat"],
    })
}

#[must_use]
pub fn min_cost_flow() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "ferrox-min-cost-flow",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::Optimization],
        output_keys: vec![ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["optimization", "min-cost-flow", "network"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Min-cost-flow solver — specialized fast algorithm for flow problems.",
        use_when: "When the problem is naturally a flow network (units moving through arcs).",
        examples: vec![
            "route units through a capacitated network at min cost",
            "balance flow across multiple paths",
        ],
        loop_contributions: vec![LoopContribution::Optimize],
        produces: vec!["ferrox.solution.flow"],
    })
}
