// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Vendor Selection — organism-domain partnerships pack as real Suggestors.
//!
//! Demonstrates: swarm evaluation, multi-criteria scoring, consensus,
//! HITL gates, all wired through converge-kernel.
//!
//! This is what organism-domain pack metadata looks like when implemented
//! as real Suggestors running on the Converge engine.

use converge_kernel::{AgentEffect, ContextKey, Engine, ProposedFact, Suggestor};
use converge_pack::Context as ContextView;

// ── Agents ─────────────────────────────────────────────────────────
//
// Each agent maps to an AgentMeta in organism_domain::packs::partnerships.
// The metadata declares the shape; this is the implementation.

/// Parses vendor data from seeds into individual vendor signals.
/// Maps to: partnerships::vendor_assessor
struct VendorDataAgent;

#[async_trait::async_trait]
impl Suggestor for VendorDataAgent {
    fn name(&self) -> &str {
        "vendor_data"
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

        let json: serde_json::Value = serde_json::from_str(&seed.content).unwrap_or_default();
        let vendors = json.get("vendors").cloned().unwrap_or_default();

        let facts: Vec<ProposedFact> = vendors
            .as_array()
            .map_or(&[] as &[serde_json::Value], |v| v)
            .iter()
            .map(|vendor| {
                let id = vendor.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                ProposedFact::new(
                    ContextKey::Signals,
                    format!("vendor:{id}"),
                    vendor.to_string(),
                    "vendor_data",
                )
                .with_confidence(1.0)
            })
            .collect();

        AgentEffect::with_proposals(facts)
    }
}

/// Scores vendors by price tier.
/// Maps to: partnerships::contract_negotiator (price dimension)
struct PriceEvaluator;

#[async_trait::async_trait]
impl Suggestor for PriceEvaluator {
    fn name(&self) -> &str {
        "price_evaluator"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        evaluate_vendors(ctx, "price", |vendor| {
            let price = vendor
                .get("price")
                .and_then(|v| v.as_f64())
                .unwrap_or(999_999.0);
            if price < 10_000.0 {
                1.0
            } else if price < 25_000.0 {
                0.7
            } else if price < 50_000.0 {
                0.4
            } else {
                0.1
            }
        })
    }
}

/// Scores vendors by compliance status.
/// Maps to: partnerships::vendor_assessor (compliance dimension)
struct ComplianceEvaluator;

#[async_trait::async_trait]
impl Suggestor for ComplianceEvaluator {
    fn name(&self) -> &str {
        "compliance_evaluator"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        evaluate_vendors(ctx, "compliance", |vendor| {
            if vendor
                .get("compliant")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
            {
                1.0
            } else {
                0.0
            }
        })
    }
}

/// Scores vendors by years in business (risk proxy).
/// Maps to: partnerships::risk_monitor
struct RiskEvaluator;

#[async_trait::async_trait]
impl Suggestor for RiskEvaluator {
    fn name(&self) -> &str {
        "risk_evaluator"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        evaluate_vendors(ctx, "risk", |vendor| {
            let years = vendor
                .get("years_in_business")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            if years > 10 {
                1.0
            } else if years > 5 {
                0.7
            } else if years > 2 {
                0.4
            } else {
                0.1
            }
        })
    }
}

/// Scores vendors by delivery timeline.
/// Maps to: partnerships::performance_reviewer (timeline dimension)
struct TimelineEvaluator;

#[async_trait::async_trait]
impl Suggestor for TimelineEvaluator {
    fn name(&self) -> &str {
        "timeline_evaluator"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        evaluate_vendors(ctx, "timeline", |vendor| {
            let weeks = vendor
                .get("delivery_weeks")
                .and_then(|v| v.as_u64())
                .unwrap_or(52);
            if weeks <= 4 {
                1.0
            } else if weeks <= 8 {
                0.8
            } else if weeks <= 12 {
                0.5
            } else {
                0.2
            }
        })
    }
}

/// Aggregates all evaluation scores and ranks vendors.
/// Maps to: partnerships::partner_sourcer (consensus/ranking stage)
///
/// Invariant: partnerships::high_risk_vendor_requires_approval
/// enforced via HITL gate when confidence >= 0.75.
struct ConsensusAgent;

#[async_trait::async_trait]
impl Suggestor for ConsensusAgent {
    fn name(&self) -> &str {
        "consensus"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn ContextView) -> bool {
        ctx.has(ContextKey::Evaluations) && !ctx.has(ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn ContextView) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut scores: std::collections::HashMap<String, (f64, u32)> =
            std::collections::HashMap::new();

        for eval in evaluations {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&eval.content) {
                let id = json
                    .get("vendor_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let score = json.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let entry = scores.entry(id.to_string()).or_insert((0.0, 0));
                entry.0 += score;
                entry.1 += 1;
            }
        }

        let mut ranked: Vec<(String, f64)> = scores
            .into_iter()
            .map(|(id, (total, count))| (id, total / count as f64))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let proposals: Vec<ProposedFact> = ranked
            .iter()
            .enumerate()
            .map(|(i, (vendor_id, score))| {
                ProposedFact::new(
                    ContextKey::Proposals,
                    format!("recommendation:{}", i + 1),
                    serde_json::json!({
                        "vendor_id": vendor_id,
                        "rank": i + 1,
                        "score": score,
                        "recommendation": if i == 0 { "recommended" } else { "alternative" }
                    })
                    .to_string(),
                    "consensus",
                )
                .with_confidence(if i == 0 { 0.85 } else { 0.6 })
            })
            .collect();

        AgentEffect::with_proposals(proposals)
    }
}

// ── Shared evaluation helper ───────────────────────────────────────

fn evaluate_vendors<F>(ctx: &dyn ContextView, criterion: &str, scorer: F) -> AgentEffect
where
    F: Fn(&serde_json::Value) -> f64,
{
    let signals = ctx.get(ContextKey::Signals);
    let evaluations: Vec<ProposedFact> = signals
        .iter()
        .filter_map(|signal| {
            let vendor: serde_json::Value = serde_json::from_str(&signal.content).ok()?;
            let id = vendor.get("id").and_then(|v| v.as_str())?;
            let score = scorer(&vendor);
            Some(
                ProposedFact::new(
                    ContextKey::Evaluations,
                    format!("{criterion}:{id}"),
                    serde_json::json!({
                        "vendor_id": id,
                        "criterion": criterion,
                        "score": score,
                    })
                    .to_string(),
                    format!("{criterion}_evaluator"),
                )
                .with_confidence(1.0),
            )
        })
        .collect();

    AgentEffect::with_proposals(evaluations)
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("=== Organism Vendor Selection ===");
    println!("    (partnerships pack + converge-kernel)\n");

    // Print pack metadata from organism-domain
    let pack = &organism_domain::packs::partnerships::AGENTS;
    println!("Pack agents ({}):", pack.len());
    for agent in pack.iter() {
        println!("  {} — {}", agent.name, agent.description);
    }
    let invariants = &organism_domain::packs::partnerships::INVARIANTS;
    println!("Pack invariants ({}):", invariants.len());
    for inv in invariants.iter() {
        println!("  {} ({:?}) — {}", inv.name, inv.class, inv.description);
    }
    println!();

    // Wire the engine
    let mut engine = Engine::new();
    engine.register_suggestor(VendorDataAgent);
    engine.register_suggestor(PriceEvaluator);
    engine.register_suggestor(ComplianceEvaluator);
    engine.register_suggestor(RiskEvaluator);
    engine.register_suggestor(TimelineEvaluator);
    engine.register_suggestor(ConsensusAgent);

    // In production: set EngineHitlPolicy with confidence_threshold 0.75
    // to enforce partnerships::high_risk_vendor_requires_approval invariant.

    // Seed data
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

    let mut ctx = converge_kernel::Context::new();
    let _ = ctx.add_input(ContextKey::Seeds, "rfp-1", rfp.to_string());

    println!("Evaluating 3 vendors across 4 criteria...\n");

    match engine.run(ctx).await {
        Ok(result) => {
            println!("Converged.\n");
            for fact in result.context.get(ContextKey::Proposals) {
                if let Ok(p) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                    let rank = p.get("rank").and_then(|v| v.as_u64()).unwrap_or(0);
                    let vendor = p.get("vendor_id").and_then(|v| v.as_str()).unwrap_or("?");
                    let score = p.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let rec = p
                        .get("recommendation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    println!("  #{rank}. {vendor} (score: {score:.2}) — {rec}");
                }
            }
        }
        Err(e) => {
            println!("Failed: {e}");
        }
    }

    println!("\n=== Done ===");
}
