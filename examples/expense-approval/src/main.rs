// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Expense Approval — organism planning loop + converge-kernel.
//!
//! Demonstrates the FULL organism pipeline:
//!   Intent → Admission → Planning → Adversarial Review → Simulation → Converge
//!
//! Uses organism-domain autonomous_org + procurement packs.
//! Uses `organism-pack` as the curated planning contract.

use converge_kernel::{AgentEffect, ContextKey, Engine, ProposedFact, Suggestor};
use converge_pack::Context as ContextView;

use organism_pack::{
    // Intent
    AdmissionResult,
    // Adversarial
    Challenge,
    // Simulation
    DimensionResult,
    FeasibilityAssessment,
    FeasibilityDimension,
    FeasibilityKind,
    IntentPacket,
    Sample,
    Severity,
    SimulationDimension,
    SimulationRecommendation,
    SimulationResult,
    SkepticismKind,
};

// ── Stage 1: Intent Admission ──────────────────────────────────────

/// Evaluates whether the expense intent is feasible before planning begins.
/// Uses Organism's 4 feasibility dimensions through `organism-pack`.
struct IntentAdmissionAgent;

impl Suggestor for IntentAdmissionAgent {
    fn name(&self) -> &str {
        "intent_admission"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Signals)
    }

    fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let Some(seed) = seeds.first() else {
            return AgentEffect::empty();
        };
        let expense: serde_json::Value = serde_json::from_str(&seed.content).unwrap_or_default();

        let amount = expense
            .get("amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let category = expense
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Organism intent admission: check 4 feasibility dimensions
        let dimensions = vec![
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Capability,
                kind: FeasibilityKind::Feasible,
                reason: "expense approval workflow available".into(),
            },
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Context,
                kind: if category.is_empty() {
                    FeasibilityKind::Infeasible
                } else {
                    FeasibilityKind::Feasible
                },
                reason: if category.is_empty() {
                    "missing expense category".into()
                } else {
                    format!("category: {category}")
                },
            },
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Resources,
                kind: if amount > 100_000.0 {
                    FeasibilityKind::Uncertain
                } else {
                    FeasibilityKind::Feasible
                },
                reason: format!("amount: ${amount:.2}"),
            },
            FeasibilityAssessment {
                dimension: FeasibilityDimension::Authority,
                kind: FeasibilityKind::Feasible,
                reason: "submitter has expense authority".into(),
            },
        ];

        let feasible = dimensions
            .iter()
            .all(|d| d.kind != FeasibilityKind::Infeasible);
        let admission = AdmissionResult {
            feasible,
            dimensions: dimensions.clone(),
            rejection_reason: if feasible {
                None
            } else {
                Some("missing required fields".into())
            },
        };

        let mut facts = vec![
            ProposedFact::new(
                ContextKey::Signals,
                "admission:result",
                serde_json::to_string(&admission).unwrap_or_default(),
                self.name(),
            )
            .with_confidence(1.0),
        ];

        // Pass through the expense data if admitted
        if feasible {
            facts.push(
                ProposedFact::new(
                    ContextKey::Signals,
                    "expense:parsed",
                    seed.content.clone(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }

        AgentEffect::with_proposals(facts)
    }
}

// ── Stage 2: Policy Planning ───────────────────────────────────────

/// Plans the approval route based on amount thresholds and category.
/// Maps to: autonomous_org::approval_router + autonomous_org::policy_enforcer
struct PolicyPlanningAgent;

impl Suggestor for PolicyPlanningAgent {
    fn name(&self) -> &str {
        "policy_planning"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let expense = signals.iter().find(|s| s.id == "expense:parsed");
        let Some(expense) = expense else {
            return AgentEffect::empty();
        };

        let json: serde_json::Value = serde_json::from_str(&expense.content).unwrap_or_default();
        let amount = json.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let category = json.get("category").and_then(|v| v.as_str()).unwrap_or("");

        let mut approvers = vec!["manager"];
        if amount >= 1_000.0 {
            approvers.push("finance");
        }
        if amount >= 10_000.0 {
            approvers.push("executive");
        }
        if category == "entertainment" && amount > 500.0 {
            approvers.push("compliance");
        }

        let plan = serde_json::json!({
            "amount": amount,
            "category": category,
            "required_approvers": approvers,
            "policy_version": "2026-Q2",
            "routing_rationale": format!(
                "${:.0} {} → {} approval(s)",
                amount, category, approvers.len()
            )
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Strategies,
                "approval:plan",
                plan.to_string(),
                self.name(),
            )
            .with_confidence(0.9),
        )
    }
}

// ── Stage 3: Adversarial Review ────────────────────────────────────

/// Challenges the approval plan using the adversarial types re-exported by `organism-pack`.
/// Maps to: autonomous_org::policy_enforcer (skeptic role)
///
/// This is the organism differentiator — plans get challenged before commit.
struct PolicySkepticAgent;

impl Suggestor for PolicySkepticAgent {
    fn name(&self) -> &str {
        "policy_skeptic"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let Some(plan_fact) = strategies.first() else {
            return AgentEffect::empty();
        };
        let plan: serde_json::Value = serde_json::from_str(&plan_fact.content).unwrap_or_default();

        let amount = plan.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let category = plan.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let approvers = plan
            .get("required_approvers")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        let mut challenges: Vec<Challenge> = Vec::new();

        // Economic skepticism: is the spend justified?
        if amount > 5_000.0 && category == "entertainment" {
            challenges.push(Challenge::new(
                SkepticismKind::EconomicSkepticism,
                uuid::Uuid::nil(),
                format!(
                    "${amount:.0} entertainment expense is high — requires business justification"
                ),
                Severity::Warning,
            ));
        }

        // Constraint checking: are enough approvers in the chain?
        if amount > 10_000.0 && approvers < 3 {
            challenges.push(Challenge::new(
                SkepticismKind::ConstraintChecking,
                uuid::Uuid::nil(),
                format!("${amount:.0} requires 3+ approvers but only {approvers} routed"),
                Severity::Blocker,
            ));
        }

        // Operational skepticism: can this be processed in time?
        if approvers > 3 {
            challenges.push(Challenge::new(
                SkepticismKind::OperationalSkepticism,
                uuid::Uuid::nil(),
                format!("{approvers} approvers will cause delays — consider escalation path"),
                Severity::Advisory,
            ));
        }

        // Assumption breaking: is the policy version current?
        if plan
            .get("policy_version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            != "2026-Q2"
        {
            challenges.push(Challenge::new(
                SkepticismKind::AssumptionBreaking,
                uuid::Uuid::nil(),
                "approval plan uses outdated policy version",
                Severity::Blocker,
            ));
        }

        let has_blockers = challenges.iter().any(|c| c.severity == Severity::Blocker);
        let review = serde_json::json!({
            "challenges": challenges.len(),
            "blockers": challenges.iter().filter(|c| c.severity == Severity::Blocker).count(),
            "warnings": challenges.iter().filter(|c| c.severity == Severity::Warning).count(),
            "advisories": challenges.iter().filter(|c| c.severity == Severity::Advisory).count(),
            "details": challenges.iter().map(|c| serde_json::json!({
                "kind": format!("{:?}", c.kind),
                "severity": format!("{:?}", c.severity),
                "description": c.description,
            })).collect::<Vec<_>>(),
            "verdict": if has_blockers { "blocked" } else { "cleared" },
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                "adversarial:review",
                review.to_string(),
                self.name(),
            )
            .with_confidence(if has_blockers { 0.3 } else { 0.9 }),
        )
    }
}

// ── Stage 4: Simulation ────────────────────────────────────────────

/// Simulates the budget impact using the simulation types re-exported by `organism-pack`.
/// Maps to: autonomous_org::budget_monitor + spend_validator
struct BudgetSimulationAgent;

impl Suggestor for BudgetSimulationAgent {
    fn name(&self) -> &str {
        "budget_simulation"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Evaluations) && !ctx.has(ContextKey::Proposals)
    }

    fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let evaluations = ctx.get(ContextKey::Evaluations);

        let Some(plan_fact) = strategies.first() else {
            return AgentEffect::empty();
        };
        let plan: serde_json::Value = serde_json::from_str(&plan_fact.content).unwrap_or_default();
        let amount = plan.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);

        // Check if adversarial review blocked
        let review = evaluations.iter().find(|e| e.id == "adversarial:review");
        if let Some(r) = review {
            let review_json: serde_json::Value =
                serde_json::from_str(&r.content).unwrap_or_default();
            if review_json.get("verdict").and_then(|v| v.as_str()) == Some("blocked") {
                return AgentEffect::with_proposal(
                    ProposedFact::new(
                        ContextKey::Proposals,
                        "decision:blocked",
                        serde_json::json!({
                            "decision": "rejected",
                            "reason": "adversarial review blocked the plan",
                            "blockers": review_json.get("blockers"),
                        })
                        .to_string(),
                        self.name(),
                    )
                    .with_confidence(0.95),
                );
            }
        }

        // Simulate budget impact across dimensions
        let quarterly_budget = 50_000.0;
        let spent_so_far = 32_000.0;
        let remaining = quarterly_budget - spent_so_far;
        let utilization_after = (spent_so_far + amount) / quarterly_budget;

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
            findings: vec!["Policy 2026-Q2 compliance verified".into()],
            samples: vec![],
        };

        let operational_result = DimensionResult {
            dimension: SimulationDimension::Operational,
            passed: true,
            confidence: 0.85,
            findings: vec!["Approval chain is reachable within SLA".into()],
            samples: vec![],
        };

        let overall =
            (cost_result.confidence + policy_result.confidence + operational_result.confidence)
                / 3.0;
        let recommendation = if !cost_result.passed {
            SimulationRecommendation::DoNotProceed
        } else if utilization_after > 0.9 {
            SimulationRecommendation::ProceedWithCaution
        } else {
            SimulationRecommendation::Proceed
        };

        let sim_result = SimulationResult {
            plan_id: uuid::Uuid::nil(),
            runs: 1,
            dimensions: vec![cost_result, policy_result, operational_result],
            overall_confidence: overall,
            recommendation,
        };

        let decision = match recommendation {
            SimulationRecommendation::Proceed => "approved",
            SimulationRecommendation::ProceedWithCaution => "approved_with_caution",
            SimulationRecommendation::DoNotProceed => "rejected",
        };

        let proposal = serde_json::json!({
            "decision": decision,
            "simulation": {
                "overall_confidence": sim_result.overall_confidence,
                "recommendation": format!("{:?}", sim_result.recommendation),
                "dimensions": sim_result.dimensions.iter().map(|d| serde_json::json!({
                    "dimension": format!("{:?}", d.dimension),
                    "passed": d.passed,
                    "confidence": d.confidence,
                    "findings": d.findings,
                })).collect::<Vec<_>>(),
            },
            "budget_impact": {
                "amount": amount,
                "remaining_before": remaining,
                "remaining_after": remaining - amount,
                "utilization_after_pct": utilization_after * 100.0,
            },
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Proposals,
                "decision:expense",
                proposal.to_string(),
                self.name(),
            )
            .with_confidence(overall),
        )
    }
}

// ── Main ───────────────────────────────────────────────────────────

fn main() {
    println!("=== Organism Expense Approval ===");
    println!("    Intent → Admission → Planning → Adversarial → Simulation → Converge\n");

    // Show organism-domain pack metadata
    println!("Packs used:");
    print_pack(
        "autonomous_org",
        organism_domain::packs::autonomous_org::AGENTS,
        organism_domain::packs::autonomous_org::INVARIANTS,
    );
    print_pack(
        "procurement",
        organism_domain::packs::procurement::AGENTS,
        organism_domain::packs::procurement::INVARIANTS,
    );
    println!();

    // Build the intent
    let intent = IntentPacket::new(
        "Approve entertainment expense for client dinner",
        chrono::Utc::now() + chrono::Duration::hours(24),
    )
    .with_reversibility(organism_pack::Reversibility::Reversible);
    println!("Intent: {}", intent.outcome);
    println!("  reversibility: {:?}", intent.reversibility);
    println!(
        "  expires: {}\n",
        intent.expires.format("%Y-%m-%d %H:%M UTC")
    );

    // Wire the engine with the full organism pipeline
    let mut engine = Engine::new();
    engine.register_suggestor(IntentAdmissionAgent);
    engine.register_suggestor(PolicyPlanningAgent);
    engine.register_suggestor(PolicySkepticAgent);
    engine.register_suggestor(BudgetSimulationAgent);

    // Seed: a $2,500 entertainment expense
    let expense = serde_json::json!({
        "employee": "karl@reflective.se",
        "amount": 2500.00,
        "category": "entertainment",
        "description": "Client dinner — annual review",
        "date": "2026-04-15"
    });

    let mut ctx = converge_kernel::Context::new();
    let _ = ctx.add_input(ContextKey::Seeds, "expense-1", expense.to_string());

    println!(
        "Expense: ${} {} — {}",
        expense["amount"], expense["category"], expense["description"]
    );
    println!("Running pipeline...\n");

    match engine.run(ctx) {
        Ok(result) => {
            // Show admission
            for fact in result.context.get(ContextKey::Signals) {
                if fact.id == "admission:result"
                    && let Ok(admission) = serde_json::from_str::<serde_json::Value>(&fact.content)
                {
                    let feasible = admission
                        .get("feasible")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    println!(
                        "[Admission] {} ",
                        if feasible { "ADMITTED" } else { "REJECTED" }
                    );
                    if let Some(dims) = admission.get("dimensions").and_then(|v| v.as_array()) {
                        for d in dims {
                            let dim = d.get("dimension").and_then(|v| v.as_str()).unwrap_or("?");
                            let kind = d.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
                            let reason = d.get("reason").and_then(|v| v.as_str()).unwrap_or("");
                            println!("  {dim}: {kind} — {reason}");
                        }
                    }
                    println!();
                }
            }

            // Show planning
            for fact in result.context.get(ContextKey::Strategies) {
                if let Ok(plan) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let rationale = plan
                        .get("routing_rationale")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    println!("[Planning] {rationale}");
                    if let Some(approvers) =
                        plan.get("required_approvers").and_then(|v| v.as_array())
                    {
                        println!(
                            "  approvers: {}",
                            approvers
                                .iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(" → ")
                        );
                    }
                    println!();
                }
            }

            // Show adversarial review
            for fact in result.context.get(ContextKey::Evaluations) {
                if fact.id == "adversarial:review"
                    && let Ok(review) = serde_json::from_str::<serde_json::Value>(&fact.content)
                {
                    let verdict = review
                        .get("verdict")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let challenges = review
                        .get("challenges")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    println!("[Adversarial] {verdict} ({challenges} challenges)");
                    if let Some(details) = review.get("details").and_then(|v| v.as_array()) {
                        for d in details {
                            let kind = d.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
                            let severity =
                                d.get("severity").and_then(|v| v.as_str()).unwrap_or("?");
                            let desc = d.get("description").and_then(|v| v.as_str()).unwrap_or("");
                            println!("  [{severity}] {kind}: {desc}");
                        }
                    }
                    println!();
                }
            }

            // Show decision
            for fact in result.context.get(ContextKey::Proposals) {
                if let Ok(decision) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let outcome = decision
                        .get("decision")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    println!("[Decision] {}", outcome.to_uppercase());
                    if let Some(sim) = decision.get("simulation") {
                        let conf = sim
                            .get("overall_confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let rec = sim
                            .get("recommendation")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        println!("  simulation: {rec} (confidence: {conf:.0}%)");
                        if let Some(dims) = sim.get("dimensions").and_then(|v| v.as_array()) {
                            for d in dims {
                                let dim =
                                    d.get("dimension").and_then(|v| v.as_str()).unwrap_or("?");
                                let passed =
                                    d.get("passed").and_then(|v| v.as_bool()).unwrap_or(false);
                                let findings = d
                                    .get("findings")
                                    .and_then(|v| v.as_array())
                                    .map(|f| {
                                        f.iter()
                                            .filter_map(|v| v.as_str())
                                            .collect::<Vec<_>>()
                                            .join("; ")
                                    })
                                    .unwrap_or_default();
                                println!(
                                    "  {dim}: {} — {findings}",
                                    if passed { "pass" } else { "FAIL" }
                                );
                            }
                        }
                    }
                    if let Some(budget) = decision.get("budget_impact") {
                        let util = budget
                            .get("utilization_after_pct")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        println!("  budget utilization after: {util:.0}%");
                    }
                }
            }
        }
        Err(e) => println!("Failed: {e}"),
    }

    println!("\n=== Done ===");
}

fn print_pack(
    name: &str,
    agents: &[organism_domain::pack::AgentMeta],
    invariants: &[organism_domain::pack::InvariantMeta],
) {
    println!(
        "  {name}: {} agents, {} invariants",
        agents.len(),
        invariants.len()
    );
}
