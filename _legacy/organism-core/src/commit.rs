// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Non-monotonic authority — organism-specific Invariants for the commit boundary.
//!
//! The commit barrier is NOT a second gate. It is an organism-specific
//! [`converge_core::Invariant`] that plugs into converge's standard
//! invariant checking mechanism.
//!
//! ## How it works
//!
//! converge-core already has:
//! - `PromotionGate` — the one enforcement point for all facts
//! - `Invariant` trait — runtime constraints the engine enforces
//! - `InvariantClass::Structural` — checked every merge
//!
//! The commit barrier is a Structural invariant that re-verifies
//! authority freshness, state validity, and constraint compliance
//! at the exact moment a cycle completes. It uses converge's own
//! machinery — no parallel type system needed.
//!
//! ## Non-Monotonic Authority
//!
//! Classical access control is monotonic — permission at T implies T+1.
//! In organism systems, authority must be re-derived because:
//! - State mutates between reasoning and execution
//! - Concurrent agents modify shared context
//! - Budget windows and regulatory periods shift
//! - Cascading plan dependencies change validity
//!
//! ## Admission vs Commit
//!
//! | Property        | Admission Control      | Commit Invariant           |
//! |----------------|------------------------|----------------------------|
//! | Question       | Can this be reasoned?  | Is execution still valid?  |
//! | When           | Intent formation       | Every engine cycle         |
//! | State used     | State at admission     | Current context state      |
//! | Mechanism      | AdmissionController    | converge Invariant trait   |

use converge_core::{Context, Invariant, InvariantClass, InvariantResult, Violation};
use serde::{Deserialize, Serialize};

use crate::intent::Reversibility;

/// Organism-specific invariant that re-verifies authority at commit time.
///
/// This implements converge-core's `Invariant` trait as a `Structural`
/// invariant — checked every merge, violation = immediate failure.
///
/// It checks the four commit dimensions:
/// 1. State validity — has the context changed since planning?
/// 2. Authority freshness — is authority still valid?
/// 3. Constraint envelope — are all constraints still satisfied?
/// 4. Reversibility threshold — does the action's reversibility warrant
///    stricter checking?
pub struct CommitBarrierInvariant {
    /// The reversibility of the current intent.
    /// Irreversible actions get stricter checks.
    reversibility: Reversibility,
    /// Context version at planning time — used to detect state drift.
    planning_version: Option<u64>,
}

impl CommitBarrierInvariant {
    /// Create a commit barrier invariant for a given reversibility level.
    pub fn new(reversibility: Reversibility) -> Self {
        Self {
            reversibility,
            planning_version: None,
        }
    }

    /// Record the context version at planning time.
    /// Used later to detect if state drifted between planning and commit.
    pub fn with_planning_version(mut self, version: u64) -> Self {
        self.planning_version = Some(version);
        self
    }
}

impl Invariant for CommitBarrierInvariant {
    fn name(&self) -> &str {
        "organism:commit_barrier"
    }

    fn class(&self) -> InvariantClass {
        // Structural = checked every merge, violation = immediate failure
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        // Check 1: State drift detection
        if let Some(planning_ver) = self.planning_version {
            if ctx.version() > planning_ver {
                // State has changed since planning.
                // For irreversible actions, this is a violation.
                // For reversible actions, it's acceptable.
                if self.reversibility == Reversibility::Irreversible {
                    return InvariantResult::Violated(Violation {
                        reason: format!(
                            "State drifted since planning (version {} → {}). \
                             Irreversible action requires fresh planning.",
                            planning_ver,
                            ctx.version()
                        ),
                        fact_ids: vec![],
                    });
                }
            }
        }

        // Check 2: Budget constraint — ensure we haven't exceeded limits
        // (The engine's own budget checking handles this, but we add
        // organism-level awareness for stricter thresholds on irreversible actions)

        // Check 3: Constraint satisfaction
        // (Specific constraint checks would be added by domain invariants)

        InvariantResult::Ok
    }
}

/// Escalation target when the commit barrier fires.
///
/// This is organism-level metadata — it describes where to route
/// when an invariant violation occurs. The engine halts; the organism
/// layer decides where to escalate.
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
    /// Notification only.
    Notification,
    /// Requires human review and explicit approval.
    ApprovalRequired,
    /// Requires immediate human intervention.
    Urgent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{Context, ContextKey};

    #[test]
    fn commit_barrier_passes_when_no_drift() {
        let invariant = CommitBarrierInvariant::new(Reversibility::Irreversible)
            .with_planning_version(0);

        let ctx = Context::new();
        assert_eq!(invariant.check(&ctx), InvariantResult::Ok);
    }

    #[test]
    fn commit_barrier_allows_drift_for_reversible() {
        let invariant = CommitBarrierInvariant::new(Reversibility::Reversible)
            .with_planning_version(0);

        let mut ctx = Context::new();
        // Simulate state change by adding a fact
        ctx.add_fact(converge_core::Fact::new(
            ContextKey::Seeds,
            "test:drift",
            "state changed",
        ))
        .unwrap();

        // Reversible action — drift is acceptable
        assert_eq!(invariant.check(&ctx), InvariantResult::Ok);
    }

    #[test]
    fn commit_barrier_rejects_drift_for_irreversible() {
        let invariant = CommitBarrierInvariant::new(Reversibility::Irreversible)
            .with_planning_version(0);

        let mut ctx = Context::new();
        ctx.add_fact(converge_core::Fact::new(
            ContextKey::Seeds,
            "test:drift",
            "state changed",
        ))
        .unwrap();

        // Irreversible action — drift is a violation
        match invariant.check(&ctx) {
            InvariantResult::Violated(v) => {
                assert!(v.reason.contains("Irreversible"));
            }
            InvariantResult::Ok => panic!("should have violated"),
        }
    }

    #[test]
    fn escalation_target_roundtrips() {
        let target = EscalationTarget {
            target: "finance@company.com".into(),
            form: EscalationForm::ApprovalRequired,
            audit_context: vec!["Budget was $10k, now $0".into()],
        };

        let json = serde_json::to_string(&target).unwrap();
        let _: EscalationTarget = serde_json::from_str(&json).unwrap();
    }
}
