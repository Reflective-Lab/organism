// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Loan Application — parallel verification with organism planning loop + learning.
//!
//! Full organism pipeline:
//!   Intent → Admission → Parallel Evaluation → Adversarial Review
//!   → Simulation (5 dimensions) → Decision → Learning Signal
//!
//! Uses organism-domain customers + legal packs.
//! Demonstrates: parallel agents, all 5 skepticism kinds, 5 simulation dimensions,
//! and learning episode capture.

use converge_kernel::{AgentEffect, ContextKey, Engine, ProposedFact, Suggestor};
use converge_pack::Context as ContextView;

use organism_pack::{
    // Intent
    AdmissionResult,
    // Adversarial
    Challenge,
    // Simulation
    DimensionResult,
    ErrorDimension,
    FeasibilityAssessment,
    FeasibilityDimension,
    FeasibilityKind,
    // Learning
    LearningEpisode,
    Lesson,
    PredictionError,
    Sample,
    Severity,
    SimulationDimension,
    SimulationRecommendation,
    SkepticismKind,
};

// ── Stage 1: Admission ─────────────────────────────────────────────

struct LoanAdmissionAgent;

#[async_trait::async_trait]
impl Suggestor for LoanAdmissionAgent {
    fn name(&self) -> &str {
        "loan_admission"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Signals)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let Some(seed) = seeds.first() else {
            return AgentEffect::empty();
        };
        let app: serde_json::Value = serde_json::from_str(&seed.content).unwrap_or_default();

        let amount = app
            .get("requested_amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let has_docs = app
            .get("documents")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let age = app.get("age").and_then(|v| v.as_u64()).unwrap_or(0);

        let admission = AdmissionResult {
            feasible: has_docs && age >= 18 && amount > 0.0,
            dimensions: vec![
                FeasibilityAssessment {
                    dimension: FeasibilityDimension::Capability,
                    kind: FeasibilityKind::Feasible,
                    reason: "loan processing available".into(),
                },
                FeasibilityAssessment {
                    dimension: FeasibilityDimension::Context,
                    kind: if has_docs {
                        FeasibilityKind::Feasible
                    } else {
                        FeasibilityKind::Infeasible
                    },
                    reason: if has_docs {
                        "documents provided".into()
                    } else {
                        "missing documents".into()
                    },
                },
                FeasibilityAssessment {
                    dimension: FeasibilityDimension::Resources,
                    kind: if amount <= 1_000_000.0 {
                        FeasibilityKind::Feasible
                    } else {
                        FeasibilityKind::Uncertain
                    },
                    reason: format!("requested: ${amount:.0}"),
                },
                FeasibilityAssessment {
                    dimension: FeasibilityDimension::Authority,
                    kind: if age >= 18 {
                        FeasibilityKind::Feasible
                    } else {
                        FeasibilityKind::Infeasible
                    },
                    reason: format!("applicant age: {age}"),
                },
            ],
            rejection_reason: None,
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
        if admission.feasible {
            facts.push(
                ProposedFact::new(
                    ContextKey::Signals,
                    "application:parsed",
                    seed.content.clone(),
                    self.name(),
                )
                .with_confidence(1.0),
            );
        }
        AgentEffect::with_proposals(facts)
    }
}

// ── Stage 2: Parallel Evaluation (4 agents run in parallel) ────────

struct CreditCheckAgent;

#[async_trait::async_trait]
impl Suggestor for CreditCheckAgent {
    fn name(&self) -> &str {
        "credit_check"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let app = get_application(ctx);
        let credit = app
            .get("credit_score")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let income = app.get("income").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let requested = app
            .get("requested_amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let credit_score = if credit >= 750 {
            1.0
        } else if credit >= 700 {
            0.8
        } else if credit >= 650 {
            0.6
        } else if credit >= 600 {
            0.4
        } else {
            0.1
        };
        let dti = if income > 0.0 {
            requested / income
        } else {
            99.0
        };
        let dti_score = if dti < 0.2 {
            1.0
        } else if dti < 0.3 {
            0.8
        } else if dti < 0.4 {
            0.5
        } else {
            0.2
        };

        emit_evaluation(
            "credit",
            (credit_score + dti_score) / 2.0,
            self.name(),
            serde_json::json!({ "credit_score": credit, "dti_ratio": format!("{dti:.2}") }),
        )
    }
}

struct DocumentVerificationAgent;

#[async_trait::async_trait]
impl Suggestor for DocumentVerificationAgent {
    fn name(&self) -> &str {
        "document_verification"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let app = get_application(ctx);
        let docs = app
            .get("documents")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        emit_evaluation(
            "documents",
            if docs { 1.0 } else { 0.0 },
            self.name(),
            serde_json::json!({ "complete": docs }),
        )
    }
}

struct ComplianceCheckAgent;

#[async_trait::async_trait]
impl Suggestor for ComplianceCheckAgent {
    fn name(&self) -> &str {
        "compliance_check"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let app = get_application(ctx);
        let citizen = app
            .get("us_citizen")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let bankruptcies = app
            .get("bankruptcies")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let score = if citizen && bankruptcies == 0 {
            1.0
        } else if !citizen {
            0.0
        } else {
            0.3
        };
        emit_evaluation(
            "compliance",
            score,
            self.name(),
            serde_json::json!({ "citizen": citizen, "bankruptcies": bankruptcies }),
        )
    }
}

struct RiskAssessmentAgent;

#[async_trait::async_trait]
impl Suggestor for RiskAssessmentAgent {
    fn name(&self) -> &str {
        "risk_assessment"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let app = get_application(ctx);
        let years = app
            .get("employment_years")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let requested = app
            .get("requested_amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let score = if years >= 5 && requested < 500_000.0 {
            1.0
        } else if years >= 2 {
            0.7
        } else {
            0.3
        };
        emit_evaluation(
            "risk",
            score,
            self.name(),
            serde_json::json!({ "employment_years": years, "amount": requested }),
        )
    }
}

// ── Stage 3: Adversarial Review (all 5 skepticism kinds) ───────────

struct LoanSkepticAgent;

#[async_trait::async_trait]
impl Suggestor for LoanSkepticAgent {
    fn name(&self) -> &str {
        "loan_skeptic"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations, ContextKey::Signals]
    }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Evaluations) && !ctx.has(ContextKey::Strategies)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let app = get_application(ctx);
        let requested = app
            .get("requested_amount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mut challenges: Vec<Challenge> = Vec::new();
        let mut avg_score = 0.0;
        let mut count = 0;

        for eval in evaluations {
            if let Ok(e) = serde_json::from_str::<serde_json::Value>(&eval.content) {
                let score = e.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                avg_score += score;
                count += 1;

                if score < 0.4 {
                    let criterion = e.get("criterion").and_then(|v| v.as_str()).unwrap_or("?");
                    challenges.push(Challenge::new(
                        SkepticismKind::ConstraintChecking,
                        uuid::Uuid::nil(),
                        format!("{criterion} score {score:.2} is below threshold"),
                        Severity::Blocker,
                    ));
                }
            }
        }
        if count > 0 {
            avg_score /= count as f64;
        }

        // Assumption breaking
        if avg_score > 0.6 && avg_score < 0.8 {
            challenges.push(Challenge::new(
                SkepticismKind::AssumptionBreaking, uuid::Uuid::nil(),
                format!("borderline average ({avg_score:.2}) — assumes all evaluations are equally weighted"), Severity::Warning,
            ));
        }

        // Causal skepticism
        if requested > 200_000.0 {
            challenges.push(Challenge::new(
                SkepticismKind::CausalSkepticism, uuid::Uuid::nil(),
                "high loan amount may not predict repayment ability — credit score alone is insufficient", Severity::Advisory,
            ));
        }

        // Economic skepticism
        let income = app.get("income").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if income > 0.0 && requested / income > 3.0 {
            challenges.push(Challenge::new(
                SkepticismKind::EconomicSkepticism,
                uuid::Uuid::nil(),
                format!(
                    "loan-to-income ratio {:.1}x exceeds safe threshold",
                    requested / income
                ),
                Severity::Warning,
            ));
        }

        // Operational skepticism
        if count < 4 {
            challenges.push(Challenge::new(
                SkepticismKind::OperationalSkepticism,
                uuid::Uuid::nil(),
                format!("only {count}/4 evaluation dimensions completed — incomplete picture"),
                Severity::Blocker,
            ));
        }

        let has_blockers = challenges.iter().any(|c| c.severity == Severity::Blocker);
        let review = serde_json::json!({
            "average_score": avg_score,
            "verdict": if has_blockers { "blocked" } else { "cleared" },
            "challenges": challenges.iter().map(|c| serde_json::json!({
                "kind": format!("{:?}", c.kind),
                "severity": format!("{:?}", c.severity),
                "description": c.description,
            })).collect::<Vec<_>>(),
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Strategies,
                "adversarial:review",
                review.to_string(),
                self.name(),
            )
            .with_confidence(if has_blockers { 0.3 } else { 0.85 }),
        )
    }
}

// ── Stage 4: Simulation (5 dimensions) + Decision + Learning ───────

struct LoanDecisionAgent;

#[async_trait::async_trait]
impl Suggestor for LoanDecisionAgent {
    fn name(&self) -> &str {
        "loan_decision"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }
    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let Some(review_fact) = strategies.first() else {
            return AgentEffect::empty();
        };
        let review: serde_json::Value =
            serde_json::from_str(&review_fact.content).unwrap_or_default();

        let avg_score = review
            .get("average_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let verdict = review
            .get("verdict")
            .and_then(|v| v.as_str())
            .unwrap_or("blocked");

        // Simulate across 5 dimensions
        let dimensions = [
            DimensionResult {
                dimension: SimulationDimension::Outcome,
                passed: avg_score >= 0.6,
                confidence: avg_score,
                findings: vec![format!("average evaluation: {avg_score:.2}")],
                samples: vec![Sample {
                    value: avg_score,
                    probability: 0.9,
                }],
            },
            DimensionResult {
                dimension: SimulationDimension::Cost,
                passed: true,
                confidence: 0.95,
                findings: vec!["processing cost within budget".into()],
                samples: vec![],
            },
            DimensionResult {
                dimension: SimulationDimension::Policy,
                passed: verdict != "blocked",
                confidence: 0.9,
                findings: vec![format!("adversarial verdict: {verdict}")],
                samples: vec![],
            },
            DimensionResult {
                dimension: SimulationDimension::Causal,
                passed: avg_score >= 0.7,
                confidence: 0.75,
                findings: vec!["historical repayment correlation with score range".into()],
                samples: vec![Sample {
                    value: avg_score * 0.95,
                    probability: 0.75,
                }],
            },
            DimensionResult {
                dimension: SimulationDimension::Operational,
                passed: true,
                confidence: 0.85,
                findings: vec!["underwriting capacity available".into()],
                samples: vec![],
            },
        ];

        let overall =
            dimensions.iter().map(|d| d.confidence).sum::<f64>() / dimensions.len() as f64;
        let all_passed = dimensions.iter().all(|d| d.passed);

        let recommendation = if all_passed && overall >= 0.75 {
            SimulationRecommendation::Proceed
        } else if all_passed {
            SimulationRecommendation::ProceedWithCaution
        } else {
            SimulationRecommendation::DoNotProceed
        };

        let decision = match recommendation {
            SimulationRecommendation::Proceed => "approved",
            SimulationRecommendation::ProceedWithCaution => "borderline",
            SimulationRecommendation::DoNotProceed => "rejected",
        };

        // Capture learning episode
        let episode = LearningEpisode {
            id: uuid::Uuid::new_v4(),
            intent_id: uuid::Uuid::nil(),
            plan_id: uuid::Uuid::nil(),
            predicted_outcome: format!("{decision} (confidence: {overall:.2})"),
            actual_outcome: None,
            prediction_error: Some(PredictionError {
                magnitude: 0.0,
                dimensions: vec![ErrorDimension {
                    name: "score".into(),
                    predicted: avg_score,
                    actual: 0.0,
                }],
            }),
            adversarial_signals: vec![],
            lessons: vec![Lesson {
                insight: format!("score {avg_score:.2} → {decision}"),
                context: "loan underwriting".into(),
                confidence: overall,
                planning_adjustment: if decision == "borderline" {
                    "consider weighted scoring".into()
                } else {
                    "none".into()
                },
            }],
        };

        let proposal = serde_json::json!({
            "decision": decision,
            "simulation": {
                "overall_confidence": overall,
                "recommendation": format!("{recommendation:?}"),
                "dimensions": dimensions.iter().map(|d| serde_json::json!({
                    "dimension": format!("{:?}", d.dimension),
                    "passed": d.passed,
                    "confidence": d.confidence,
                    "findings": d.findings,
                })).collect::<Vec<_>>(),
            },
            "learning": {
                "episode_id": episode.id.to_string(),
                "predicted": episode.predicted_outcome,
                "lessons": episode.lessons.iter().map(|l| l.insight.clone()).collect::<Vec<_>>(),
            },
        });

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Proposals,
                "decision:loan",
                proposal.to_string(),
                self.name(),
            )
            .with_confidence(overall),
        )
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn get_application(ctx: &dyn ContextView) -> serde_json::Value {
    ctx.get(ContextKey::Signals)
        .iter()
        .find(|s| s.id == "application:parsed")
        .and_then(|s| serde_json::from_str(&s.content).ok())
        .unwrap_or_default()
}

fn emit_evaluation(
    criterion: &str,
    score: f64,
    provenance: &str,
    details: serde_json::Value,
) -> AgentEffect {
    AgentEffect::with_proposal(
        ProposedFact::new(
            ContextKey::Evaluations,
            format!("eval:{criterion}"),
            serde_json::json!({ "criterion": criterion, "score": score, "details": details })
                .to_string(),
            provenance,
        )
        .with_confidence(1.0),
    )
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("=== Organism Loan Application ===");
    println!("    Intent → Admission → Parallel Eval → Adversarial → Simulation (5D) → Learning\n");

    // Show packs
    println!(
        "Packs: customers ({} agents), legal ({} agents)",
        organism_domain::packs::customers::AGENTS.len(),
        organism_domain::packs::legal::AGENTS.len(),
    );
    println!();

    let mut engine = Engine::new();
    engine.register_suggestor(LoanAdmissionAgent);
    engine.register_suggestor(CreditCheckAgent);
    engine.register_suggestor(DocumentVerificationAgent);
    engine.register_suggestor(ComplianceCheckAgent);
    engine.register_suggestor(RiskAssessmentAgent);
    engine.register_suggestor(LoanSkepticAgent);
    engine.register_suggestor(LoanDecisionAgent);

    let application = serde_json::json!({
        "applicant": "Jane Smith",
        "requested_amount": 250_000,
        "credit_score": 720,
        "income": 85_000,
        "documents": true,
        "us_citizen": true,
        "age": 35,
        "bankruptcies": 0,
        "employment_years": 5
    });

    let mut ctx = converge_kernel::Context::new();
    let _ = ctx.add_input(ContextKey::Seeds, "app-1", application.to_string());

    println!(
        "Applicant: {} — ${} loan, credit {}, income ${}",
        application["applicant"],
        application["requested_amount"],
        application["credit_score"],
        application["income"]
    );
    println!("Running 7-agent pipeline...\n");

    match engine.run(ctx).await {
        Ok(result) => {
            // Admission
            for fact in result.context.get(ContextKey::Signals) {
                if fact.id == "admission:result"
                    && let Ok(a) = serde_json::from_str::<serde_json::Value>(&fact.content)
                {
                    println!(
                        "[Admission] {}",
                        if a["feasible"].as_bool().unwrap_or(false) {
                            "ADMITTED"
                        } else {
                            "REJECTED"
                        }
                    );
                }
            }

            // Evaluations
            println!("[Evaluations]");
            for fact in result.context.get(ContextKey::Evaluations) {
                if let Ok(e) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let c = e["criterion"].as_str().unwrap_or("?");
                    let s = e["score"].as_f64().unwrap_or(0.0);
                    println!("  {c}: {s:.2}");
                }
            }

            // Adversarial
            for fact in result.context.get(ContextKey::Strategies) {
                if let Ok(r) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let verdict = r["verdict"].as_str().unwrap_or("?");
                    println!("[Adversarial] {verdict}");
                    if let Some(challenges) = r["challenges"].as_array() {
                        for c in challenges {
                            let kind = c["kind"].as_str().unwrap_or("?");
                            let sev = c["severity"].as_str().unwrap_or("?");
                            let desc = c["description"].as_str().unwrap_or("");
                            println!("  [{sev}] {kind}: {desc}");
                        }
                    }
                }
            }

            // Decision
            println!();
            for fact in result.context.get(ContextKey::Proposals) {
                if let Ok(d) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let decision = d["decision"].as_str().unwrap_or("?");
                    println!("[Decision] {}", decision.to_uppercase());
                    if let Some(sim) = d.get("simulation") {
                        let rec = sim["recommendation"].as_str().unwrap_or("?");
                        let conf = sim["overall_confidence"].as_f64().unwrap_or(0.0);
                        println!("  simulation: {rec} (confidence: {conf:.2})");
                        if let Some(dims) = sim["dimensions"].as_array() {
                            for dim in dims {
                                let name = dim["dimension"].as_str().unwrap_or("?");
                                let passed = dim["passed"].as_bool().unwrap_or(false);
                                println!("  {name}: {}", if passed { "pass" } else { "FAIL" });
                            }
                        }
                    }
                    if let Some(learning) = d.get("learning") {
                        println!(
                            "[Learning] {}",
                            learning["predicted"].as_str().unwrap_or("?")
                        );
                        if let Some(lessons) = learning["lessons"].as_array() {
                            for l in lessons {
                                println!("  lesson: {}", l.as_str().unwrap_or(""));
                            }
                        }
                    }
                }
            }
        }
        Err(e) => println!("Failed: {e}"),
    }

    println!("\n=== Done ===");
}
