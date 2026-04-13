// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Supply Chain Re-planning agents for routing, sourcing, and delivery optimization.
//!
//! This module implements a supply chain re-planning use case that demonstrates
//! multiple parallel tracks (routing, sourcing, cost, risk) with consolidation.
//!
//! # Agent Pipeline
//!
//! ```text
//! Seeds (orders, supplier delay)
//!    │
//!    ├─► DemandSnapshotAgent → Signals (order requirements)
//!    ├─► InventoryStateAgent → Signals (stock levels)
//!    └─► SupplierStatusAgent → Signals (supplier availability)
//!    │
//!    ├─► RouteGenerationAgent → Strategies (alternative routes)
//!    ├─► CostEstimationAgent → Strategies (cost analysis)
//!    ├─► RiskAssessmentAgent → Strategies (risk scores)
//!    └─► SLAValidationAgent → Constraints (SLA requirements)
//!    │
//!    ▼
//! ConsolidationAgent → Evaluations (feasible plans ranked)
//! ```
//!
//! # Example
//!
//! ```
//! use converge_core::{Engine, Context, ContextKey};
//! use converge_core::agents::SeedAgent;
//! use converge_domain::supply_chain::{
//!     DemandSnapshotAgent, InventoryStateAgent, SupplierStatusAgent,
//!     RouteGenerationAgent, CostEstimationAgent, RiskAssessmentAgent,
//!     SLAValidationAgent, ConsolidationAgent,
//! };
//!
//! let mut engine = Engine::new();
//! engine.register(SeedAgent::new("orders", "Order A, Order B, Order C"));
//! engine.register(SeedAgent::new("supplier:delay", "Supplier X delayed 3 days"));
//! engine.register(DemandSnapshotAgent);
//! engine.register(InventoryStateAgent);
//! engine.register(SupplierStatusAgent);
//! // ... register all agents
//! let result = engine.run(Context::new()).expect("should converge");
//! ```

// Agent trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Agent that captures current demand snapshot from orders.
pub struct DemandSnapshotAgent;

impl Agent for DemandSnapshotAgent {
    fn name(&self) -> &str {
        "DemandSnapshotAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.id == "orders" || s.content.contains("Order"))
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("demand:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let orders_seed = seeds.iter().find(|s| s.id == "orders");

        if let Some(seed) = orders_seed {
            let order_count = seed.content.matches("Order").count();
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "demand:snapshot".into(),
                content: format!(
                    "Demand snapshot: {} orders | Total units: {} | Priority: High",
                    order_count,
                    order_count * 10
                ),
            });
        } else {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "demand:snapshot".into(),
                content: "Demand snapshot: 3 orders | Total units: 30 | Priority: High".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that reads current inventory state across locations.
pub struct InventoryStateAgent;

impl Agent for InventoryStateAgent {
    fn name(&self) -> &str {
        "InventoryStateAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("inventory:"))
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let facts = vec![
            Fact {
                key: ContextKey::Signals,
                id: "inventory:warehouse-a".into(),
                content: "Warehouse A: 50 units | Location: East | Status: Available".into(),
            },
            Fact {
                key: ContextKey::Signals,
                id: "inventory:warehouse-b".into(),
                content: "Warehouse B: 30 units | Location: West | Status: Available".into(),
            },
            Fact {
                key: ContextKey::Signals,
                id: "inventory:warehouse-c".into(),
                content: "Warehouse C: 20 units | Location: Central | Status: Low stock".into(),
            },
        ];

        AgentEffect::with_facts(facts)
    }
}

/// Agent that monitors supplier status and availability.
pub struct SupplierStatusAgent;

impl Agent for SupplierStatusAgent {
    fn name(&self) -> &str {
        "SupplierStatusAgent"
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

        let has_delay = seeds
            .iter()
            .any(|s| s.id.contains("delay") || s.content.contains("delayed"));

        if has_delay {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "supplier:x".into(),
                content:
                    "Supplier X: DELAYED 3 days | Capacity: 0 | Alternative: Supplier Y available"
                        .into(),
            });
        }

        facts.push(Fact {
            key: ContextKey::Signals,
            id: "supplier:y".into(),
            content: "Supplier Y: Available | Lead time: 2 days | Capacity: 100 units".into(),
        });
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "supplier:z".into(),
            content: "Supplier Z: Available | Lead time: 5 days | Capacity: 50 units".into(),
        });

        AgentEffect::with_facts(facts)
    }
}

/// Agent that generates alternative routing plans.
pub struct RouteGenerationAgent;

impl Agent for RouteGenerationAgent {
    fn name(&self) -> &str {
        "RouteGenerationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let signals = ctx.get(ContextKey::Signals);
        let has_demand = signals.iter().any(|s| s.id.starts_with("demand:"));
        let has_inventory = signals.iter().any(|s| s.id.starts_with("inventory:"));
        let has_supplier = signals.iter().any(|s| s.id.starts_with("supplier:"));

        has_demand && has_inventory && has_supplier && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        // Check for supplier delay
        let has_delay = signals
            .iter()
            .any(|s| s.content.contains("DELAYED") || s.content.contains("delayed"));

        if has_delay {
            // Generate alternative routes
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "route:alternative-1".into(),
                content:
                    "Route 1: Warehouse A → Supplier Y → Delivery | Distance: 200km | Time: 2 days"
                        .into(),
            });
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "route:alternative-2".into(),
                content: "Route 2: Warehouse B → Supplier Y → Delivery | Distance: 150km | Time: 1.5 days".into(),
            });
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "route:alternative-3".into(),
                content: "Route 3: Warehouse A + B → Supplier Y → Delivery | Distance: 180km | Time: 2 days".into(),
            });
        } else {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "route:standard".into(),
                content: "Route: Standard path | Distance: 100km | Time: 1 day".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that estimates costs for each route.
pub struct CostEstimationAgent;

impl Agent for CostEstimationAgent {
    fn name(&self) -> &str {
        "CostEstimationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies)
            && ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|s| s.id.starts_with("route:"))
            && !ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|s| s.id.starts_with("cost:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut facts = Vec::new();

        for route in strategies.iter().filter(|s| s.id.starts_with("route:")) {
            // Extract distance and estimate cost
            let distance = route
                .content
                .split("Distance: ")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.strip_suffix("km"))
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(100);

            let cost = distance * 2 + 50; // Simple cost model

            facts.push(Fact {
                key: ContextKey::Strategies,
                id: format!(
                    "cost:{}",
                    route.id.strip_prefix("route:").unwrap_or("unknown")
                ),
                content: format!(
                    "Cost estimate: ${} | Route: {} | Breakdown: Transport ${}, Handling ${}",
                    cost,
                    route.id,
                    distance * 2,
                    50
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that assesses risks for each plan.
pub struct RiskAssessmentAgent;

impl Agent for RiskAssessmentAgent {
    fn name(&self) -> &str {
        "RiskAssessmentAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies)
            && !ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|s| s.id.starts_with("risk:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        let has_delay = signals
            .iter()
            .any(|s| s.content.contains("DELAYED") || s.content.contains("delayed"));

        for route in strategies.iter().filter(|s| s.id.starts_with("route:")) {
            let risk_level = if route.content.contains("Warehouse B") {
                "LOW" // Closer warehouse
            } else {
                "MEDIUM" // Multiple warehouses or default
            };

            let risk_score = match risk_level {
                "LOW" => 20,
                "MEDIUM" => 50,
                _ => 80,
            };

            facts.push(Fact {
                key: ContextKey::Strategies,
                id: format!(
                    "risk:{}",
                    route.id.strip_prefix("route:").unwrap_or("unknown")
                ),
                content: format!(
                    "Risk assessment: {} | Score: {}/100 | Factors: {}",
                    risk_level,
                    risk_score,
                    if has_delay {
                        "Supplier delay, alternative routing"
                    } else {
                        "Standard operations"
                    }
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that validates SLA requirements.
pub struct SLAValidationAgent;

impl Agent for SLAValidationAgent {
    fn name(&self) -> &str {
        "SLAValidationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Constraints)
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let facts = vec![
            Fact {
                key: ContextKey::Constraints,
                id: "sla:delivery-time".into(),
                content: "SLA: Delivery within 3 days | Penalty: $100/day delay".into(),
            },
            Fact {
                key: ContextKey::Constraints,
                id: "sla:quality".into(),
                content: "SLA: Quality standard must be maintained | No substitutions allowed"
                    .into(),
            },
        ];

        AgentEffect::with_facts(facts)
    }
}

/// Agent that consolidates all plans and ranks feasible options.
pub struct ConsolidationAgent;

impl Agent for ConsolidationAgent {
    fn name(&self) -> &str {
        "ConsolidationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Wait for routes, costs, risks, and SLA constraints
        let strategies = ctx.get(ContextKey::Strategies);
        let has_routes = strategies.iter().any(|s| s.id.starts_with("route:"));
        let has_costs = strategies.iter().any(|s| s.id.starts_with("cost:"));
        let has_risks = strategies.iter().any(|s| s.id.starts_with("risk:"));
        let has_sla = ctx.has(ContextKey::Constraints);

        has_routes && has_costs && has_risks && has_sla && !ctx.has(ContextKey::Evaluations)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let _constraints = ctx.get(ContextKey::Constraints);
        let mut facts = Vec::new();

        // Extract routes, costs, and risks
        let routes: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("route:"))
            .collect();

        let mut plans = Vec::new();

        for route in routes {
            let route_id = route.id.strip_prefix("route:").unwrap_or("unknown");
            let cost_strategy = strategies
                .iter()
                .find(|s| s.id.contains(route_id) && s.id.starts_with("cost:"));
            let risk_strategy = strategies
                .iter()
                .find(|s| s.id.contains(route_id) && s.id.starts_with("risk:"));

            // Extract time from route
            let time = route
                .content
                .split("Time: ")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(3.0);

            // Check SLA compliance
            let sla_ok = time <= 3.0;

            if sla_ok {
                let cost = cost_strategy
                    .and_then(|c| {
                        c.content
                            .split('$')
                            .nth(1)
                            .and_then(|s| s.split_whitespace().next())
                            .and_then(|s| s.parse::<u32>().ok())
                    })
                    .unwrap_or(1000);

                let risk = risk_strategy
                    .and_then(|r| {
                        r.content
                            .split("Score: ")
                            .nth(1)
                            .and_then(|s| s.split('/').next())
                            .and_then(|s| s.parse::<u32>().ok())
                    })
                    .unwrap_or(50);

                // Score: lower is better (cost + risk)
                let score = cost + (risk * 10);

                plans.push((route_id, score, cost, risk, time));
            }
        }

        // Sort by score (best first)
        plans.sort_by_key(|(_, score, _, _, _)| *score);

        // Emit evaluations
        for (i, (route_id, score, cost, risk, time)) in plans.iter().enumerate() {
            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: format!("eval:plan-{}", i + 1),
                content: format!(
                    "Plan {}: Route {} | Score: {} | Cost: ${} | Risk: {}/100 | Time: {} days | {}",
                    i + 1,
                    route_id,
                    score,
                    cost,
                    risk,
                    time,
                    if i == 0 { "RECOMMENDED" } else { "ALTERNATIVE" }
                ),
            });
        }

        if facts.is_empty() {
            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: "eval:infeasible".into(),
                content: "Status: INFEASIBLE | No plans satisfy SLA requirements".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

// =============================================================================
// SUPPLY CHAIN INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: At least one feasible plan must exist.
pub struct RequireFeasiblePlan;

impl Invariant for RequireFeasiblePlan {
    fn name(&self) -> &str {
        "require_feasible_plan"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_feasible = evaluations.iter().any(|e| {
            !e.content.contains("INFEASIBLE")
                && (e.content.contains("RECOMMENDED") || e.content.contains("ALTERNATIVE"))
        });

        if !has_feasible {
            return InvariantResult::Violated(Violation::new(
                "no feasible supply chain plan found",
            ));
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: All plans must satisfy SLA constraints.
pub struct RequireSLACompliance;

impl Invariant for RequireSLACompliance {
    fn name(&self) -> &str {
        "require_sla_compliance"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let constraints = ctx.get(ContextKey::Constraints);

        // Extract SLA requirement
        let max_days = constraints
            .iter()
            .find(|c| c.id.starts_with("sla:"))
            .and_then(|c| {
                c.content
                    .split("within ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.parse::<f64>().ok())
            })
            .unwrap_or(3.0);

        for eval in evaluations
            .iter()
            .filter(|e| !e.content.contains("INFEASIBLE"))
        {
            // Extract time from evaluation
            if let Some(time_str) = eval.content.split("Time: ").nth(1) {
                if let Some(time) = time_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    if time > max_days {
                        return InvariantResult::Violated(Violation::with_facts(
                            format!("plan violates SLA: {time} days > {max_days} days"),
                            vec![eval.id.clone()],
                        ));
                    }
                }
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: All routes must have cost and risk assessments.
pub struct RequireCompleteAssessments;

impl Invariant for RequireCompleteAssessments {
    fn name(&self) -> &str {
        "require_complete_assessments"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);

        let routes: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("route:"))
            .collect();

        // Only check if we have any costs/risks (meaning assessment agents have run)
        // This prevents checking too early in the cycle
        let has_any_costs = strategies.iter().any(|s| s.id.starts_with("cost:"));
        let has_any_risks = strategies.iter().any(|s| s.id.starts_with("risk:"));

        // If assessments haven't started yet, skip the check
        if !has_any_costs && !has_any_risks {
            return InvariantResult::Ok;
        }

        // Now check that all routes have assessments
        for route in routes {
            let route_id = route.id.strip_prefix("route:").unwrap_or("unknown");
            let has_cost = strategies
                .iter()
                .any(|s| s.id.contains(route_id) && s.id.starts_with("cost:"));
            let has_risk = strategies
                .iter()
                .any(|s| s.id.contains(route_id) && s.id.starts_with("risk:"));

            if !has_cost || !has_risk {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("route {route_id} missing cost or risk assessment"),
                    vec![route.id.clone()],
                ));
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
    fn parallel_data_agents_run_independently() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("orders", "Order A, Order B"));
        engine.register(DemandSnapshotAgent);
        engine.register(InventoryStateAgent);
        engine.register(SupplierStatusAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("demand:")));
        assert!(signals.iter().any(|s| s.id.starts_with("inventory:")));
        assert!(signals.iter().any(|s| s.id.starts_with("supplier:")));
    }

    #[test]
    fn route_generation_waits_for_data() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("orders", "Order A"));
        engine.register(SeedAgent::new("supplier:delay", "Supplier X delayed"));
        engine.register(DemandSnapshotAgent);
        engine.register(InventoryStateAgent);
        engine.register(SupplierStatusAgent);
        engine.register(RouteGenerationAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));
        let strategies = result.context.get(ContextKey::Strategies);
        assert!(strategies.iter().any(|s| s.id.starts_with("route:")));
    }

    #[test]
    fn consolidation_waits_for_all_assessments() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("orders", "Order A"));
        engine.register(SeedAgent::new("supplier:delay", "Supplier X delayed"));
        engine.register(DemandSnapshotAgent);
        engine.register(InventoryStateAgent);
        engine.register(SupplierStatusAgent);
        engine.register(RouteGenerationAgent);
        engine.register(CostEstimationAgent);
        engine.register(RiskAssessmentAgent);
        engine.register(SLAValidationAgent);
        engine.register(ConsolidationAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));
        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
        assert!(
            evals
                .iter()
                .any(|e| e.content.contains("RECOMMENDED") || e.content.contains("ALTERNATIVE"))
        );
    }

    #[test]
    fn invariants_enforce_quality() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("orders", "Order A"));
        engine.register(SeedAgent::new("supplier:delay", "Supplier X delayed"));
        engine.register(DemandSnapshotAgent);
        engine.register(InventoryStateAgent);
        engine.register(SupplierStatusAgent);
        engine.register(RouteGenerationAgent);
        engine.register(CostEstimationAgent);
        engine.register(RiskAssessmentAgent);
        engine.register(SLAValidationAgent);
        engine.register(ConsolidationAgent);

        engine.register_invariant(RequireFeasiblePlan);
        engine.register_invariant(RequireSLACompliance);
        engine.register_invariant(RequireCompleteAssessments);

        let result = engine.run(Context::new());

        assert!(result.is_ok(), "Engine run failed: {result:?}");
        let result = result.unwrap();
        assert!(result.converged);
    }

    #[test]
    fn full_pipeline_converges_deterministically() {
        let run = || {
            let mut engine = Engine::new();
            engine.register(SeedAgent::new("orders", "Order A, Order B"));
            engine.register(SeedAgent::new("supplier:delay", "Supplier X delayed"));
            engine.register(DemandSnapshotAgent);
            engine.register(InventoryStateAgent);
            engine.register(SupplierStatusAgent);
            engine.register(RouteGenerationAgent);
            engine.register(CostEstimationAgent);
            engine.register(RiskAssessmentAgent);
            engine.register(SLAValidationAgent);
            engine.register(ConsolidationAgent);
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
