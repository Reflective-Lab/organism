// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! CRM Account Health & Growth Strategy agents for continuous account monitoring.
//!
//! This module implements continuous account health assessment and growth action proposals,
//! demonstrating reactive agents and continuous monitoring patterns.
//!
//! # Agent Pipeline
//!
//! ```text
//! Seeds (account data)
//!    │
//!    ├─► UsageSignalAgent → Signals (usage metrics)
//!    ├─► SupportTicketAgent → Signals (support activity)
//!    ├─► RevenueTrendAgent → Signals (revenue trends)
//!    ├─► ChurnRiskAgent → Strategies (risk assessments)
//!    ├─► UpsellOpportunityAgent → Strategies (opportunity analysis)
//!    └─► ActionPrioritizationAgent → Evaluations (ranked actions)
//! ```

// Agent trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Agent that monitors usage signals.
pub struct UsageSignalAgent;

impl Agent for UsageSignalAgent {
    fn name(&self) -> &str {
        "UsageSignalAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("usage:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let account_id = seeds
            .iter()
            .find(|s| s.id == "account")
            .map_or("Account123", |s| s.content.as_str());

        // Simulate usage metrics
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "usage:active-users".into(),
            content: format!(
                "Usage {account_id}: Active users: 450 | Trend: +5% MoM | Engagement: High"
            ),
        });
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "usage:feature-adoption".into(),
            content: format!(
                "Usage {account_id}: Feature adoption: 78% | Top features: A, B, C | Underutilized: D"
            ),
        });

        AgentEffect::with_facts(facts)
    }
}

/// Agent that analyzes support ticket patterns.
pub struct SupportTicketAgent;

impl Agent for SupportTicketAgent {
    fn name(&self) -> &str {
        "SupportTicketAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("support:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let account_id = seeds
            .iter()
            .find(|s| s.id == "account")
            .map_or("Account123", |s| s.content.as_str());

        // Simulate support metrics
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "support:tickets".into(),
            content: format!("Support {account_id}: Tickets last 30 days: 12 | Avg resolution: 2.5 days | Satisfaction: 4.2/5"),
        });
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "support:escalations".into(),
            content: format!(
                "Support {account_id}: Escalations: 2 | Critical issues: 0 | Status: Healthy"
            ),
        });

        AgentEffect::with_facts(facts)
    }
}

/// Agent that tracks revenue trends.
pub struct RevenueTrendAgent;

impl Agent for RevenueTrendAgent {
    fn name(&self) -> &str {
        "RevenueTrendAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("revenue:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let account_id = seeds
            .iter()
            .find(|s| s.id == "account")
            .map_or("Account123", |s| s.content.as_str());

        // Simulate revenue metrics
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "revenue:mrr".into(),
            content: format!(
                "Revenue {account_id}: MRR: $15,000 | Growth: +8% MoM | Contract: Annual"
            ),
        });
        facts.push(Fact {
            key: ContextKey::Signals,
            id: "revenue:expansion".into(),
            content: format!("Revenue {account_id}: Expansion revenue: $2,000 | Upsell potential: $5,000 | Churn risk: Low"),
        });

        AgentEffect::with_facts(facts)
    }
}

/// Agent that assesses churn risk.
pub struct ChurnRiskAgent;

impl Agent for ChurnRiskAgent {
    fn name(&self) -> &str {
        "ChurnRiskAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let signals = ctx.get(ContextKey::Signals);
        let has_usage = signals.iter().any(|s| s.id.starts_with("usage:"));
        let has_support = signals.iter().any(|s| s.id.starts_with("support:"));
        let has_revenue = signals.iter().any(|s| s.id.starts_with("revenue:"));

        has_usage && has_support && has_revenue && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        // Analyze signals for churn indicators
        let usage_trend = signals
            .iter()
            .find(|s| s.id == "usage:active-users")
            .map_or(0, |s| if s.content.contains('+') { 1 } else { -1 });

        let support_health = signals
            .iter()
            .find(|s| s.id == "support:escalations")
            .map_or(0, |s| if s.content.contains("Healthy") { 1 } else { -1 });

        let revenue_growth = signals
            .iter()
            .find(|s| s.id == "revenue:mrr")
            .map_or(0, |s| if s.content.contains('+') { 1 } else { -1 });

        let risk_score = (3 - (usage_trend + support_health + revenue_growth)) * 20;
        let risk_level = if risk_score < 30 {
            "LOW"
        } else if risk_score < 60 {
            "MEDIUM"
        } else {
            "HIGH"
        };

        facts.push(Fact {
            key: ContextKey::Strategies,
            id: "risk:churn".into(),
            content: format!("Churn risk: {} | Score: {}/100 | Factors: Usage trend: {}, Support: {}, Revenue: {} | Action: {}", 
                risk_level, risk_score,
                if usage_trend > 0 { "Positive" } else { "Negative" },
                if support_health > 0 { "Healthy" } else { "Concerning" },
                if revenue_growth > 0 { "Growing" } else { "Declining" },
                if risk_score > 60 { "IMMEDIATE INTERVENTION" } else if risk_score > 30 { "Monitor closely" } else { "Low priority" }),
        });

        AgentEffect::with_facts(facts)
    }
}

/// Agent that identifies upsell opportunities (simulated LLM).
pub struct UpsellOpportunityAgent;

impl Agent for UpsellOpportunityAgent {
    fn name(&self) -> &str {
        "UpsellOpportunityAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let signals = ctx.get(ContextKey::Signals);
        let has_usage = signals.iter().any(|s| s.id.starts_with("usage:"));
        let has_revenue = signals.iter().any(|s| s.id.starts_with("revenue:"));

        has_usage
            && has_revenue
            && !ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|s| s.id.starts_with("opportunity:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        let usage_adoption = signals
            .iter()
            .find(|s| s.id == "usage:feature-adoption")
            .and_then(|s| {
                s.content
                    .split("adoption: ")
                    .nth(1)
                    .and_then(|s| s.split('%').next())
                    .and_then(|s| s.parse::<u32>().ok())
            })
            .unwrap_or(0);

        let expansion_potential = signals
            .iter()
            .find(|s| s.id == "revenue:expansion")
            .and_then(|s| {
                s.content
                    .split("potential: $")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .and_then(|s| s.replace(',', "").parse::<u32>().ok())
            })
            .unwrap_or(0);

        if usage_adoption > 70 && expansion_potential > 0 {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "opportunity:upsell".into(),
                content: format!("Upsell opportunity: High | Potential: ${expansion_potential} | Rationale: High feature adoption ({usage_adoption}%) + expansion potential | Priority: HIGH"),
            });
        } else if expansion_potential > 0 {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "opportunity:upsell".into(),
                content: format!("Upsell opportunity: Medium | Potential: ${expansion_potential} | Rationale: Expansion potential identified | Priority: MEDIUM"),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that prioritizes actions.
pub struct ActionPrioritizationAgent;

impl Agent for ActionPrioritizationAgent {
    fn name(&self) -> &str {
        "ActionPrioritizationAgent"
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

        let churn_risk = strategies.iter().find(|s| s.id.starts_with("risk:"));
        let upsell_opp = strategies.iter().find(|s| s.id.starts_with("opportunity:"));

        let mut actions = Vec::new();

        if let Some(risk) = churn_risk {
            let priority = if risk.content.contains("HIGH") {
                1
            } else if risk.content.contains("MEDIUM") {
                2
            } else {
                3
            };
            actions.push((priority, "Churn Risk", risk.content.clone()));
        }

        if let Some(opp) = upsell_opp {
            let priority = if opp.content.contains("HIGH") { 2 } else { 3 };
            actions.push((priority, "Upsell", opp.content.clone()));
        }

        actions.sort_by_key(|(p, _, _)| *p);

        for (i, (_, action_type, content)) in actions.iter().enumerate() {
            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: format!("eval:{}", i + 1),
                content: format!(
                    "Action {}: {} | {} | Priority: {} | {}",
                    i + 1,
                    action_type,
                    content,
                    if i == 0 { "URGENT" } else { "NORMAL" },
                    if i == 0 { "RECOMMENDED" } else { "ALTERNATIVE" }
                ),
            });
        }

        if facts.is_empty() {
            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: "eval:no-action".into(),
                content: "Status: NO ACTION NEEDED | Account healthy | Continue monitoring".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

// =============================================================================
// CRM ACCOUNT HEALTH INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: High churn risk must have action plan.
pub struct RequireChurnActionPlan;

impl Invariant for RequireChurnActionPlan {
    fn name(&self) -> &str {
        "require_churn_action_plan"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_high_risk = strategies
            .iter()
            .any(|s| s.id.starts_with("risk:") && s.content.contains("HIGH"));

        if has_high_risk {
            let has_action = evaluations
                .iter()
                .any(|e| e.content.contains("Churn Risk") && e.content.contains("RECOMMENDED"));

            if !has_action {
                return InvariantResult::Violated(Violation::new(
                    "high churn risk detected but no action plan provided",
                ));
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: All signals must be analyzed.
pub struct RequireCompleteAnalysis;

impl Invariant for RequireCompleteAnalysis {
    fn name(&self) -> &str {
        "require_complete_analysis"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);
        let strategies = ctx.get(ContextKey::Strategies);

        // Only check if we have evaluations (meaning analysis is complete)
        let has_evaluations = ctx.has(ContextKey::Evaluations);
        if !has_evaluations {
            return InvariantResult::Ok; // Too early, skip check
        }

        let required_signals = ["usage:", "support:", "revenue:"];
        let has_all_signals = required_signals
            .iter()
            .all(|prefix| signals.iter().any(|s| s.id.starts_with(prefix)));

        if !has_all_signals {
            return InvariantResult::Violated(Violation::new("missing required signal analysis"));
        }

        // Should have either risk or opportunity assessment
        let has_assessment = strategies
            .iter()
            .any(|s| s.id.starts_with("risk:") || s.id.starts_with("opportunity:"));

        if !has_assessment {
            return InvariantResult::Violated(Violation::new(
                "no risk or opportunity assessment provided",
            ));
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
    fn parallel_signal_agents_run_independently() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("account", "Account123"));
        engine.register(UsageSignalAgent);
        engine.register(SupportTicketAgent);
        engine.register(RevenueTrendAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("usage:")));
        assert!(signals.iter().any(|s| s.id.starts_with("support:")));
        assert!(signals.iter().any(|s| s.id.starts_with("revenue:")));
    }

    #[test]
    fn churn_risk_assessed_from_signals() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("account", "Account123"));
        engine.register(UsageSignalAgent);
        engine.register(SupportTicketAgent);
        engine.register(RevenueTrendAgent);
        engine.register(ChurnRiskAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));
        let strategies = result.context.get(ContextKey::Strategies);
        assert!(strategies.iter().any(|s| s.id.starts_with("risk:")));
    }

    #[test]
    fn upsell_opportunities_identified() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("account", "Account123"));
        engine.register(UsageSignalAgent);
        engine.register(SupportTicketAgent);
        engine.register(RevenueTrendAgent);
        engine.register(UpsellOpportunityAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let strategies = result.context.get(ContextKey::Strategies);
        // May or may not have opportunity depending on signals
        assert!(
            strategies
                .iter()
                .any(|s| s.id.starts_with("risk:") || s.id.starts_with("opportunity:"))
        );
    }

    #[test]
    fn actions_prioritized() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("account", "Account123"));
        engine.register(UsageSignalAgent);
        engine.register(SupportTicketAgent);
        engine.register(RevenueTrendAgent);
        engine.register(ChurnRiskAgent);
        engine.register(UpsellOpportunityAgent);
        engine.register(ActionPrioritizationAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));
        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
    }

    #[test]
    fn invariants_enforce_quality() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("account", "Account123"));
        engine.register(UsageSignalAgent);
        engine.register(SupportTicketAgent);
        engine.register(RevenueTrendAgent);
        engine.register(ChurnRiskAgent);
        engine.register(UpsellOpportunityAgent);
        engine.register(ActionPrioritizationAgent);

        engine.register_invariant(RequireChurnActionPlan);
        engine.register_invariant(RequireCompleteAnalysis);

        let result = engine.run(Context::new());

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.converged);
    }

    #[test]
    fn full_pipeline_converges_deterministically() {
        let run = || {
            let mut engine = Engine::new();
            engine.register(SeedAgent::new("account", "Account123"));
            engine.register(UsageSignalAgent);
            engine.register(SupportTicketAgent);
            engine.register(RevenueTrendAgent);
            engine.register(ChurnRiskAgent);
            engine.register(UpsellOpportunityAgent);
            engine.register(ActionPrioritizationAgent);
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
