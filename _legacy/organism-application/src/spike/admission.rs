// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Admission control for the Budget Approval Decision spike.
//!
//! Implements `organism_core::intent::AdmissionController` to gate
//! whether a budget intent should enter the system at all.

use organism_core::intent::{
    AdmissionController, AdmissionResult, FeasibilityDimension, FeasibilityKind, OrganismIntent,
};

/// Admission controller for budget approval requests.
///
/// Checks four dimensions:
/// - **Capability**: Pure deterministic agents are available
/// - **Context**: Budget data is present in seeds
/// - **Resources**: Within compute budget
/// - **Authority**: Department delegation limit vs requested amount
pub struct BudgetAdmissionController {
    delegation_limit: u64,
}

impl BudgetAdmissionController {
    pub fn new(delegation_limit: u64) -> Self {
        Self { delegation_limit }
    }
}

impl AdmissionController for BudgetAdmissionController {
    fn evaluate(&self, intent: &OrganismIntent) -> AdmissionResult {
        let root = intent.root_intent();
        let mut dimensions = Vec::new();
        let mut blockers = Vec::new();

        // Capability: always feasible (pure agents)
        dimensions.push(FeasibilityDimension {
            kind: FeasibilityKind::Capability,
            feasible: true,
            reason: "Pure deterministic agents available".into(),
        });

        // Context: check that budget constraint exists in the intent
        let has_budget_constraint = root.constraints.iter().any(|c| c.key == "budget");
        dimensions.push(FeasibilityDimension {
            kind: FeasibilityKind::Context,
            feasible: has_budget_constraint,
            reason: if has_budget_constraint {
                "Budget constraint present in intent".into()
            } else {
                "Missing budget constraint in intent".into()
            },
        });
        if !has_budget_constraint {
            blockers.push("Intent must include a budget constraint".into());
        }

        // Resources: always feasible for this spike
        dimensions.push(FeasibilityDimension {
            kind: FeasibilityKind::Resources,
            feasible: true,
            reason: "Within compute budget for deterministic agents".into(),
        });

        // Authority: extract amount from budget constraint
        let amount: u64 = root
            .constraints
            .iter()
            .find(|c| c.key == "budget")
            .and_then(|c| c.value.split_whitespace().next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let within_authority = amount <= self.delegation_limit;
        dimensions.push(FeasibilityDimension {
            kind: FeasibilityKind::Authority,
            feasible: within_authority,
            reason: if within_authority {
                format!(
                    "Requested ${amount} within delegation limit ${}",
                    self.delegation_limit
                )
            } else {
                format!(
                    "Requested ${amount} exceeds delegation limit ${}",
                    self.delegation_limit
                )
            },
        });
        if !within_authority {
            blockers.push(format!(
                "Amount ${amount} exceeds department delegation limit ${}",
                self.delegation_limit
            ));
        }

        AdmissionResult {
            admitted: blockers.is_empty(),
            dimensions,
            blockers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spike::scenario::test_budget_intent;

    #[test]
    fn admits_within_delegation() {
        let controller = BudgetAdmissionController::new(100_000);
        let intent = test_budget_intent(50_000);
        let result = controller.evaluate(&intent);
        assert!(result.admitted);
        assert!(result.blockers.is_empty());
    }

    #[test]
    fn rejects_over_delegation() {
        let controller = BudgetAdmissionController::new(100_000);
        let intent = test_budget_intent(200_000);
        let result = controller.evaluate(&intent);
        assert!(!result.admitted);
        assert!(!result.blockers.is_empty());
    }

    #[test]
    fn all_dimensions_evaluated() {
        let controller = BudgetAdmissionController::new(100_000);
        let intent = test_budget_intent(50_000);
        let result = controller.evaluate(&intent);
        assert_eq!(result.dimensions.len(), 4);
    }
}
