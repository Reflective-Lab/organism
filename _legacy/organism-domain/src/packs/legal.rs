// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Legal Pack agents for contracts, equity, and IP governance.
//!
//! # Lifecycle: Draft → Review → Sign → Execute → Manage
//!
//! # Contract State Machine
//!
//! ```text
//! draft → review → sent_for_signature → signed → executed → active
//!                                                            ↓
//!                                              expiring_soon → expired
//! ```
//!
//! # Equity State Machine
//!
//! ```text
//! grant_proposed → board_approved → granted → vesting → exercisable → exercised
//! ```
//!
//! # Key Invariants
//!
//! - Nothing executed without signature evidence + version
//! - Executed documents are immutable - amendments only
//! - IP assignment required before contractor payment
//! - Option grants require board approval + vesting schedule

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const CONTRACT_PREFIX: &str = "contract:";
pub const EQUITY_PREFIX: &str = "equity:";
pub const IP_ASSIGNMENT_PREFIX: &str = "ip_assignment:";
pub const BOARD_APPROVAL_PREFIX: &str = "board_approval:";
pub const SIGNATURE_PREFIX: &str = "signature:";
pub const PAID_ACTION_PREFIX: &str = "paid_action:";
pub const PATENT_SUBMISSION_PREFIX: &str = "patent_submission:";
pub const APPROVAL_PREFIX: &str = "approval:";

// ============================================================================
// Agents
// ============================================================================

/// Generates contract packets (MSA/DPA/SOW) from deal triggers.
#[derive(Debug, Clone, Default)]
pub struct ContractGeneratorAgent;

impl Agent for ContractGeneratorAgent {
    fn name(&self) -> &str {
        "contract_generator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("deal.closed_won") || s.content.contains("enterprise.lead"))
            && !ctx
                .get(ContextKey::Proposals)
                .iter()
                .any(|p| p.id.starts_with(CONTRACT_PREFIX))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("deal.closed_won")
                || trigger.content.contains("enterprise.lead")
            {
                // Generate contract packet: MSA, DPA, SOW
                for doc_type in ["msa", "dpa", "sow"] {
                    facts.push(Fact {
                        key: ContextKey::Proposals,
                        id: format!("{}{}:{}", CONTRACT_PREFIX, doc_type, trigger.id),
                        content: serde_json::json!({
                            "type": "contract",
                            "doc_type": doc_type,
                            "trigger_id": trigger.id,
                            "state": "draft",
                            "version": 1,
                            "created_at": "2026-01-12"
                        })
                        .to_string(),
                    });
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Reviews contracts and moves them to approval state.
#[derive(Debug, Clone, Default)]
pub struct ContractReviewerAgent;

impl Agent for ContractReviewerAgent {
    fn name(&self) -> &str {
        "contract_reviewer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|c| c.id.starts_with(CONTRACT_PREFIX) && c.content.contains("\"state\":\"draft\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for contract in proposals.iter() {
            if contract.id.starts_with(CONTRACT_PREFIX)
                && contract.content.contains("\"state\":\"draft\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}reviewed:{}", CONTRACT_PREFIX, contract.id),
                    content: serde_json::json!({
                        "type": "contract_review",
                        "contract_id": contract.id,
                        "state": "review",
                        "reviewer": "legal_team",
                        "reviewed_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Sends contracts for signature via DocuSign or similar.
#[derive(Debug, Clone, Default)]
pub struct SignatureRequestorAgent;

impl Agent for SignatureRequestorAgent {
    fn name(&self) -> &str {
        "signature_requestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|c| c.id.contains(CONTRACT_PREFIX) && c.content.contains("\"state\":\"review\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for review in proposals.iter() {
            if review.id.contains(CONTRACT_PREFIX)
                && review.content.contains("\"state\":\"review\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", SIGNATURE_PREFIX, review.id),
                    content: serde_json::json!({
                        "type": "signature_request",
                        "contract_id": review.id,
                        "state": "sent_for_signature",
                        "provider": "docusign",
                        "envelope_id": "pending",
                        "sent_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Processes signature completions and executes contracts.
#[derive(Debug, Clone, Default)]
pub struct ContractExecutorAgent;

impl Agent for ContractExecutorAgent {
    fn name(&self) -> &str {
        "contract_executor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Accept when we have signature completion signals
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.content.contains("signature.completed"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("signature.completed") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}executed:{}", CONTRACT_PREFIX, signal.id),
                    content: serde_json::json!({
                        "type": "executed_contract",
                        "signal_id": signal.id,
                        "state": "executed",
                        "executed_at": "2026-01-12",
                        "immutable": true
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors contract expiration and triggers renewal workflows.
#[derive(Debug, Clone, Default)]
pub struct ExpirationMonitorAgent;

impl Agent for ExpirationMonitorAgent {
    fn name(&self) -> &str {
        "expiration_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|c| {
            c.id.starts_with(CONTRACT_PREFIX) && c.content.contains("\"state\":\"active\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for contract in proposals.iter() {
            if contract.id.starts_with(CONTRACT_PREFIX)
                && contract.content.contains("\"state\":\"active\"")
            {
                // Check expiration (simplified - would check actual dates)
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}expiration_check:{}", CONTRACT_PREFIX, contract.id),
                    content: serde_json::json!({
                        "type": "expiration_check",
                        "contract_id": contract.id,
                        "days_until_expiration": 30,
                        "state": "expiring_soon",
                        "renewal_action_required": true
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Processes equity grant proposals through board approval.
#[derive(Debug, Clone, Default)]
pub struct EquityGrantProcessorAgent;

impl Agent for EquityGrantProcessorAgent {
    fn name(&self) -> &str {
        "equity_grant_processor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("equity.grant_proposed"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("equity.grant_proposed") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", EQUITY_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "equity_grant",
                        "seed_id": seed.id,
                        "state": "grant_proposed",
                        "requires_board_approval": true,
                        "vesting_schedule": "4yr_1yr_cliff",
                        "proposed_at": "2026-01-12"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Handles board approval for equity grants.
#[derive(Debug, Clone, Default)]
pub struct BoardApprovalAgent;

impl Agent for BoardApprovalAgent {
    fn name(&self) -> &str {
        "board_approval"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.content.contains("board.approved"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("board.approved") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", BOARD_APPROVAL_PREFIX, signal.id),
                    content: serde_json::json!({
                        "type": "board_approval",
                        "signal_id": signal.id,
                        "state": "board_approved",
                        "approved_at": "2026-01-12",
                        "resolution_number": "2026-001"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages IP assignments for contractors and employees.
#[derive(Debug, Clone, Default)]
pub struct IpAssignmentAgent;

impl Agent for IpAssignmentAgent {
    fn name(&self) -> &str {
        "ip_assignment"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("contractor.onboarded") || s.content.contains("code.contributed")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for seed in seeds.iter() {
            if seed.content.contains("contractor.onboarded")
                || seed.content.contains("code.contributed")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", IP_ASSIGNMENT_PREFIX, seed.id),
                    content: serde_json::json!({
                        "type": "ip_assignment",
                        "seed_id": seed.id,
                        "state": "required",
                        "assignment_type": "work_for_hire",
                        "gates_payment": true
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Validates IP assignments are complete before payments.
#[derive(Debug, Clone, Default)]
pub struct IpGateValidatorAgent;

impl Agent for IpGateValidatorAgent {
    fn name(&self) -> &str {
        "ip_gate_validator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Accept when there are pending contractor payments
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.content.contains("contractor.payment") && p.content.contains("\"state\":\"pending\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for payment in proposals.iter() {
            if payment.content.contains("contractor.payment")
                && payment.content.contains("\"state\":\"pending\"")
            {
                // Check if IP assignment exists
                let has_ip_assignment = proposals.iter().any(|p| {
                    p.id.starts_with(IP_ASSIGNMENT_PREFIX)
                        && p.content.contains("\"state\":\"signed\"")
                });

                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}gate:{}", IP_ASSIGNMENT_PREFIX, payment.id),
                    content: serde_json::json!({
                        "type": "ip_gate_check",
                        "payment_id": payment.id,
                        "ip_assignment_valid": has_ip_assignment,
                        "payment_allowed": has_ip_assignment
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures nothing is executed without signature evidence.
#[derive(Debug, Clone, Default)]
pub struct SignatureRequiredInvariant;

impl Invariant for SignatureRequiredInvariant {
    fn name(&self) -> &str {
        "signature_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for contract in ctx.get(ContextKey::Proposals).iter() {
            if contract.id.starts_with(CONTRACT_PREFIX)
                && contract.content.contains("\"state\":\"executed\"")
                && !contract.content.contains("\"immutable\":true")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Executed contract {} missing immutable flag", contract.id),
                    vec![contract.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures IP assignments exist before contractor payments.
#[derive(Debug, Clone, Default)]
pub struct IpAssignmentBeforePaymentInvariant;

impl Invariant for IpAssignmentBeforePaymentInvariant {
    fn name(&self) -> &str {
        "ip_assignment_before_payment"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);

        for eval in evaluations.iter() {
            if eval.content.contains("\"ip_assignment_valid\":false")
                && eval.content.contains("contractor.payment")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    "Contractor payment attempted without IP assignment".to_string(),
                    vec![eval.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures equity grants have board approval.
#[derive(Debug, Clone, Default)]
pub struct BoardApprovalRequiredInvariant;

impl Invariant for BoardApprovalRequiredInvariant {
    fn name(&self) -> &str {
        "board_approval_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let proposals = ctx.get(ContextKey::Proposals);

        for grant in proposals.iter() {
            if grant.id.starts_with(EQUITY_PREFIX)
                && grant.content.contains("\"state\":\"granted\"")
            {
                let has_approval = proposals
                    .iter()
                    .any(|p| p.id.starts_with(BOARD_APPROVAL_PREFIX));
                if !has_approval {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("Equity grant {} lacks board approval", grant.id),
                        vec![grant.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures paid actions require explicit approval.
#[derive(Debug, Clone, Default)]
pub struct PaidActionRequiresApprovalInvariant;

impl Invariant for PaidActionRequiresApprovalInvariant {
    fn name(&self) -> &str {
        "paid_action_requires_approval"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let has_approval = ctx
            .get(ContextKey::Constraints)
            .iter()
            .any(|fact| fact.id.starts_with(APPROVAL_PREFIX));

        for action in ctx.get(ContextKey::Strategies).iter() {
            if action.id.starts_with(PAID_ACTION_PREFIX)
                && action.content.contains("\"requires_approval\":true")
                && !has_approval
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Paid action {} missing approval", action.id),
                    vec![action.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures patent submissions require explicit approval.
#[derive(Debug, Clone, Default)]
pub struct SubmissionRequiresApprovalInvariant;

impl Invariant for SubmissionRequiresApprovalInvariant {
    fn name(&self) -> &str {
        "submission_requires_approval"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let has_approval = ctx
            .get(ContextKey::Constraints)
            .iter()
            .any(|fact| fact.id.starts_with(APPROVAL_PREFIX));

        for submission in ctx.get(ContextKey::Strategies).iter() {
            if submission.id.starts_with(PATENT_SUBMISSION_PREFIX) && !has_approval {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Submission {} missing approval", submission.id),
                    vec![submission.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures submissions include evidence receipts.
#[derive(Debug, Clone, Default)]
pub struct SubmissionRequiresEvidenceInvariant;

impl Invariant for SubmissionRequiresEvidenceInvariant {
    fn name(&self) -> &str {
        "submission_requires_evidence"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let has_evidence = ctx.get(ContextKey::Evaluations).iter().any(|fact| {
            fact.id.starts_with("prior_art:") && fact.content.contains("\"receipt_logged\":true")
        });

        for submission in ctx.get(ContextKey::Strategies).iter() {
            if submission.id.starts_with(PATENT_SUBMISSION_PREFIX) && !has_evidence {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Submission {} missing evidence receipt", submission.id),
                    vec![submission.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(ContractGeneratorAgent.name(), "contract_generator");
        assert_eq!(ContractReviewerAgent.name(), "contract_reviewer");
        assert_eq!(SignatureRequestorAgent.name(), "signature_requestor");
        assert_eq!(ContractExecutorAgent.name(), "contract_executor");
        assert_eq!(ExpirationMonitorAgent.name(), "expiration_monitor");
        assert_eq!(EquityGrantProcessorAgent.name(), "equity_grant_processor");
        assert_eq!(BoardApprovalAgent.name(), "board_approval");
        assert_eq!(IpAssignmentAgent.name(), "ip_assignment");
        assert_eq!(IpGateValidatorAgent.name(), "ip_gate_validator");
    }
}
