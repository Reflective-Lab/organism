//! Adversarial agents.
//!
//! Institutionalized disagreement: assumption breakers, constraint checkers,
//! causal skeptics, economic skeptics, operational skeptics. Their job is to
//! attack candidate plans before they reach the simulation swarm.
//!
//! Challenges emitted as Facts block convergence. The debate loop cycles:
//! planning proposes → adversaries challenge → planning revises → repeat.
//! Converge's fixed-point detection handles this naturally.
//!
//! Second-order effect: adversarial firings become labeled training signals
//! for the learning system.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Challenge ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: Uuid,
    pub kind: SkepticismKind,
    pub target_plan: Uuid,
    pub description: String,
    pub severity: Severity,
    pub evidence: Vec<String>,
    pub suggestion: Option<String>,
}

impl Challenge {
    pub fn new(
        kind: SkepticismKind,
        target_plan: Uuid,
        description: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            target_plan,
            description: description.into(),
            severity,
            evidence: Vec::new(),
            suggestion: None,
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Blocker
    }
}

// ── Skepticism Taxonomy ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkepticismKind {
    AssumptionBreaking,
    ConstraintChecking,
    CausalSkepticism,
    EconomicSkepticism,
    OperationalSkepticism,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Advisory,
    Warning,
    Blocker,
}

// ── Finding (simplified challenge for reporting) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub agent: String,
    pub severity: Severity,
    pub message: String,
}

// ── Adversarial Signal (training data for learning system) ─────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialSignal {
    pub kind: SkepticismKind,
    pub failed_assumption: String,
    pub context: serde_json::Value,
    pub revision_summary: Option<String>,
}

// ── Skeptic Trait ──────────────────────────────────────────────────

pub trait Skeptic: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SkepticismKind;
    fn review(&self, plan: &serde_json::Value) -> Vec<Challenge>;
}
