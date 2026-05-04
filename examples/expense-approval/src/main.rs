// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Expense Approval — organism planning loop + converge-kernel.
//!
//! Demonstrates the FULL organism pipeline:
//!   Intent → Admission → Planning → Adversarial Review → Simulation → Converge
//!
//! Uses organism-domain autonomous_org + procurement packs.
//! Uses `organism-pack` as the curated planning contract.

use converge_kernel::{ContextKey, Engine};
use organism_domain::packs::autonomous_org::{
    ApprovalPolicySkepticSuggestor, ApprovalRoutingSuggestor, BudgetSimulationSuggestor,
    SpendAdmissionSuggestor,
};
use organism_pack::{IntentPacket, Reversibility};

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
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
    .with_reversibility(Reversibility::Reversible);
    println!("Intent: {}", intent.outcome);
    println!("  reversibility: {:?}", intent.reversibility);
    println!(
        "  expires: {}\n",
        intent.expires.format("%Y-%m-%d %H:%M UTC")
    );

    // Wire the engine with the full organism pipeline
    let mut engine = Engine::new();
    engine.register_suggestor(SpendAdmissionSuggestor);
    engine.register_suggestor(ApprovalRoutingSuggestor);
    engine.register_suggestor(ApprovalPolicySkepticSuggestor);
    engine.register_suggestor(BudgetSimulationSuggestor::default());

    // Seed: a $2,500 entertainment expense
    let expense = serde_json::json!({
        "employee": "karl@reflective.se",
        "amount": 2500.00,
        "category": "entertainment",
        "description": "Client dinner — annual review",
        "date": "2026-04-15"
    });

    let mut ctx = converge_kernel::ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "expense-1", expense.to_string());

    let amount = expense
        .get("amount")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let category = expense
        .get("category")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    let description = expense
        .get("description")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("?");
    println!("Expense: ${amount:.2} {category} — {description}");
    println!("Running pipeline...\n");

    match engine.run(ctx).await {
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
                        println!("  simulation: {rec} (confidence: {:.0}%)", conf * 100.0);
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
