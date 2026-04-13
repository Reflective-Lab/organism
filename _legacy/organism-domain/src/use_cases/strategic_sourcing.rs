// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Strategic Sourcing / Vendor Selection agents for supplier evaluation.
//!
//! This module implements vendor selection under cost, ESG, risk, and compliance constraints,
//! demonstrating wide fan-out (many suppliers) and narrow fan-in (shortlist consolidation).
//!
//! # Agent Pipeline
//!
//! ```text
//! Seeds (requirements, suppliers)
//!    │
//!    ├─► SupplierDiscoveryAgent → Signals (supplier profiles)
//!    ├─► ComplianceAgent → Signals (compliance status)
//!    ├─► ESGScoringAgent → Signals (ESG scores)
//!    ├─► PriceBenchmarkAgent → Signals (price comparisons)
//!    └─► RiskModelAgent → Signals (risk assessments)
//!    │
//!    ▼
//! SourcingStrategyAgent → Strategies (shortlisted suppliers)
//!    │
//!    ▼
//! VendorRankingAgent → Evaluations (ranked vendors with rationale)
//! ```

// Agent trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Agent that discovers and profiles suppliers.
pub struct SupplierDiscoveryAgent;

impl Agent for SupplierDiscoveryAgent {
    fn name(&self) -> &str {
        "SupplierDiscoveryAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("supplier:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        // Extract suppliers from seeds or use defaults
        let suppliers = if let Some(suppliers_seed) = seeds.iter().find(|s| s.id == "suppliers") {
            suppliers_seed
                .content
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>()
        } else {
            vec!["VendorA", "VendorB", "VendorC", "VendorD", "VendorE"]
        };

        for (i, supplier) in suppliers.iter().enumerate() {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("supplier:{}", supplier.to_lowercase()),
                content: format!(
                    "Supplier {}: {} | Location: {} | Capacity: {} units/month | Established: {}",
                    i + 1,
                    supplier,
                    if i % 2 == 0 {
                        "Domestic"
                    } else {
                        "International"
                    },
                    (i + 1) * 100,
                    2020 - (i * 2)
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that checks compliance status.
pub struct ComplianceAgent;

impl Agent for ComplianceAgent {
    fn name(&self) -> &str {
        "ComplianceAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("supplier:"))
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("compliance:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for supplier_signal in signals.iter().filter(|s| s.id.starts_with("supplier:")) {
            let supplier = supplier_signal
                .id
                .strip_prefix("supplier:")
                .unwrap_or("unknown");

            // Simulate compliance checks
            let is_compliant = !supplier.contains("vendorc"); // VendorC fails
            let certifications = if is_compliant {
                "ISO 9001, ISO 14001, SOC 2"
            } else {
                "ISO 9001 (expired)"
            };

            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("compliance:{supplier}"),
                content: format!(
                    "Compliance {}: {} | Certifications: {} | Status: {}",
                    supplier,
                    if is_compliant {
                        "COMPLIANT"
                    } else {
                        "NON-COMPLIANT"
                    },
                    certifications,
                    if is_compliant { "PASS" } else { "FAIL" }
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that scores ESG (Environmental, Social, Governance) factors.
pub struct ESGScoringAgent;

impl Agent for ESGScoringAgent {
    fn name(&self) -> &str {
        "ESGScoringAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("supplier:"))
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("esg:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for supplier_signal in signals.iter().filter(|s| s.id.starts_with("supplier:")) {
            let supplier = supplier_signal
                .id
                .strip_prefix("supplier:")
                .unwrap_or("unknown");

            // Simulate ESG scoring (0-100)
            let esg_score = match supplier {
                s if s.contains("vendora") => 95,
                s if s.contains("vendorb") => 85,
                s if s.contains("vendorc") => 60,
                s if s.contains("vendord") => 75,
                _ => 70,
            };

            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("esg:{supplier}"),
                content: format!(
                    "ESG {}: Score {}/100 | Environmental: {} | Social: {} | Governance: {}",
                    supplier,
                    esg_score,
                    esg_score - 10,
                    esg_score - 5,
                    esg_score
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that benchmarks prices.
pub struct PriceBenchmarkAgent;

impl Agent for PriceBenchmarkAgent {
    fn name(&self) -> &str {
        "PriceBenchmarkAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("supplier:"))
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("price:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        // Calculate market average
        let _supplier_count = signals
            .iter()
            .filter(|s| s.id.starts_with("supplier:"))
            .count();
        let market_avg = 100; // Base price

        for (i, supplier_signal) in signals
            .iter()
            .filter(|s| s.id.starts_with("supplier:"))
            .enumerate()
        {
            let supplier = supplier_signal
                .id
                .strip_prefix("supplier:")
                .unwrap_or("unknown");

            // Simulate pricing (some below, some above market)
            #[allow(clippy::cast_possible_wrap)]
            let price = market_avg + ((i as i32 - 2) * 10);
            let vs_market = match price.cmp(&market_avg) {
                std::cmp::Ordering::Less => format!(
                    "{}% below market",
                    ((market_avg - price) * 100) / market_avg
                ),
                std::cmp::Ordering::Greater => format!(
                    "{}% above market",
                    ((price - market_avg) * 100) / market_avg
                ),
                std::cmp::Ordering::Equal => "at market".to_string(),
            };

            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("price:{supplier}"),
                content: format!(
                    "Price {supplier}: ${price}/unit | {vs_market} | Payment terms: Net 30"
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that assesses risk factors.
pub struct RiskModelAgent;

impl Agent for RiskModelAgent {
    fn name(&self) -> &str {
        "RiskModelAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("supplier:"))
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("risk:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for supplier_signal in signals.iter().filter(|s| s.id.starts_with("supplier:")) {
            let supplier = supplier_signal
                .id
                .strip_prefix("supplier:")
                .unwrap_or("unknown");

            // Extract location to assess risk
            let is_international = supplier_signal.content.contains("International");
            let risk_score = if is_international { 40 } else { 20 };
            let risk_level = if risk_score > 30 { "MEDIUM" } else { "LOW" };

            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("risk:{supplier}"),
                content: format!(
                    "Risk {}: Score {}/100 | Level: {} | Factors: {} | Mitigation: Required",
                    supplier,
                    risk_score,
                    risk_level,
                    if is_international {
                        "Currency, logistics, geopolitical"
                    } else {
                        "Minimal"
                    }
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that synthesizes sourcing strategy (simulated LLM reasoning).
pub struct SourcingStrategyAgent;

impl Agent for SourcingStrategyAgent {
    fn name(&self) -> &str {
        "SourcingStrategyAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let signals = ctx.get(ContextKey::Signals);
        let has_compliance = signals.iter().any(|s| s.id.starts_with("compliance:"));
        let has_esg = signals.iter().any(|s| s.id.starts_with("esg:"));
        let has_price = signals.iter().any(|s| s.id.starts_with("price:"));
        let has_risk = signals.iter().any(|s| s.id.starts_with("risk:"));

        has_compliance && has_esg && has_price && has_risk && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        // Shortlist suppliers that meet minimum criteria
        let mut candidates = Vec::new();

        for supplier_signal in signals.iter().filter(|s| s.id.starts_with("supplier:")) {
            let supplier = supplier_signal
                .id
                .strip_prefix("supplier:")
                .unwrap_or("unknown");

            let compliance = signals
                .iter()
                .find(|s| s.id == format!("compliance:{supplier}"));
            let esg = signals.iter().find(|s| s.id == format!("esg:{supplier}"));
            let price = signals.iter().find(|s| s.id == format!("price:{supplier}"));
            let _risk = signals.iter().find(|s| s.id == format!("risk:{supplier}"));

            let is_compliant = compliance.is_some_and(|c| c.content.contains("COMPLIANT"));
            let esg_score = esg
                .and_then(|e| {
                    e.content
                        .split("Score ")
                        .nth(1)
                        .and_then(|s| s.split('/').next())
                        .and_then(|s| s.parse::<u32>().ok())
                })
                .unwrap_or(0);
            let price_val = price
                .and_then(|p| {
                    p.content
                        .split('$')
                        .nth(1)
                        .and_then(|s| s.split('/').next())
                        .and_then(|s| s.parse::<u32>().ok())
                })
                .unwrap_or(1000);

            // Shortlist criteria: compliant, ESG > 60, price reasonable
            if is_compliant && esg_score > 60 && price_val < 120 {
                let score = (esg_score * 2) + (120 - price_val) + 50; // Composite score
                candidates.push((supplier.to_string(), score, esg_score, price_val));
            }
        }

        // Sort by score and take top 3
        candidates.sort_by_key(|(_, score, _, _)| std::cmp::Reverse(*score));
        candidates.truncate(3);

        for (i, (supplier, score, esg, price)) in candidates.iter().enumerate() {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: format!("shortlist:{}", i + 1),
                content: format!(
                    "Shortlist {}: {} | Composite score: {} | ESG: {} | Price: ${}",
                    i + 1,
                    supplier,
                    score,
                    esg,
                    price
                ),
            });
        }

        if facts.is_empty() {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "shortlist:none".into(),
                content: "No suppliers meet minimum criteria".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that ranks vendors and provides rationale.
pub struct VendorRankingAgent;

impl Agent for VendorRankingAgent {
    fn name(&self) -> &str {
        "VendorRankingAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut facts = Vec::new();

        let shortlisted: Vec<_> = strategies
            .iter()
            .filter(|s| {
                s.id.starts_with("shortlist:")
                    && s.id != "shortlist:none"
                    && !s.content.contains("No suppliers")
            })
            .collect();

        if shortlisted.is_empty() {
            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: "eval:no-vendors".into(),
                content: "Status: NO VENDORS FOUND | No suppliers meet all criteria | Recommendation: Relax constraints or expand search".into(),
            });
        } else {
            for (i, shortlist) in shortlisted.iter().enumerate() {
                // Extract supplier name
                let supplier = shortlist
                    .content
                    .split(": ")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap_or("unknown");

                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("eval:{}", i + 1),
                    content: format!(
                        "Rank {}: {} | {} | Rationale: {} | {}",
                        i + 1,
                        supplier,
                        shortlist.content,
                        if i == 0 {
                            "Best overall balance of ESG, price, and compliance"
                        } else {
                            "Strong alternative option"
                        },
                        if i == 0 { "RECOMMENDED" } else { "ALTERNATIVE" }
                    ),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// =============================================================================
// STRATEGIC SOURCING INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: At least one compliant vendor must be shortlisted.
pub struct RequireCompliantVendor;

impl Invariant for RequireCompliantVendor {
    fn name(&self) -> &str {
        "require_compliant_vendor"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_recommended = evaluations
            .iter()
            .any(|e| e.content.contains("RECOMMENDED") && !e.content.contains("NO VENDORS"));

        if !has_recommended {
            return InvariantResult::Violated(Violation::new(
                "no compliant vendors found in shortlist",
            ));
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: All shortlisted vendors must be compliant.
pub struct RequireShortlistCompliance;

impl Invariant for RequireShortlistCompliance {
    fn name(&self) -> &str {
        "require_shortlist_compliance"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let signals = ctx.get(ContextKey::Signals);

        for shortlist in strategies.iter().filter(|s| s.id.starts_with("shortlist:")) {
            let supplier = shortlist
                .content
                .split(": ")
                .nth(1)
                .and_then(|s| s.split(' ').next())
                .unwrap_or("unknown");

            let compliance = signals
                .iter()
                .find(|s| s.id == format!("compliance:{}", supplier.to_lowercase()));

            if let Some(comp) = compliance {
                if comp.content.contains("NON-COMPLIANT") || comp.content.contains("FAIL") {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("shortlisted vendor {supplier} is non-compliant"),
                        vec![shortlist.id.clone()],
                    ));
                }
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: All suppliers must have complete assessments.
pub struct RequireCompleteAssessments;

impl Invariant for RequireCompleteAssessments {
    fn name(&self) -> &str {
        "require_complete_assessments"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);

        // Only check if assessments have started
        let has_any_assessment = signals.iter().any(|s| {
            s.id.starts_with("compliance:")
                || s.id.starts_with("esg:")
                || s.id.starts_with("price:")
                || s.id.starts_with("risk:")
        });

        if !has_any_assessment {
            return InvariantResult::Ok; // Too early, skip check
        }

        let suppliers: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("supplier:"))
            .map(|s| s.id.strip_prefix("supplier:").unwrap_or("unknown"))
            .collect();

        for supplier in suppliers {
            let required = ["compliance:", "esg:", "price:", "risk:"];
            for prefix in &required {
                let has_assessment = signals
                    .iter()
                    .any(|s| s.id == format!("{prefix}{supplier}"));
                if !has_assessment {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!(
                            "supplier {} missing {} assessment",
                            supplier,
                            prefix.strip_suffix(':').unwrap_or(prefix)
                        ),
                        vec![format!("supplier:{}", supplier)],
                    ));
                }
            }
        }

        InvariantResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Engine;
    use converge_core::agents::SeedAgent;

    #[test]
    fn parallel_assessment_agents_run_independently() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("suppliers", "VendorA, VendorB, VendorC"));
        engine.register(SupplierDiscoveryAgent);
        engine.register(ComplianceAgent);
        engine.register(ESGScoringAgent);
        engine.register(PriceBenchmarkAgent);
        engine.register(RiskModelAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("compliance:")));
        assert!(signals.iter().any(|s| s.id.starts_with("esg:")));
        assert!(signals.iter().any(|s| s.id.starts_with("price:")));
        assert!(signals.iter().any(|s| s.id.starts_with("risk:")));
    }

    #[test]
    fn sourcing_strategy_waits_for_all_assessments() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("suppliers", "VendorA, VendorB"));
        engine.register(SupplierDiscoveryAgent);
        engine.register(ComplianceAgent);
        engine.register(ESGScoringAgent);
        engine.register(PriceBenchmarkAgent);
        engine.register(RiskModelAgent);
        engine.register(SourcingStrategyAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));
        let strategies = result.context.get(ContextKey::Strategies);
        assert!(strategies.iter().any(|s| s.id.starts_with("shortlist:")));
    }

    #[test]
    fn vendor_ranking_provides_rationale() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("suppliers", "VendorA, VendorB, VendorD"));
        engine.register(SupplierDiscoveryAgent);
        engine.register(ComplianceAgent);
        engine.register(ESGScoringAgent);
        engine.register(PriceBenchmarkAgent);
        engine.register(RiskModelAgent);
        engine.register(SourcingStrategyAgent);
        engine.register(VendorRankingAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));
        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
        assert!(
            evals
                .iter()
                .any(|e| e.content.contains("RECOMMENDED") || e.content.contains("Rationale"))
        );
    }

    #[test]
    fn invariants_enforce_quality() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("suppliers", "VendorA, VendorB"));
        engine.register(SupplierDiscoveryAgent);
        engine.register(ComplianceAgent);
        engine.register(ESGScoringAgent);
        engine.register(PriceBenchmarkAgent);
        engine.register(RiskModelAgent);
        engine.register(SourcingStrategyAgent);
        engine.register(VendorRankingAgent);

        engine.register_invariant(RequireCompliantVendor);
        engine.register_invariant(RequireShortlistCompliance);
        engine.register_invariant(RequireCompleteAssessments);

        let result = engine.run(Context::new());

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.converged);
    }

    #[test]
    fn handles_no_qualified_vendors() {
        let mut engine = Engine::new();
        // Use only VendorC which fails compliance
        engine.register(SeedAgent::new("suppliers", "VendorC"));
        engine.register(SupplierDiscoveryAgent);
        engine.register(ComplianceAgent);
        engine.register(ESGScoringAgent);
        engine.register(PriceBenchmarkAgent);
        engine.register(RiskModelAgent);
        engine.register(SourcingStrategyAgent);
        engine.register(VendorRankingAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let evals = result.context.get(ContextKey::Evaluations);
        // Should indicate no vendors found (either NO VENDORS message or empty evaluations)
        assert!(evals.is_empty() || evals.iter().any(|e| e.content.contains("NO VENDORS")));
    }

    #[test]
    fn full_pipeline_converges_deterministically() {
        let run = || {
            let mut engine = Engine::new();
            engine.register(SeedAgent::new("suppliers", "VendorA, VendorB, VendorD"));
            engine.register(SupplierDiscoveryAgent);
            engine.register(ComplianceAgent);
            engine.register(ESGScoringAgent);
            engine.register(PriceBenchmarkAgent);
            engine.register(RiskModelAgent);
            engine.register(SourcingStrategyAgent);
            engine.register(VendorRankingAgent);
            engine.run(Context::new()).expect("should converge")
        };

        let r1 = run();
        let r2 = run();

        assert_eq!(r1.cycles, r2.cycles);
        assert_eq!(
            r1.context.get(ContextKey::Evaluations),
            r2.context.get(ContextKey::Evaluations)
        );
    }
}
