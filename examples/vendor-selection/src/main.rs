// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Vendor Selection - organism-domain partnerships pack as real Suggestors.
//!
//! Demonstrates: swarm evaluation, multi-criteria scoring, consensus,
//! HITL-ready pack metadata, all wired through converge-kernel.

use converge_kernel::{ContextKey, ContextState, Engine};
use organism_domain::packs::partnerships::{
    VendorComplianceEvaluatorSuggestor, VendorConsensusSuggestor, VendorDataSuggestor,
    VendorPriceEvaluatorSuggestor, VendorRiskEvaluatorSuggestor, VendorTimelineEvaluatorSuggestor,
};

#[tokio::main]
async fn main() {
    println!("=== Organism Vendor Selection ===");
    println!("    (partnerships pack + converge-kernel)\n");

    let pack = organism_domain::packs::partnerships::AGENTS;
    println!("Pack agents ({}):", pack.len());
    for agent in pack {
        println!("  {} - {}", agent.name, agent.description);
    }
    let invariants = organism_domain::packs::partnerships::INVARIANTS;
    println!("Pack invariants ({}):", invariants.len());
    for inv in invariants {
        println!("  {} ({:?}) - {}", inv.name, inv.class, inv.description);
    }
    println!();

    let mut engine = Engine::new();
    engine.register_suggestor(VendorDataSuggestor);
    engine.register_suggestor(VendorPriceEvaluatorSuggestor);
    engine.register_suggestor(VendorComplianceEvaluatorSuggestor);
    engine.register_suggestor(VendorRiskEvaluatorSuggestor);
    engine.register_suggestor(VendorTimelineEvaluatorSuggestor);
    engine.register_suggestor(VendorConsensusSuggestor);

    // In production: set EngineHitlPolicy with confidence_threshold 0.75
    // to enforce partnerships::high_risk_vendor_requires_approval.
    let rfp = serde_json::json!({
        "vendors": [
            {
                "id": "acme",
                "name": "Acme Corp",
                "price": 15000,
                "compliant": true,
                "years_in_business": 15,
                "delivery_weeks": 6
            },
            {
                "id": "beta",
                "name": "Beta Solutions",
                "price": 22000,
                "compliant": true,
                "years_in_business": 8,
                "delivery_weeks": 4
            },
            {
                "id": "gamma",
                "name": "Gamma Industries",
                "price": 8000,
                "compliant": false,
                "years_in_business": 3,
                "delivery_weeks": 10
            }
        ]
    });

    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "rfp-1", rfp.to_string());

    println!("Evaluating 3 vendors across 4 criteria...\n");

    match engine.run(ctx).await {
        Ok(result) => {
            println!("Converged.\n");
            for fact in result.context.get(ContextKey::Proposals) {
                if let Ok(p) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let rank = p
                        .get("rank")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let vendor = p
                        .get("vendor_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("?");
                    let score = p
                        .get("score")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0);
                    let rec = p
                        .get("recommendation")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("?");
                    println!("  #{rank}. {vendor} (score: {score:.2}) - {rec}");
                }
            }
        }
        Err(e) => {
            println!("Failed: {e}");
        }
    }

    println!("\n=== Done ===");
}
