// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Organism intent envelope — extends converge-core's RootIntent.
//!
//! The [`OrganismIntent`] wraps a [`converge_core::RootIntent`] with
//! organism-specific fields that the kernel doesn't need to know about:
//!
//! - **Reversibility**: Is this action reversible, partial, or irreversible?
//!   Irreversible actions deserve stricter commit barriers.
//! - **Expiry**: When does authority expire? What happens at deadline?
//! - **Forbidden actions**: Explicit blacklist (not just constraint absence).
//! - **Admission control**: Should this intent enter the system at all?
//!
//! The kernel's RootIntent carries the constitutional properties (objective,
//! constraints, budgets, success criteria). The organism envelope carries
//! the organizational properties that affect planning and governance.

use converge_core::{
    IntentId, IntentKind, RootIntent,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Organism Intent Envelope
// ---------------------------------------------------------------------------

/// An organism-level intent that wraps a converge-core RootIntent.
///
/// The kernel sees only the `RootIntent`. The organism layer uses the
/// full envelope for planning, admission control, and commit barrier
/// decisions.
#[derive(Debug, Clone)]
pub struct OrganismIntent {
    /// The converge-core RootIntent — the constitutional contract.
    inner: RootIntent,

    /// Classification of action reversibility.
    /// Irreversible actions trigger stricter commit barriers.
    pub reversibility: Reversibility,

    /// When this intent's authority expires.
    pub expires: Option<Expiry>,

    /// Explicitly forbidden actions (beyond what constraints exclude).
    pub forbidden: Vec<ForbiddenAction>,
}

impl OrganismIntent {
    /// Create a new organism intent wrapping a RootIntent.
    pub fn new(root: RootIntent) -> Self {
        Self {
            inner: root,
            reversibility: Reversibility::Reversible,
            expires: None,
            forbidden: Vec::new(),
        }
    }

    /// Set reversibility classification.
    pub fn with_reversibility(mut self, rev: Reversibility) -> Self {
        self.reversibility = rev;
        self
    }

    /// Set authority expiry.
    pub fn with_expiry(mut self, expiry: Expiry) -> Self {
        self.expires = Some(expiry);
        self
    }

    /// Add a forbidden action.
    pub fn with_forbidden(mut self, action: ForbiddenAction) -> Self {
        self.forbidden.push(action);
        self
    }

    /// Access the underlying RootIntent (what the kernel sees).
    pub fn root_intent(&self) -> &RootIntent {
        &self.inner
    }

    /// Consume and return the RootIntent for engine execution.
    pub fn into_root_intent(self) -> RootIntent {
        self.inner
    }

    /// Convenience: the intent ID from the kernel.
    pub fn id(&self) -> &IntentId {
        &self.inner.id
    }

    /// Convenience: the intent kind from the kernel.
    pub fn kind(&self) -> &IntentKind {
        &self.inner.kind
    }
}

// ---------------------------------------------------------------------------
// Organism-specific envelope types
// ---------------------------------------------------------------------------

/// Classification of action reversibility.
///
/// This feeds directly into the commit barrier — an irreversible action
/// deserves stricter re-verification than a reversible one. This is how
/// safety-critical systems actually work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reversibility {
    /// Action can be fully undone.
    Reversible,
    /// Action can be partially undone.
    Partial,
    /// Action cannot be undone — demands stricter commit barrier.
    Irreversible,
}

/// When authority expires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expiry {
    /// ISO 8601 timestamp or duration.
    pub deadline: String,
    /// What happens when the deadline passes.
    pub on_expiry: ExpiryAction,
}

/// What happens when authority expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpiryAction {
    /// Halt all execution.
    Halt,
    /// Escalate to human authority.
    Escalate,
    /// Complete current step, then halt.
    CompleteAndHalt,
}

/// An explicitly forbidden action.
///
/// Distinct from constraints: constraints say "stay within these bounds",
/// forbidden actions say "never do this specific thing".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenAction {
    pub description: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Admission Control
// ---------------------------------------------------------------------------

/// Result of admission control evaluation.
///
/// Admission control answers: "Can this intent be reasoned about?"
/// This is distinct from the commit barrier which answers:
/// "Is execution still valid at this moment?"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionResult {
    /// Whether the intent is admitted for planning.
    pub admitted: bool,
    /// Feasibility assessment per dimension.
    pub dimensions: Vec<FeasibilityDimension>,
    /// Blocking issues that prevent admission.
    pub blockers: Vec<String>,
}

/// A dimension of intent feasibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityDimension {
    pub kind: FeasibilityKind,
    pub feasible: bool,
    pub reason: String,
}

/// The four admission control dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeasibilityKind {
    /// Does the system have the required tools and models?
    Capability,
    /// Is there enough information to reason about this intent?
    Context,
    /// Are compute, budget, time, and access within envelope?
    Resources,
    /// Is the system permitted to act on this intent?
    Authority,
}

/// Trait for implementing admission control.
///
/// Admission control is Kubernetes scheduling for organizational work.
/// It decides: Run, Queue, Reject, or Refine intent.
pub trait AdmissionController: Send + Sync {
    /// Evaluate whether an intent should be admitted for planning.
    fn evaluate(&self, intent: &OrganismIntent) -> AdmissionResult;
}

// ---------------------------------------------------------------------------
// Intent Decomposition
// ---------------------------------------------------------------------------

/// A node in an intent decomposition tree.
///
/// Most intents are too large to execute directly. The system decomposes
/// them into intent trees where each node has its own bounded envelope.
/// Authority can only narrow during decomposition, never expand.
#[derive(Debug, Clone)]
pub struct IntentNode {
    /// The organism intent at this node.
    pub intent: OrganismIntent,
    /// Child intents (decomposed sub-goals).
    pub children: Vec<IntentNode>,
    /// Parent intent ID (None for root).
    pub parent_id: Option<String>,
}

impl IntentNode {
    /// Create a root intent node.
    pub fn root(intent: OrganismIntent) -> Self {
        Self {
            intent,
            children: Vec::new(),
            parent_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{
        Budgets, ConstraintSeverity, IntentConstraint, Objective, RootIntent, Scope,
        SuccessCriteria,
    };

    fn test_root_intent() -> RootIntent {
        RootIntent {
            id: IntentId::new("test-001"),
            kind: IntentKind::Custom,
            objective: Some(Objective::Custom("Increase Q2 revenue by 15%".into())),
            scope: Scope::default(),
            constraints: vec![IntentConstraint {
                key: "budget".into(),
                value: "10000 USD".into(),
                severity: ConstraintSeverity::Hard,
            }],
            success_criteria: SuccessCriteria::default(),
            budgets: Budgets::default(),
        }
    }

    #[test]
    fn organism_intent_wraps_root_intent() {
        let root = test_root_intent();
        let organism = OrganismIntent::new(root.clone())
            .with_reversibility(Reversibility::Partial)
            .with_forbidden(ForbiddenAction {
                description: "No cold outreach to existing customers".into(),
                reason: "Account management policy".into(),
            });

        assert_eq!(organism.id(), &root.id);
        assert_eq!(organism.reversibility, Reversibility::Partial);
        assert_eq!(organism.forbidden.len(), 1);
    }

    #[test]
    fn kernel_sees_only_root_intent() {
        let organism = OrganismIntent::new(test_root_intent())
            .with_reversibility(Reversibility::Irreversible)
            .with_expiry(Expiry {
                deadline: "2026-04-01T00:00:00Z".into(),
                on_expiry: ExpiryAction::Halt,
            });

        // The kernel only sees the RootIntent — no reversibility, no expiry
        let root = organism.into_root_intent();
        assert_eq!(root.id, IntentId::new("test-001"));
    }

}
