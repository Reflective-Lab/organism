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

impl AdmissionResult {
    /// Build a result with `feasible` derived from the standard rule (any
    /// Infeasible dimension blocks the intent) and `rejection_reason`
    /// auto-composed from the infeasible reasons joined by "; ".
    #[must_use]
    pub fn from_dimensions(dimensions: Vec<FeasibilityAssessment>) -> Self {
        let infeasible_reasons: Vec<String> = dimensions
            .iter()
            .filter(|d| d.kind == FeasibilityKind::Infeasible)
            .map(|d| d.reason.clone())
            .collect();
        let feasible = infeasible_reasons.is_empty();
        let rejection_reason = if feasible {
            None
        } else {
            Some(infeasible_reasons.join("; "))
        };
        Self {
            feasible,
            dimensions,
            rejection_reason,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeasibilityAssessment {
    pub dimension: FeasibilityDimension,
    pub kind: FeasibilityKind,
    pub reason: String,
}

impl FeasibilityAssessment {
    #[must_use]
    pub fn feasible(dimension: FeasibilityDimension, reason: impl Into<String>) -> Self {
        Self {
            dimension,
            kind: FeasibilityKind::Feasible,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn infeasible(dimension: FeasibilityDimension, reason: impl Into<String>) -> Self {
        Self {
            dimension,
            kind: FeasibilityKind::Infeasible,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn uncertain(dimension: FeasibilityDimension, reason: impl Into<String>) -> Self {
        Self {
            dimension,
            kind: FeasibilityKind::Uncertain,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn with_constraints(dimension: FeasibilityDimension, reason: impl Into<String>) -> Self {
        Self {
            dimension,
            kind: FeasibilityKind::FeasibleWithConstraints,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn is_blocking(&self) -> bool {
        self.kind == FeasibilityKind::Infeasible
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn future() -> DateTime<Utc> {
        Utc::now() + Duration::hours(1)
    }

    fn past() -> DateTime<Utc> {
        Utc::now() - Duration::seconds(10)
    }

    #[test]
    fn new_sets_defaults() {
        let intent = IntentPacket::new("ship q3", future());
        assert_eq!(intent.outcome, "ship q3");
        assert_eq!(intent.context, serde_json::Value::Null);
        assert!(intent.constraints.is_empty());
        assert!(intent.authority.is_empty());
        assert!(intent.forbidden.is_empty());
        assert_eq!(intent.reversibility, Reversibility::Reversible);
        assert_eq!(intent.expiry_action, ExpiryAction::Halt);
    }

    #[test]
    fn new_generates_unique_ids() {
        let a = IntentPacket::new("a", future());
        let b = IntentPacket::new("b", future());
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn is_expired_past() {
        let intent = IntentPacket::new("late", past());
        assert!(intent.is_expired(Utc::now()));
    }

    #[test]
    fn is_expired_future() {
        let intent = IntentPacket::new("on time", future());
        assert!(!intent.is_expired(Utc::now()));
    }

    #[test]
    fn is_expired_exact_boundary() {
        let now = Utc::now();
        let intent = IntentPacket::new("boundary", now);
        assert!(intent.is_expired(now));
    }

    #[test]
    fn with_context() {
        let intent =
            IntentPacket::new("ctx", future()).with_context(serde_json::json!({"key": "value"}));
        assert_eq!(intent.context["key"], "value");
    }

    #[test]
    fn with_authority() {
        let intent = IntentPacket::new("auth", future())
            .with_authority(vec!["admin".into(), "finance".into()]);
        assert_eq!(intent.authority.len(), 2);
        assert_eq!(intent.authority[0], "admin");
    }

    #[test]
    fn with_reversibility() {
        let intent =
            IntentPacket::new("rev", future()).with_reversibility(Reversibility::Irreversible);
        assert_eq!(intent.reversibility, Reversibility::Irreversible);
    }

    #[test]
    fn with_expiry_action() {
        let intent = IntentPacket::new("exp", future()).with_expiry_action(ExpiryAction::Escalate);
        assert_eq!(intent.expiry_action, ExpiryAction::Escalate);
    }

    #[test]
    fn builder_chain() {
        let intent = IntentPacket::new("full", future())
            .with_context(serde_json::json!(null))
            .with_authority(vec![])
            .with_reversibility(Reversibility::Partial)
            .with_expiry_action(ExpiryAction::CompleteAndHalt);
        assert_eq!(intent.reversibility, Reversibility::Partial);
        assert_eq!(intent.expiry_action, ExpiryAction::CompleteAndHalt);
    }

    #[test]
    fn serde_roundtrip() {
        let intent = IntentPacket::new("roundtrip", future())
            .with_context(serde_json::json!({"n": 42}))
            .with_authority(vec!["ops".into()])
            .with_reversibility(Reversibility::Partial)
            .with_expiry_action(ExpiryAction::Escalate);

        let json = serde_json::to_string(&intent).unwrap();
        let back: IntentPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, intent.id);
        assert_eq!(back.outcome, "roundtrip");
        assert_eq!(back.context["n"], 42);
        assert_eq!(back.authority, vec!["ops"]);
        assert_eq!(back.reversibility, Reversibility::Partial);
        assert_eq!(back.expiry_action, ExpiryAction::Escalate);
    }

    #[test]
    fn serde_with_forbidden() {
        let mut intent = IntentPacket::new("forbidden", future());
        intent.forbidden.push(ForbiddenAction {
            action: "delete_prod".into(),
            reason: "destructive".into(),
        });

        let json = serde_json::to_string(&intent).unwrap();
        let back: IntentPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(back.forbidden.len(), 1);
        assert_eq!(back.forbidden[0].action, "delete_prod");
    }

    #[test]
    fn reversibility_all_variants_serde() {
        for v in [
            Reversibility::Reversible,
            Reversibility::Partial,
            Reversibility::Irreversible,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: Reversibility = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn reversibility_snake_case() {
        assert_eq!(
            serde_json::to_string(&Reversibility::Reversible).unwrap(),
            "\"reversible\""
        );
        assert_eq!(
            serde_json::to_string(&Reversibility::Partial).unwrap(),
            "\"partial\""
        );
        assert_eq!(
            serde_json::to_string(&Reversibility::Irreversible).unwrap(),
            "\"irreversible\""
        );
    }

    #[test]
    fn expiry_action_all_variants_serde() {
        for v in [
            ExpiryAction::Halt,
            ExpiryAction::Escalate,
            ExpiryAction::CompleteAndHalt,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: ExpiryAction = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn expiry_action_snake_case() {
        assert_eq!(
            serde_json::to_string(&ExpiryAction::Halt).unwrap(),
            "\"halt\""
        );
        assert_eq!(
            serde_json::to_string(&ExpiryAction::Escalate).unwrap(),
            "\"escalate\""
        );
        assert_eq!(
            serde_json::to_string(&ExpiryAction::CompleteAndHalt).unwrap(),
            "\"complete_and_halt\""
        );
    }

    #[test]
    fn feasibility_dimension_all_variants_serde() {
        for v in [
            FeasibilityDimension::Capability,
            FeasibilityDimension::Context,
            FeasibilityDimension::Resources,
            FeasibilityDimension::Authority,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: FeasibilityDimension = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn feasibility_kind_all_variants_serde() {
        for v in [
            FeasibilityKind::Feasible,
            FeasibilityKind::FeasibleWithConstraints,
            FeasibilityKind::Uncertain,
            FeasibilityKind::Infeasible,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: FeasibilityKind = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn feasibility_kind_snake_case() {
        assert_eq!(
            serde_json::to_string(&FeasibilityKind::FeasibleWithConstraints).unwrap(),
            "\"feasible_with_constraints\""
        );
    }

    #[test]
    fn admission_result_serde_roundtrip() {
        let result = AdmissionResult {
            feasible: false,
            dimensions: vec![FeasibilityAssessment {
                dimension: FeasibilityDimension::Authority,
                kind: FeasibilityKind::Infeasible,
                reason: "no authority".into(),
            }],
            rejection_reason: Some("not authorized".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AdmissionResult = serde_json::from_str(&json).unwrap();
        assert!(!back.feasible);
        assert_eq!(back.dimensions.len(), 1);
        assert_eq!(back.rejection_reason.as_deref(), Some("not authorized"));
    }

    #[test]
    fn feasibility_constructors_set_kind() {
        let f = FeasibilityAssessment::feasible(FeasibilityDimension::Capability, "ok");
        assert_eq!(f.kind, FeasibilityKind::Feasible);
        assert_eq!(f.reason, "ok");

        let i = FeasibilityAssessment::infeasible(FeasibilityDimension::Context, "missing");
        assert_eq!(i.kind, FeasibilityKind::Infeasible);
        assert!(i.is_blocking());

        let u = FeasibilityAssessment::uncertain(FeasibilityDimension::Authority, "unclear");
        assert_eq!(u.kind, FeasibilityKind::Uncertain);
        assert!(!u.is_blocking());

        let c = FeasibilityAssessment::with_constraints(FeasibilityDimension::Resources, "tight");
        assert_eq!(c.kind, FeasibilityKind::FeasibleWithConstraints);
        assert!(!c.is_blocking());
    }

    #[test]
    fn admission_from_dimensions_feasible_when_no_infeasible() {
        let result = AdmissionResult::from_dimensions(vec![
            FeasibilityAssessment::feasible(FeasibilityDimension::Capability, "ok"),
            FeasibilityAssessment::with_constraints(FeasibilityDimension::Resources, "tight"),
            FeasibilityAssessment::uncertain(FeasibilityDimension::Authority, "unclear"),
        ]);
        assert!(result.feasible);
        assert!(result.rejection_reason.is_none());
        assert_eq!(result.dimensions.len(), 3);
    }

    #[test]
    fn admission_from_dimensions_infeasible_with_joined_reason() {
        let result = AdmissionResult::from_dimensions(vec![
            FeasibilityAssessment::feasible(FeasibilityDimension::Capability, "ok"),
            FeasibilityAssessment::infeasible(FeasibilityDimension::Context, "missing outcome"),
            FeasibilityAssessment::infeasible(FeasibilityDimension::Authority, "no authority"),
        ]);
        assert!(!result.feasible);
        assert_eq!(
            result.rejection_reason.as_deref(),
            Some("missing outcome; no authority")
        );
    }

    #[test]
    fn admission_from_dimensions_empty_is_feasible() {
        let result = AdmissionResult::from_dimensions(vec![]);
        assert!(result.feasible);
        assert!(result.rejection_reason.is_none());
        assert!(result.dimensions.is_empty());
    }

    #[test]
    fn forbidden_action_serde_roundtrip() {
        let fa = ForbiddenAction {
            action: "fire_all".into(),
            reason: "HR policy".into(),
        };
        let json = serde_json::to_string(&fa).unwrap();
        let back: ForbiddenAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "fire_all");
        assert_eq!(back.reason, "HR policy");
    }

    #[test]
    fn intent_node_leaf_is_leaf() {
        let node = IntentNode::leaf(IntentPacket::new("leaf", future()));
        assert!(node.is_leaf());
    }

    #[test]
    fn intent_node_with_children_not_leaf() {
        let child = IntentNode::leaf(IntentPacket::new("child", future()));
        let parent = IntentNode {
            id: Uuid::new_v4(),
            intent: IntentPacket::new("parent", future()),
            children: vec![child],
        };
        assert!(!parent.is_leaf());
    }

    #[test]
    fn intent_error_display() {
        let err = IntentError::Forbidden("no access".into());
        assert_eq!(err.to_string(), "intent forbidden by rule: no access");

        let err = IntentError::Infeasible("not enough resources".into());
        assert_eq!(err.to_string(), "intent infeasible: not enough resources");
    }

    #[test]
    fn intent_error_expired_display() {
        let t = Utc::now();
        let err = IntentError::Expired(t);
        assert!(err.to_string().starts_with("intent expired at "));
    }

    #[test]
    fn intent_packet_accepts_string_and_str() {
        let from_str = IntentPacket::new("literal", future());
        let from_string = IntentPacket::new(String::from("owned"), future());
        assert_eq!(from_str.outcome, "literal");
        assert_eq!(from_string.outcome, "owned");
    }

    #[test]
    fn intent_packet_empty_outcome() {
        let intent = IntentPacket::new("", future());
        assert_eq!(intent.outcome, "");
    }

    #[test]
    fn intent_packet_with_constraints() {
        let mut intent = IntentPacket::new("constrained", future());
        intent.constraints = vec!["budget < 10k".into(), "no external vendors".into()];
        let json = serde_json::to_string(&intent).unwrap();
        let back: IntentPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(back.constraints.len(), 2);
    }
}
