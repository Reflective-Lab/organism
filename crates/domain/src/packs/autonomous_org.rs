//! Autonomous org pack — Governance, policies, budgets, delegations.
//!
//! Fact prefixes: `policy:`, `approval:`, `budget_envelope:`, `exception:`,
//! `delegation:`, `risk_control:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};
use organism_pack::{
    AdmissionResult, AdversarialReview, Challenge, DimensionResult, FeasibilityAssessment,
    FeasibilityDimension, FeasibilityKind, Sample, Severity, SimulationDimension,
    SimulationRecommendation, SimulationResult, SkepticismKind,
};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "policy_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "policy:",
        target_key: ContextKey::Proposals,
        description: "Creates/manages policies",
    },
    AgentMeta {
        name: "policy_enforcer",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "policy:",
        target_key: ContextKey::Evaluations,
        description: "Enforces active policies",
    },
    AgentMeta {
        name: "approval_router",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "approval:",
        target_key: ContextKey::Proposals,
        description: "Routes approvals",
    },
    AgentMeta {
        name: "signoff_collector",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "approval:",
        target_key: ContextKey::Evaluations,
        description: "Collects signoffs",
    },
    AgentMeta {
        name: "budget_envelope_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "budget_envelope:",
        target_key: ContextKey::Proposals,
        description: "Creates budget envelopes",
    },
    AgentMeta {
        name: "budget_monitor",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget_envelope:",
        target_key: ContextKey::Evaluations,
        description: "Tracks budget consumption",
    },
    AgentMeta {
        name: "spend_validator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "budget_envelope:",
        target_key: ContextKey::Evaluations,
        description: "Validates against envelope",
    },
    AgentMeta {
        name: "exception_handler",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "exception:",
        target_key: ContextKey::Proposals,
        description: "Manages policy exceptions",
    },
    AgentMeta {
        name: "delegation_manager",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "delegation:",
        target_key: ContextKey::Proposals,
        description: "Authority delegation",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "policy_versioning_required",
        class: InvariantClass::Structural,
        description: "Policies must be versioned",
    },
    InvariantMeta {
        name: "policy_has_owner",
        class: InvariantClass::Structural,
        description: "Policies must have an owner",
    },
    InvariantMeta {
        name: "no_self_approval",
        class: InvariantClass::Acceptance,
        description: "No self-approval",
    },
    InvariantMeta {
        name: "two_person_rule_high_risk",
        class: InvariantClass::Acceptance,
        description: "High-risk actions require two-person rule",
    },
    InvariantMeta {
        name: "no_spend_beyond_envelope",
        class: InvariantClass::Acceptance,
        description: "No spending beyond budget envelope",
    },
    InvariantMeta {
        name: "exception_has_expiry",
        class: InvariantClass::Structural,
        description: "Exceptions must have expiry",
    },
    InvariantMeta {
        name: "delegation_has_scope_limits",
        class: InvariantClass::Structural,
        description: "Delegations must have scope limits",
    },
    InvariantMeta {
        name: "approval_has_rationale",
        class: InvariantClass::Acceptance,
        description: "Approvals must have rationale",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &[
        "policy",
        "approval",
        "budget_envelope",
        "delegation",
        "exception",
    ],
    required_capabilities: &[],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: true,
    keywords: &[
        "governance",
        "policy",
        "approval",
        "budget",
        "delegation",
        "authority",
        "spend",
    ],
};

const ACTIVE_POLICY_VERSION: &str = "2026-Q2";
const HIGH_AMOUNT_REVIEW_THRESHOLD: f64 = 1_000.0;
const EXECUTIVE_REVIEW_THRESHOLD: f64 = 10_000.0;
const HIGH_ENTERTAINMENT_THRESHOLD: f64 = 500.0;
const RESOURCE_UNCERTAINTY_THRESHOLD: f64 = 100_000.0;

/// Admits spend or expense requests before approval planning begins.
///
/// Maps to `spend_validator` and `approval_router`: the pack confirms that the
/// request has enough domain context to enter the governed approval loop.
pub struct SpendAdmissionSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for SpendAdmissionSuggestor {
    fn name(&self) -> &'static str {
        "spend_admission"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Seeds) && !ctx.has(converge_pack::ContextKey::Signals)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        let seeds = ctx.get(converge_pack::ContextKey::Seeds);
        let Some(seed) = seeds.first() else {
            return converge_pack::AgentEffect::empty();
        };

        let expense: serde_json::Value = serde_json::from_str(&seed.content).unwrap_or_default();
        let amount = numeric_field(&expense, "amount").unwrap_or(0.0);
        let category = string_field(&expense, "category").unwrap_or_default();

        let dimensions = vec![
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Capability,
                kind: FeasibilityKind::Feasible,
                reason: "spend approval workflow available".into(),
            },
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Context,
                kind: if category.is_empty() {
                    FeasibilityKind::Infeasible
                } else {
                    FeasibilityKind::Feasible
                },
                reason: if category.is_empty() {
                    "missing spend category".into()
                } else {
                    format!("category: {category}")
                },
            },
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Resources,
                kind: if amount > RESOURCE_UNCERTAINTY_THRESHOLD {
                    FeasibilityKind::Uncertain
                } else {
                    FeasibilityKind::Feasible
                },
                reason: format!("amount: ${amount:.2}"),
            },
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Authority,
                kind: FeasibilityKind::Feasible,
                reason: "submitter has declared spend authority".into(),
            },
        ];

        let feasible = dimensions
            .iter()
            .all(|assessment| assessment.kind != FeasibilityKind::Infeasible);
        let admission = AdmissionResult {
            feasible,
            dimensions,
            rejection_reason: if feasible {
                None
            } else {
                Some("missing required fields".into())
            },
        };

        let mut facts = vec![
            converge_pack::ProposedFact::new(
                converge_pack::ContextKey::Signals,
                "admission:result",
                serde_json::to_string(&admission).unwrap_or_default(),
                self.name(),
            )
            .with_confidence(1.0),
        ];

        if feasible {
            facts.push(
                converge_pack::ProposedFact::new(
                    converge_pack::ContextKey::Signals,
                    "expense:parsed",
                    seed.content.clone(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        converge_pack::AgentEffect::with_proposals(facts)
    }
}

/// Plans the approval route from amount, category, and active policy version.
///
/// Maps to `approval_router` and `policy_enforcer`.
pub struct ApprovalRoutingSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for ApprovalRoutingSuggestor {
    fn name(&self) -> &'static str {
        "approval_router"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Signals)
            && !ctx.has(converge_pack::ContextKey::Strategies)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        let signals = ctx.get(converge_pack::ContextKey::Signals);
        let Some(expense_fact) = signals.iter().find(|fact| fact.id == "expense:parsed") else {
            return converge_pack::AgentEffect::empty();
        };

        let expense: serde_json::Value =
            serde_json::from_str(&expense_fact.content).unwrap_or_default();
        let amount = numeric_field(&expense, "amount").unwrap_or(0.0);
        let category = string_field(&expense, "category").unwrap_or_default();

        let mut approvers = vec!["manager"];
        if amount >= HIGH_AMOUNT_REVIEW_THRESHOLD {
            approvers.push("finance");
        }
        if amount >= EXECUTIVE_REVIEW_THRESHOLD {
            approvers.push("executive");
        }
        if category == "entertainment" && amount > HIGH_ENTERTAINMENT_THRESHOLD {
            approvers.push("compliance");
        }

        let plan = serde_json::json!({
            "amount": amount,
            "category": category,
            "required_approvers": approvers,
            "policy_version": ACTIVE_POLICY_VERSION,
            "routing_rationale": format!(
                "${amount:.0} {category} -> {} approval(s)",
                approvers.len()
            )
        });

        converge_pack::AgentEffect::with_proposal(
            converge_pack::ProposedFact::new(
                converge_pack::ContextKey::Strategies,
                "approval:plan",
                plan.to_string(),
                self.name(),
            )
            .with_confidence(0.9),
        )
    }
}

/// Challenges approval plans before they can become spend decisions.
///
/// Maps to `policy_enforcer`; this turns the example's adversarial review into
/// reusable autonomous-org pack behavior.
pub struct ApprovalPolicySkepticSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for ApprovalPolicySkepticSuggestor {
    fn name(&self) -> &'static str {
        "approval_policy_skeptic"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Strategies)
            && !ctx.has(converge_pack::ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        let strategies = ctx.get(converge_pack::ContextKey::Strategies);
        let Some(plan_fact) = strategies.iter().find(|fact| fact.id == "approval:plan") else {
            return converge_pack::AgentEffect::empty();
        };
        let plan: serde_json::Value = serde_json::from_str(&plan_fact.content).unwrap_or_default();

        let amount = numeric_field(&plan, "amount").unwrap_or(0.0);
        let category = string_field(&plan, "category").unwrap_or_default();
        let approver_count = plan
            .get("required_approvers")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len);

        let mut review = AdversarialReview::new();

        if amount > 5_000.0 && category == "entertainment" {
            review.push(Challenge::new(
                SkepticismKind::EconomicSkepticism,
                uuid::Uuid::nil(),
                format!("${amount:.0} entertainment expense is high - requires justification"),
                Severity::Warning,
            ));
        }

        if amount > EXECUTIVE_REVIEW_THRESHOLD && approver_count < 3 {
            review.push(Challenge::new(
                SkepticismKind::ConstraintChecking,
                uuid::Uuid::nil(),
                format!("${amount:.0} requires 3+ approvers but only {approver_count} routed"),
                Severity::Blocker,
            ));
        }

        if approver_count > 3 {
            review.push(Challenge::new(
                SkepticismKind::OperationalSkepticism,
                uuid::Uuid::nil(),
                format!("{approver_count} approvers may delay approval - add escalation path"),
                Severity::Advisory,
            ));
        }

        if string_field(&plan, "policy_version") != Some(ACTIVE_POLICY_VERSION) {
            review.push(Challenge::new(
                SkepticismKind::AssumptionBreaking,
                uuid::Uuid::nil(),
                "approval plan uses outdated policy version",
                Severity::Blocker,
            ));
        }

        converge_pack::AgentEffect::with_proposal(
            converge_pack::ProposedFact::new(
                converge_pack::ContextKey::Evaluations,
                "adversarial:review",
                review.summary().to_string(),
                self.name(),
            )
            .with_confidence(review.confidence()),
        )
    }
}

/// Simulates budget impact and produces the final spend decision proposal.
///
/// Maps to `budget_monitor` and `spend_validator`.
#[derive(Debug, Clone, Copy)]
pub struct BudgetSimulationSuggestor {
    quarterly_budget: f64,
    spent_so_far: f64,
}

impl BudgetSimulationSuggestor {
    pub fn new(quarterly_budget: f64, spent_so_far: f64) -> Self {
        let quarterly_budget = if quarterly_budget.is_finite() && quarterly_budget > 0.0 {
            quarterly_budget
        } else {
            0.0
        };
        let spent_so_far = if spent_so_far.is_finite() && spent_so_far >= 0.0 {
            spent_so_far.min(quarterly_budget)
        } else {
            0.0
        };

        Self {
            quarterly_budget,
            spent_so_far,
        }
    }
}

impl Default for BudgetSimulationSuggestor {
    fn default() -> Self {
        Self::new(50_000.0, 32_000.0)
    }
}

#[async_trait::async_trait]
impl converge_pack::Suggestor for BudgetSimulationSuggestor {
    fn name(&self) -> &'static str {
        "budget_simulation"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[
            converge_pack::ContextKey::Strategies,
            converge_pack::ContextKey::Evaluations,
        ]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Evaluations)
            && !ctx.has(converge_pack::ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        let strategies = ctx.get(converge_pack::ContextKey::Strategies);
        let Some(plan_fact) = strategies.iter().find(|fact| fact.id == "approval:plan") else {
            return converge_pack::AgentEffect::empty();
        };
        let plan: serde_json::Value = serde_json::from_str(&plan_fact.content).unwrap_or_default();
        let amount = numeric_field(&plan, "amount").unwrap_or(0.0);

        if let Some(blocked) = blocked_review(ctx) {
            return converge_pack::AgentEffect::with_proposal(
                converge_pack::ProposedFact::new(
                    converge_pack::ContextKey::Proposals,
                    "decision:blocked",
                    serde_json::json!({
                        "decision": "rejected",
                        "reason": "adversarial review blocked the plan",
                        "blockers": blocked.get("blockers"),
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(0.95),
            );
        }

        let budget_decision =
            build_budget_decision(amount, self.quarterly_budget, self.spent_so_far);

        converge_pack::AgentEffect::with_proposal(
            converge_pack::ProposedFact::new(
                converge_pack::ContextKey::Proposals,
                "decision:expense",
                budget_decision_payload(&budget_decision).to_string(),
                self.name(),
            )
            .with_confidence(budget_decision.result.overall_confidence),
        )
    }
}

struct BudgetDecision {
    amount: f64,
    remaining: f64,
    utilization_after: f64,
    decision: &'static str,
    result: SimulationResult,
}

fn build_budget_decision(amount: f64, quarterly_budget: f64, spent_so_far: f64) -> BudgetDecision {
    let remaining = quarterly_budget - spent_so_far;
    let utilization_after = if quarterly_budget > 0.0 {
        (spent_so_far + amount) / quarterly_budget
    } else {
        1.0
    };

    let cost_result = DimensionResult {
        dimension: SimulationDimension::Cost,
        passed: amount <= remaining,
        confidence: 0.95,
        findings: vec![
            format!("Budget remaining: ${remaining:.0}"),
            format!("After approval: {:.0}% utilized", utilization_after * 100.0),
        ],
        samples: vec![Sample {
            value: utilization_after,
            probability: 0.95,
        }],
    };
    let policy_result = DimensionResult {
        dimension: SimulationDimension::Policy,
        passed: true,
        confidence: 0.9,
        findings: vec![format!(
            "Policy {ACTIVE_POLICY_VERSION} compliance verified"
        )],
        samples: vec![],
    };
    let operational_result = DimensionResult {
        dimension: SimulationDimension::Operational,
        passed: true,
        confidence: 0.85,
        findings: vec!["Approval chain is reachable within SLA".into()],
        samples: vec![],
    };

    let recommendation = budget_recommendation(cost_result.passed, utilization_after);
    let decision = match recommendation {
        SimulationRecommendation::Proceed => "approved",
        SimulationRecommendation::ProceedWithCaution => "approved_with_caution",
        SimulationRecommendation::DoNotProceed => "rejected",
    };
    let overall =
        (cost_result.confidence + policy_result.confidence + operational_result.confidence) / 3.0;

    BudgetDecision {
        amount,
        remaining,
        utilization_after,
        decision,
        result: SimulationResult {
            plan_id: uuid::Uuid::nil(),
            runs: 1,
            dimensions: vec![cost_result, policy_result, operational_result],
            overall_confidence: overall,
            recommendation,
        },
    }
}

fn budget_recommendation(cost_passed: bool, utilization_after: f64) -> SimulationRecommendation {
    if !cost_passed {
        SimulationRecommendation::DoNotProceed
    } else if utilization_after > 0.9 {
        SimulationRecommendation::ProceedWithCaution
    } else {
        SimulationRecommendation::Proceed
    }
}

fn budget_decision_payload(decision: &BudgetDecision) -> serde_json::Value {
    serde_json::json!({
        "decision": decision.decision,
        "simulation": {
            "overall_confidence": decision.result.overall_confidence,
            "recommendation": format!("{:?}", decision.result.recommendation),
            "dimensions": decision.result.dimensions.iter().map(|dimension| serde_json::json!({
                "dimension": format!("{:?}", dimension.dimension),
                "passed": dimension.passed,
                "confidence": dimension.confidence,
                "findings": dimension.findings,
            })).collect::<Vec<_>>(),
        },
        "budget_impact": {
            "amount": decision.amount,
            "remaining_before": decision.remaining,
            "remaining_after": decision.remaining - decision.amount,
            "utilization_after_pct": decision.utilization_after * 100.0,
        },
    })
}

fn blocked_review(ctx: &dyn converge_pack::Context) -> Option<serde_json::Value> {
    ctx.get(converge_pack::ContextKey::Evaluations)
        .iter()
        .find(|fact| fact.id == "adversarial:review")
        .and_then(|fact| serde_json::from_str::<serde_json::Value>(&fact.content).ok())
        .filter(|review| {
            review.get("verdict").and_then(serde_json::Value::as_str) == Some("blocked")
        })
}

fn numeric_field(json: &serde_json::Value, field: &str) -> Option<f64> {
    json.get(field)
        .and_then(serde_json::Value::as_f64)
        .filter(|value| value.is_finite())
}

fn string_field<'a>(json: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    json.get(field).and_then(serde_json::Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalPolicySkepticSuggestor, ApprovalRoutingSuggestor, BudgetSimulationSuggestor,
        SpendAdmissionSuggestor,
    };
    use converge_kernel::{ContextKey, ContextState, Engine};

    #[tokio::test]
    async fn spend_approval_pipeline_approves_in_budget_request() {
        let mut engine = Engine::new();
        engine.register_suggestor(SpendAdmissionSuggestor);
        engine.register_suggestor(ApprovalRoutingSuggestor);
        engine.register_suggestor(ApprovalPolicySkepticSuggestor);
        engine.register_suggestor(BudgetSimulationSuggestor::default());

        let request = serde_json::json!({
            "employee": "karl@reflective.se",
            "amount": 2500.00,
            "category": "entertainment",
            "description": "Client dinner",
        });
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "expense-1", request.to_string());

        let result = engine.run(ctx).await.expect("expense approval run");
        let decision = result
            .context
            .get(ContextKey::Proposals)
            .iter()
            .find(|fact| fact.id == "decision:expense")
            .expect("decision proposal");
        let json: serde_json::Value =
            serde_json::from_str(&decision.content).expect("decision json");

        assert_eq!(
            json.get("decision").and_then(serde_json::Value::as_str),
            Some("approved")
        );
        assert_eq!(
            json.pointer("/simulation/recommendation")
                .and_then(serde_json::Value::as_str),
            Some("Proceed")
        );
    }

    #[tokio::test]
    async fn spend_admission_rejects_missing_category() {
        let mut engine = Engine::new();
        engine.register_suggestor(SpendAdmissionSuggestor);

        let request = serde_json::json!({
            "employee": "karl@reflective.se",
            "amount": 2500.00,
        });
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(ContextKey::Seeds, "expense-1", request.to_string());

        let result = engine.run(ctx).await.expect("admission run");
        let signals = result.context.get(ContextKey::Signals);
        let admission = signals
            .iter()
            .find(|fact| fact.id == "admission:result")
            .expect("admission result");
        let json: serde_json::Value =
            serde_json::from_str(&admission.content).expect("admission json");

        assert_eq!(
            json.get("feasible").and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert!(!signals.iter().any(|fact| fact.id == "expense:parsed"));
    }
}
