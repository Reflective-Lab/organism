//! Descriptors for `converge-arbiter-policy` Suggestors.
//!
//! Authored against `converge-arbiter-policy = "2.0.1"`. Arbiter gates
//! enforce policy invariants on proposals (budget, rate limits, approval
//! flow, data classification, regulatory compliance, Cedar analysis).

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

/// Returns every arbiter-family descriptor in this seed.
#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        budget_gate(),
        rate_limit_gate(),
        approval_gate(),
        data_classification_gate(),
        compliance_gate(),
        cedar_analysis_gate(),
    ]
}

/// Gate proposals against a declared budget envelope. Rejects proposals
/// that would push cumulative spend over the configured cap.
#[must_use]
pub fn budget_gate() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "arbiter-budget-gate",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![ContextKey::Constraints],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["policy", "budget", "cost-control"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Gate proposals against a declared budget envelope.",
        use_when: "When a proposal must not push cumulative spend past a configured cap.",
        examples: vec![
            "does this exceed our budget",
            "block if the rolling spend would exceed $50k this quarter",
            "enforce per-project budget caps",
        ],
        loop_contributions: vec![LoopContribution::Authorize],
        produces: vec!["arbiter.policy.budget-decision"],
    })
}

/// Throttle proposals according to a rate-limit policy (per actor, per
/// tenant, per minute, etc.).
#[must_use]
pub fn rate_limit_gate() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "arbiter-rate-limit-gate",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![ContextKey::Constraints],
        reads: vec![ContextKey::Proposals],
        domain_tags: vec!["policy", "rate-limit", "throttle"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Throttle proposals according to a rate-limit policy.",
        use_when: "When per-actor or per-tenant request frequency must be capped.",
        examples: vec![
            "limit to 10 requests per minute per user",
            "rate-limit outbound API calls",
            "throttle bulk submissions",
        ],
        loop_contributions: vec![LoopContribution::Authorize],
        produces: vec!["arbiter.policy.rate-limit-decision"],
    })
}

/// Route proposals into the configured approval flow (single-approver,
/// quorum, etc.). Emits an approval-required marker that downstream HITL
/// consumes.
#[must_use]
pub fn approval_gate() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "arbiter-approval-gate",
        role: SuggestorRole::Constraint,
        capabilities: vec![
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::HumanInTheLoop,
        ],
        output_keys: vec![ContextKey::Constraints],
        reads: vec![ContextKey::Proposals],
        domain_tags: vec!["policy", "approval", "hitl"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Route proposals into the approval flow (single-approver, quorum).",
        use_when: "When proposals above a risk threshold require human sign-off.",
        examples: vec![
            "require approval before promoting this decision",
            "send to manager for sign-off",
            "block until two reviewers approve",
        ],
        loop_contributions: vec![LoopContribution::Authorize],
        produces: vec!["arbiter.policy.approval-decision"],
    })
}

/// Classify proposals against the configured data-classification policy
/// (public/internal/confidential/restricted/PII).
#[must_use]
pub fn data_classification_gate() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "arbiter-data-classification-gate",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![ContextKey::Constraints],
        reads: vec![ContextKey::Proposals, ContextKey::Signals],
        domain_tags: vec!["policy", "classification", "pii", "data-handling"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Classify proposals against the data-classification policy.",
        use_when: "When proposals may carry PII or restricted data needing tagging.",
        examples: vec![
            "tag this fact as confidential",
            "detect PII before it lands in audit logs",
            "block PII from being sent to external providers",
        ],
        loop_contributions: vec![LoopContribution::Authorize, LoopContribution::Validate],
        produces: vec!["arbiter.policy.classification-decision"],
    })
}

/// Apply the configured regulatory compliance policy (SOC2, HIPAA,
/// GDPR, etc.). Emits a compliance verdict on the proposed action.
#[must_use]
pub fn compliance_gate() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "arbiter-compliance-gate",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![ContextKey::Constraints],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["policy", "compliance", "regulatory"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Apply regulatory compliance policy (SOC2, HIPAA, GDPR, etc.).",
        use_when: "When proposals must pass a regulatory framework before promotion.",
        examples: vec![
            "verify this passes HIPAA",
            "block if it violates GDPR data-subject rights",
            "tag with SOC2 evidence",
        ],
        loop_contributions: vec![LoopContribution::Authorize, LoopContribution::Validate],
        produces: vec!["arbiter.policy.compliance-decision"],
    })
}

/// Static Cedar policy analysis. Evaluates a proposal against the active
/// Cedar policy set; emits a structured permit/forbid with the matched
/// policy IDs.
#[must_use]
pub fn cedar_analysis_gate() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "arbiter-cedar-analysis-gate",
        role: SuggestorRole::Constraint,
        capabilities: vec![SuggestorCapability::PolicyEnforcement],
        output_keys: vec![ContextKey::Constraints],
        reads: vec![ContextKey::Proposals, ContextKey::Constraints],
        domain_tags: vec!["policy", "cedar", "access-control"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Static Cedar policy analysis with permit/forbid decision.",
        use_when: "When fine-grained Cedar access-control policies must be evaluated.",
        examples: vec![
            "is this principal allowed to perform this action",
            "evaluate against the production Cedar policy set",
            "explain why this would be forbidden",
        ],
        loop_contributions: vec![LoopContribution::Authorize, LoopContribution::Validate],
        produces: vec!["arbiter.policy.cedar-decision"],
    })
}
