// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Non-monotonic authority and the commit barrier.
//!
//! ## Non-Monotonic Authority
//!
//! Classical access control is monotonic — permission at time T implies
//! permission at T+1 unless explicitly revoked. In non-monotonic authority,
//! permission at T says nothing about T+1. Authority must be re-derived.
//!
//! This is required because:
//! - State mutates between reasoning and execution
//! - Concurrent agents modify shared context
//! - Time-sensitive constraints (budget windows, regulatory periods) shift
//! - Cascading plan dependencies change validity of subsequent steps
//!
//! ## The Commit Barrier
//!
//! Architecturally distinct from Admission Control:
//!
//! | Property        | Admission Control          | Commit Barrier                |
//! |----------------|---------------------------|-------------------------------|
//! | Question       | Can this intent be reasoned about? | Is execution still valid? |
//! | Evaluates at   | Intent formation time     | Mutation time                  |
//! | State used     | State at admission        | Freshly recomputed state       |
//! | Authority      | Checks initial grant      | Re-derives from current state  |
//!
//! The reasoning phase may proceed freely. The execution phase is structurally
//! incapable of proceeding unless a fresh authorization resolves correctly
//! against current state.

use serde::{Deserialize, Serialize};

/// Result of commit barrier evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitBarrierResult {
    /// Whether the commit is authorized.
    pub authorized: bool,
    /// Results for each verification dimension.
    pub verifications: Vec<Verification>,
    /// If not authorized, the reason for rejection.
    pub rejection_reason: Option<String>,
    /// Escalation target if the barrier fires.
    pub escalation: Option<EscalationTarget>,
}

/// A single verification check at the commit barrier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub dimension: CommitDimension,
    pub passed: bool,
    pub reason: String,
}

/// The dimensions checked at commit time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitDimension {
    /// Is the current state still valid for this action?
    StateValidity,
    /// Is the authority still valid (re-derived, not inherited)?
    CurrentAuthority,
    /// Are all constraints still satisfied at this moment?
    ConstraintEnvelope,
    /// Classification of action reversibility — irreversible actions
    /// deserve a stricter barrier.
    Reversibility,
}

/// Where to escalate when the commit barrier fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationTarget {
    /// Who should be notified.
    pub target: String,
    /// What form the escalation takes.
    pub form: EscalationForm,
    /// Audit context to include.
    pub audit_context: Vec<String>,
}

/// How an escalation is surfaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscalationForm {
    /// Notification only — no action required.
    Notification,
    /// Requires human review and explicit approval.
    ApprovalRequired,
    /// Requires immediate human intervention.
    Urgent,
}

/// Trait for implementing the commit barrier.
///
/// The commit barrier re-verifies authority at the exact moment of state
/// mutation. It is not a policy layer on top. It is a structural precondition
/// that the execution mechanism cannot bypass.
pub trait CommitBarrier {
    /// Check whether a commit should proceed.
    ///
    /// This must re-derive authority from current state, not inherit
    /// it from the reasoning phase.
    fn check(&self, action: &CommitAction) -> CommitBarrierResult;
}

/// An action that is about to be committed (mutate state).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitAction {
    /// What is being committed.
    pub description: String,
    /// The plan step this commit belongs to.
    pub plan_id: String,
    /// The intent this ultimately traces to.
    pub intent_id: String,
    /// Classification of reversibility.
    pub reversibility: crate::intent::Reversibility,
    /// The state keys that would be mutated.
    pub affected_state: Vec<String>,
    /// The authority scope required.
    pub required_authority: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_barrier_result_roundtrips() {
        let result = CommitBarrierResult {
            authorized: false,
            verifications: vec![
                Verification {
                    dimension: CommitDimension::StateValidity,
                    passed: true,
                    reason: "State unchanged since planning".into(),
                },
                Verification {
                    dimension: CommitDimension::CurrentAuthority,
                    passed: false,
                    reason: "Budget envelope exhausted by concurrent action".into(),
                },
            ],
            rejection_reason: Some("Authority re-derivation failed: budget exceeded".into()),
            escalation: Some(EscalationTarget {
                target: "finance@company.com".into(),
                form: EscalationForm::ApprovalRequired,
                audit_context: vec!["Budget was $10k, now $0 remaining".into()],
            }),
        };

        let json = serde_json::to_string(&result).unwrap();
        let _: CommitBarrierResult = serde_json::from_str(&json).unwrap();
    }
}
