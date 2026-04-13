// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Scenario wiring for the Budget Approval Decision spike.
//!
//! Builds the converge Engine with all 7 agents, 4 invariants,
//! and seeds from the OrganismIntent.

use converge_core::Engine;
use organism_core::commit::CommitBarrierInvariant;
use organism_core::intent::{OrganismIntent, Reversibility};

use crate::spike::agents::{
    AuthorityVerificationAgent, BudgetAdversarialAgent, DecisionAgent, IntentDecompositionAgent,
    PlanRevisionAgent, PlanningAgent, SimulationAgent,
};
use crate::spike::invariants::{
    BudgetEnvelopeInvariant, ChallengeResolutionInvariant, DecisionRequiredInvariant,
};

/// Build a fully wired convergence engine for a budget approval decision.
pub fn build_budget_approval_engine(intent: &OrganismIntent) -> Engine {
    let root = intent.root_intent();

    // Extract amount from intent constraints
    let amount: u64 = root
        .constraints
        .iter()
        .find(|c| c.key == "budget")
        .and_then(|c| c.value.split_whitespace().next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Extract metadata from intent
    let department = root.objective.as_ref().map_or("general", |o| match o {
        converge_core::Objective::Custom(s) => {
            if s.contains("marketing") {
                "marketing"
            } else {
                "general"
            }
        }
        _ => "general",
    });

    let channel = root
        .constraints
        .iter()
        .find(|c| c.key == "channel")
        .map_or("LinkedIn", |c| &c.value);

    let reversibility_str = match intent.reversibility {
        Reversibility::Reversible => "Reversible",
        Reversibility::Partial => "Partial",
        Reversibility::Irreversible => "Irreversible",
    };

    let mut engine = Engine::new();

    // Register 7 agents in pipeline order
    engine.register(IntentDecompositionAgent {
        amount,
        department: department.to_string(),
        purpose: root.objective.as_ref().map_or_else(
            || "budget approval".into(),
            |o| match o {
                converge_core::Objective::Custom(s) => s.clone(),
                _ => "budget approval".into(),
            },
        ),
        channel: channel.to_string(),
        reversibility: reversibility_str.to_string(),
    });
    engine.register(AuthorityVerificationAgent {
        delegation_limit: 100_000,
    });
    engine.register(PlanningAgent);
    engine.register(BudgetAdversarialAgent);
    engine.register(PlanRevisionAgent);
    engine.register(SimulationAgent);
    engine.register(DecisionAgent);

    // Register 4 invariants
    // Truth #6: Governance is structural
    engine.register_invariant(BudgetEnvelopeInvariant::new(amount));
    engine.register_invariant(ChallengeResolutionInvariant);
    engine.register_invariant(DecisionRequiredInvariant);
    // Truth #2: Authority re-derived at commit
    engine.register_invariant(CommitBarrierInvariant::new(intent.reversibility));

    engine
}

/// Create a test OrganismIntent for a given budget amount.
pub fn test_budget_intent(amount: u64) -> OrganismIntent {
    use converge_core::{
        Budgets, ConstraintSeverity, IntentConstraint, IntentId, IntentKind, Objective, RootIntent,
        Scope, SuccessCriteria,
    };
    use organism_core::intent::{Expiry, ExpiryAction};

    let root = RootIntent {
        id: IntentId::new("budget-approval-001"),
        kind: IntentKind::Custom,
        objective: Some(Objective::Custom(
            "Approve marketing spend for Q2 LinkedIn campaign".into(),
        )),
        scope: Scope::default(),
        constraints: vec![
            IntentConstraint {
                key: "budget".into(),
                value: format!("{amount} USD"),
                severity: ConstraintSeverity::Hard,
            },
            IntentConstraint {
                key: "channel".into(),
                value: "LinkedIn".into(),
                severity: ConstraintSeverity::Soft,
            },
        ],
        success_criteria: SuccessCriteria::default(),
        budgets: Budgets::default(),
    };

    OrganismIntent::new(root)
        .with_reversibility(Reversibility::Partial)
        .with_expiry(Expiry {
            deadline: "2026-06-30T00:00:00Z".into(),
            on_expiry: ExpiryAction::Halt,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Context;

    #[test]
    fn engine_builds_without_panic() {
        let intent = test_budget_intent(50_000);
        let _engine = build_budget_approval_engine(&intent);
    }

    #[test]
    fn engine_runs_to_convergence() {
        let intent = test_budget_intent(50_000);
        let mut engine = build_budget_approval_engine(&intent);
        let result = engine.run(Context::new()).expect("should converge");
        assert!(result.converged);
    }
}
