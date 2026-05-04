//! Admission control — feasibility gate before any planning begins.
//!
//! Cheap checks that filter out obviously infeasible, expired, or forbidden
//! intents. Admission control deliberately knows nothing about plans —
//! it only inspects the [`IntentPacket`] itself.

use chrono::Utc;

use crate::{
    AdmissionController, AdmissionResult, FeasibilityAssessment, FeasibilityDimension, IntentError,
    IntentPacket, Reversibility,
};

/// Result of an admission decision.
#[derive(Debug)]
pub enum Admission {
    Admit,
    Reject(IntentError),
}

/// Run admission control over an intent packet.
pub fn admit(intent: &IntentPacket) -> Admission {
    if intent.is_expired(Utc::now()) {
        return Admission::Reject(IntentError::Expired(intent.expires));
    }
    if intent.outcome.trim().is_empty() {
        return Admission::Reject(IntentError::Infeasible("empty outcome".into()));
    }
    Admission::Admit
}

/// Default admission controller that evaluates all 4 feasibility dimensions.
///
/// - **Capability**: checks that the intent doesn't reference unknown capabilities
/// - **Context**: checks that context is well-formed (not missing required fields)
/// - **Resources**: checks budget/time constraints are satisfiable
/// - **Authority**: checks that declared authority scopes are non-empty for
///   irreversible actions
pub struct DefaultAdmissionController {
    /// Known capability names the system can handle.
    known_capabilities: Vec<String>,
}

impl DefaultAdmissionController {
    pub fn new() -> Self {
        Self {
            known_capabilities: Vec::new(),
        }
    }

    /// Register known capabilities for the capability dimension check.
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.known_capabilities = capabilities;
        self
    }

    fn assess_capability(&self, intent: &IntentPacket) -> FeasibilityAssessment {
        if let Some(arr) = intent
            .context
            .get("required_capabilities")
            .and_then(|v| v.as_array())
        {
            let missing: Vec<_> = arr
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|cap| {
                    !self.known_capabilities.is_empty()
                        && !self.known_capabilities.iter().any(|k| k == cap)
                })
                .collect();

            if !missing.is_empty() {
                return FeasibilityAssessment::infeasible(
                    FeasibilityDimension::Capability,
                    format!("unknown capabilities: {}", missing.join(", ")),
                );
            }
        }

        FeasibilityAssessment::feasible(
            FeasibilityDimension::Capability,
            "all capabilities available",
        )
    }

    #[allow(clippy::unused_self)]
    fn assess_context(&self, intent: &IntentPacket) -> FeasibilityAssessment {
        if intent.outcome.trim().is_empty() {
            return FeasibilityAssessment::infeasible(
                FeasibilityDimension::Context,
                "empty outcome",
            );
        }

        if intent.is_expired(Utc::now()) {
            return FeasibilityAssessment::infeasible(
                FeasibilityDimension::Context,
                "intent already expired",
            );
        }

        FeasibilityAssessment::feasible(FeasibilityDimension::Context, "context well-formed")
    }

    #[allow(clippy::unused_self)]
    fn assess_resources(&self, intent: &IntentPacket) -> FeasibilityAssessment {
        let remaining = intent.expires - Utc::now();
        if remaining.num_seconds() < 60 {
            return FeasibilityAssessment::with_constraints(
                FeasibilityDimension::Resources,
                format!(
                    "only {}s remaining — tight deadline",
                    remaining.num_seconds()
                ),
            );
        }

        FeasibilityAssessment::feasible(FeasibilityDimension::Resources, "adequate time budget")
    }

    #[allow(clippy::unused_self)]
    fn assess_authority(&self, intent: &IntentPacket) -> FeasibilityAssessment {
        if intent.reversibility == Reversibility::Irreversible && intent.authority.is_empty() {
            return FeasibilityAssessment::uncertain(
                FeasibilityDimension::Authority,
                "irreversible action with no declared authority scope",
            );
        }

        FeasibilityAssessment::feasible(FeasibilityDimension::Authority, "authority scope declared")
    }
}

impl AdmissionController for DefaultAdmissionController {
    fn evaluate(&self, intent: &IntentPacket) -> AdmissionResult {
        AdmissionResult::from_dimensions(vec![
            self.assess_capability(intent),
            self.assess_context(intent),
            self.assess_resources(intent),
            self.assess_authority(intent),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FeasibilityKind;
    use chrono::Duration;

    #[test]
    fn rejects_expired() {
        let intent = IntentPacket::new("ship q3", Utc::now() - Duration::seconds(1));
        assert!(matches!(admit(&intent), Admission::Reject(_)));
    }

    #[test]
    fn rejects_empty_outcome() {
        let intent = IntentPacket::new("", Utc::now() + Duration::hours(1));
        assert!(matches!(admit(&intent), Admission::Reject(_)));
    }

    #[test]
    fn admits_valid() {
        let intent = IntentPacket::new("ship q3", Utc::now() + Duration::hours(1));
        assert!(matches!(admit(&intent), Admission::Admit));
    }

    // ── DefaultAdmissionController tests ──────────────────────────────

    #[test]
    fn controller_admits_valid_intent() {
        let controller = DefaultAdmissionController::new();
        let intent = IntentPacket::new("hire senior engineer", Utc::now() + Duration::hours(24));
        let result = controller.evaluate(&intent);
        assert!(result.feasible);
        assert_eq!(result.dimensions.len(), 4);
        assert!(result.rejection_reason.is_none());
    }

    #[test]
    fn controller_rejects_expired_intent() {
        let controller = DefaultAdmissionController::new();
        let intent = IntentPacket::new("too late", Utc::now() - Duration::hours(1));
        let result = controller.evaluate(&intent);
        assert!(!result.feasible);
        assert!(result.rejection_reason.is_some());
    }

    #[test]
    fn controller_rejects_empty_outcome() {
        let controller = DefaultAdmissionController::new();
        let intent = IntentPacket::new("   ", Utc::now() + Duration::hours(1));
        let result = controller.evaluate(&intent);
        assert!(!result.feasible);
    }

    #[test]
    fn controller_flags_irreversible_without_authority() {
        let controller = DefaultAdmissionController::new();
        let intent = IntentPacket::new(
            "delete production database",
            Utc::now() + Duration::hours(1),
        )
        .with_reversibility(Reversibility::Irreversible);
        let result = controller.evaluate(&intent);
        // Feasible (uncertain is not infeasible) but flagged
        assert!(result.feasible);
        let auth_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Authority)
            .unwrap();
        assert_eq!(auth_dim.kind, FeasibilityKind::Uncertain);
    }

    #[test]
    fn controller_accepts_irreversible_with_authority() {
        let controller = DefaultAdmissionController::new();
        let intent = IntentPacket::new(
            "delete production database",
            Utc::now() + Duration::hours(1),
        )
        .with_reversibility(Reversibility::Irreversible)
        .with_authority(vec!["infra-admin".into()]);
        let result = controller.evaluate(&intent);
        assert!(result.feasible);
        let auth_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Authority)
            .unwrap();
        assert_eq!(auth_dim.kind, FeasibilityKind::Feasible);
    }

    #[test]
    fn controller_flags_missing_capabilities() {
        let controller =
            DefaultAdmissionController::new().with_capabilities(vec!["web".into(), "ocr".into()]);
        let intent = IntentPacket::new("analyze patents", Utc::now() + Duration::hours(1))
            .with_context(serde_json::json!({
                "required_capabilities": ["web", "patent_search"]
            }));
        let result = controller.evaluate(&intent);
        assert!(!result.feasible);
        let cap_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Capability)
            .unwrap();
        assert_eq!(cap_dim.kind, FeasibilityKind::Infeasible);
        assert!(cap_dim.reason.contains("patent_search"));
    }

    #[test]
    fn controller_flags_tight_deadline() {
        let controller = DefaultAdmissionController::new();
        let intent = IntentPacket::new("quick task", Utc::now() + Duration::seconds(30));
        let result = controller.evaluate(&intent);
        assert!(result.feasible); // still feasible, just constrained
        let res_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Resources)
            .unwrap();
        assert_eq!(res_dim.kind, FeasibilityKind::FeasibleWithConstraints);
    }
}
