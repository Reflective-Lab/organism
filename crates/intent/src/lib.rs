//! Intent system.
//!
//! Translates human goals into structured, machine-executable specifications.
//!
//! - [`IntentPacket`] — typed contract between humans and the runtime
//! - [`admission`] — feasibility gate before any planning begins
//! - [`decomposition`] — breaks intents into governed intent trees

pub mod admission;
pub mod decomposition;
pub mod resolution;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Intent Packet ──────────────────────────────────────────────────

/// The contract between humans and the runtime.
///
/// Authority is *not* granted by the existence of an IntentPacket — it is
/// recomputed at the Converge commit boundary. The `authority` field is a
/// declaration of what the system is *permitted* to attempt, not proof that
/// it is allowed to commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPacket {
    pub id: Uuid,
    pub outcome: String,
    pub context: serde_json::Value,
    pub constraints: Vec<String>,
    pub authority: Vec<String>,
    pub forbidden: Vec<ForbiddenAction>,
    pub reversibility: Reversibility,
    pub expires: DateTime<Utc>,
    pub expiry_action: ExpiryAction,
}

impl IntentPacket {
    pub fn new(outcome: impl Into<String>, expires: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4(),
            outcome: outcome.into(),
            context: serde_json::Value::Null,
            constraints: Vec::new(),
            authority: Vec::new(),
            forbidden: Vec::new(),
            reversibility: Reversibility::Reversible,
            expires,
            expiry_action: ExpiryAction::Halt,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires
    }

    pub fn with_context(mut self, ctx: serde_json::Value) -> Self {
        self.context = ctx;
        self
    }

    pub fn with_authority(mut self, authority: Vec<String>) -> Self {
        self.authority = authority;
        self
    }

    pub fn with_reversibility(mut self, r: Reversibility) -> Self {
        self.reversibility = r;
        self
    }

    pub fn with_expiry_action(mut self, action: ExpiryAction) -> Self {
        self.expiry_action = action;
        self
    }
}

// ── Reversibility ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    Reversible,
    Partial,
    Irreversible,
}

// ── Forbidden Actions ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenAction {
    pub action: String,
    pub reason: String,
}

// ── Expiry ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpiryAction {
    Halt,
    Escalate,
    CompleteAndHalt,
}

// ── Admission Control ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdmissionResult {
    pub feasible: bool,
    pub dimensions: Vec<FeasibilityAssessment>,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityAssessment {
    pub dimension: FeasibilityDimension,
    pub kind: FeasibilityKind,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeasibilityDimension {
    Capability,
    Context,
    Resources,
    Authority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeasibilityKind {
    Feasible,
    FeasibleWithConstraints,
    Uncertain,
    Infeasible,
}

pub trait AdmissionController: Send + Sync {
    fn evaluate(&self, intent: &IntentPacket) -> AdmissionResult;
}

// ── Intent Decomposition ───────────────────────────────────────────

/// A node in the intent decomposition tree. Authority can only narrow
/// during decomposition, never expand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentNode {
    pub id: Uuid,
    pub intent: IntentPacket,
    pub children: Vec<IntentNode>,
}

impl IntentNode {
    pub fn leaf(intent: IntentPacket) -> Self {
        Self {
            id: Uuid::new_v4(),
            intent,
            children: Vec::new(),
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

// ── Errors ─────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum IntentError {
    #[error("intent expired at {0}")]
    Expired(DateTime<Utc>),
    #[error("intent forbidden by rule: {0}")]
    Forbidden(String),
    #[error("intent infeasible: {0}")]
    Infeasible(String),
}
