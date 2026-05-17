//! Descriptors for `organism-adversarial::*` agents.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        assumption_breaker(),
        constraint_checker(),
        economic_skeptic(),
        operational_skeptic(),
        anomaly_skeptic(),
    ]
}

#[must_use]
pub fn assumption_breaker() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-assumption-breaker",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Diagnostic, ContextKey::Disagreements],
        reads: vec![ContextKey::Hypotheses, ContextKey::Proposals],
        domain_tags: vec!["adversarial", "challenge", "assumptions"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Surface hidden assumptions in a proposal and challenge each one.",
        use_when: "When a proposal looks confident but rests on undeclared premises.",
        examples: vec![
            "what are we assuming about market demand here",
            "challenge the implicit assumption that customers want this",
        ],
        loop_contributions: vec![LoopContribution::Challenge],
        produces: vec!["organism.adversarial.assumption-challenge"],
    })
}

#[must_use]
pub fn constraint_checker() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-constraint-checker",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![ContextKey::Diagnostic, ContextKey::Constraints],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["adversarial", "constraints", "validation"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Check a proposal against declared constraints and flag violations.",
        use_when: "When a proposal must be validated against hard limits before promotion.",
        examples: vec![
            "does this respect our team-size cap",
            "flag if this violates the data-locality constraint",
        ],
        loop_contributions: vec![LoopContribution::Validate, LoopContribution::Challenge],
        produces: vec!["organism.adversarial.constraint-violation"],
    })
}

#[must_use]
pub fn economic_skeptic() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-economic-skeptic",
        role: SuggestorRole::Constraint,
        capabilities: vec![
            SuggestorCapability::LlmReasoning,
            SuggestorCapability::Analytics,
        ],
        output_keys: vec![ContextKey::Diagnostic, ContextKey::Disagreements],
        reads: vec![ContextKey::Proposals, ContextKey::Evaluations],
        domain_tags: vec!["adversarial", "economics", "cost", "roi"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Challenge a proposal on cost, ROI, and economic feasibility.",
        use_when: "When a proposal needs scrutiny on whether the economics actually work.",
        examples: vec![
            "does this pay back within three years",
            "challenge the revenue assumptions",
        ],
        loop_contributions: vec![LoopContribution::Challenge],
        produces: vec!["organism.adversarial.economic-challenge"],
    })
}

#[must_use]
pub fn operational_skeptic() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-operational-skeptic",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Diagnostic, ContextKey::Disagreements],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["adversarial", "operations", "execution"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Challenge a proposal on operational feasibility (capacity, dependencies, timelines).",
        use_when: "When a proposal looks good on paper but execution risk is real.",
        examples: vec![
            "do we actually have the people to ship this",
            "challenge the timeline given our other commitments",
        ],
        loop_contributions: vec![LoopContribution::Challenge],
        produces: vec!["organism.adversarial.operational-challenge"],
    })
}

#[must_use]
pub fn anomaly_skeptic() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-anomaly-skeptic",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Diagnostic, ContextKey::Disagreements],
        reads: vec![ContextKey::Signals, ContextKey::Proposals],
        domain_tags: vec![
            "adversarial",
            "anomaly",
            "outlier-detection",
            "prism-backed",
        ],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Detect statistical anomalies in proposals or signals and raise them as challenges.",
        use_when: "When a proposal's numbers or supporting signals look statistically off.",
        examples: vec![
            "is this conversion rate an outlier vs history",
            "flag if this proposal differs sharply from cohort norms",
        ],
        loop_contributions: vec![LoopContribution::Challenge, LoopContribution::Observe],
        produces: vec!["organism.adversarial.anomaly-challenge"],
    })
}
