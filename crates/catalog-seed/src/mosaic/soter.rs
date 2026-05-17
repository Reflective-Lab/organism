//! Descriptors for `converge-soter-smt` Suggestors.
//!
//! Authored against `converge-soter-smt = "0.2.2"`. Soter wraps SMT
//! solvers (Z3, CVC5) behind the Suggestor surface.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![smt_solver()]
}

#[must_use]
pub fn smt_solver() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "soter-smt-solver",
        role: SuggestorRole::Constraint,
        capabilities: vec![
            SuggestorCapability::Optimization,
            SuggestorCapability::PolicyEnforcement,
        ],
        output_keys: vec![ContextKey::Constraints, ContextKey::Strategies],
        reads: vec![ContextKey::Constraints, ContextKey::Hypotheses],
        domain_tags: vec!["smt", "z3", "cvc5", "satisfiability", "verification"],
        cost: CostClass::High,
        latency: LatencyClass::Batch,
        summary: "Decide satisfiability of a constraint set via an SMT backend (Z3 / CVC5).",
        use_when: "When constraints are non-trivially logical and a SAT/SMT decision is the right hammer.",
        examples: vec![
            "is this configuration satisfiable",
            "find a witness that makes these constraints hold",
            "prove no policy violation is reachable",
        ],
        loop_contributions: vec![LoopContribution::Validate, LoopContribution::Optimize],
        produces: vec!["soter.smt.decision"],
    })
}
