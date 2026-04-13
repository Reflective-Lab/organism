// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Growth Strategy agents for market analysis.
//!
//! This module implements a deterministic growth strategy use case
//! that validates the Converge engine with a real domain.
//!
//! # Agent Pipeline
//!
//! ```text
//! Seeds (market, product)
//!    │
//!    ▼
//! MarketSignalAgent → Signals (market observations)
//!    │
//!    ▼
//! CompetitorAgent → Competitors (competitor profiles)
//!    │
//!    ▼
//! StrategyAgent → Strategies (proposed strategies)
//!    │
//!    ▼
//! EvaluationAgent → Evaluations (ranked strategies)
//! ```
//!
//! # Example
//!
//! ```
//! use converge_core::{Engine, Context, ContextKey};
//! use converge_core::agents::SeedAgent;
//! use converge_domain::growth_strategy::{
//!     MarketSignalAgent, CompetitorAgent, StrategyAgent, EvaluationAgent,
//! };
//!
//! let mut engine = Engine::new();
//!
//! // Seed the context with market and product
//! engine.register(SeedAgent::new("market:nordic-b2b", "Nordic B2B market"));
//! engine.register(SeedAgent::new("product:product-x", "Product X - SaaS platform"));
//!
//! // Register growth strategy agents
//! engine.register(MarketSignalAgent);
//! engine.register(CompetitorAgent);
//! engine.register(StrategyAgent);
//! engine.register(EvaluationAgent);
//!
//! let result = engine.run(Context::new()).expect("should converge");
//!
//! assert!(result.converged);
//! assert!(result.context.has(ContextKey::Strategies));
//! assert!(result.context.has(ContextKey::Evaluations));
//! ```

// Agent trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Agent that observes seeds and emits market signals.
///
/// Simulates discovery of market observations based on seed data.
/// In a real system, this would fetch from web, social, and market sources.
pub struct MarketSignalAgent;

impl Agent for MarketSignalAgent {
    fn name(&self) -> &str {
        "MarketSignalAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run once when seeds exist but no signals yet
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Signals)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);

        // Deterministic signal generation based on seeds
        let mut facts = Vec::new();

        // Check if we have market info
        let has_nordic = seeds.iter().any(|s| s.content.contains("Nordic"));
        let has_b2b = seeds.iter().any(|s| s.content.contains("B2B"));

        if has_nordic {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "signal:nordic-growth".into(),
                content: "Nordic SaaS market growing 15% YoY".into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "signal:nordic-competition".into(),
                content: "3 major competitors in Nordic region".into(),
            });
        }

        if has_b2b {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "signal:b2b-trend".into(),
                content: "B2B buyers prefer self-service demos".into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "signal:b2b-channel".into(),
                content: "LinkedIn most effective B2B channel".into(),
            });
        }

        // Always emit a baseline signal
        if facts.is_empty() {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "signal:baseline".into(),
                content: "Market conditions stable".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that analyzes signals and identifies competitors.
///
/// Turns raw signals into structured competitor profiles.
pub struct CompetitorAgent;

impl Agent for CompetitorAgent {
    fn name(&self) -> &str {
        "CompetitorAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run once when signals exist but no competitors yet
        ctx.has(ContextKey::Signals) && !ctx.has(ContextKey::Competitors)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);

        let mut facts = Vec::new();

        // Check for competition signals
        let has_competition_signal = signals.iter().any(|s| s.content.contains("competitor"));

        if has_competition_signal {
            facts.push(Fact {
                key: ContextKey::Competitors,
                id: "competitor:alpha-corp".into(),
                content: "AlphaCorp: Strong in enterprise, weak in SMB".into(),
            });
            facts.push(Fact {
                key: ContextKey::Competitors,
                id: "competitor:beta-inc".into(),
                content: "BetaInc: Price leader, limited features".into(),
            });
            facts.push(Fact {
                key: ContextKey::Competitors,
                id: "competitor:gamma-tech".into(),
                content: "GammaTech: New entrant, aggressive marketing".into(),
            });
        } else {
            // No specific competition data - emit generic profile
            facts.push(Fact {
                key: ContextKey::Competitors,
                id: "competitor:unknown".into(),
                content: "Competitor landscape unclear - recommend analysis".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that synthesizes strategies from competitors and constraints.
///
/// Proposes viable growth strategies based on competitive analysis.
pub struct StrategyAgent;

impl Agent for StrategyAgent {
    fn name(&self) -> &str {
        "StrategyAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Competitors, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run once when competitors exist but no strategies yet
        ctx.has(ContextKey::Competitors) && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let competitors = ctx.get(ContextKey::Competitors);
        let signals = ctx.get(ContextKey::Signals);

        let mut facts = Vec::new();

        // Analyze competitive gaps
        let has_smb_gap = competitors
            .iter()
            .any(|c| c.content.contains("weak in SMB"));
        let has_linkedin_channel = signals.iter().any(|s| s.content.contains("LinkedIn"));
        let has_self_service = signals.iter().any(|s| s.content.contains("self-service"));

        // Strategy 1: SMB focus if competitor gap exists
        if has_smb_gap {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "strategy:smb-focus".into(),
                content: "Target SMB segment where AlphaCorp is weak. \
                          Position as affordable enterprise-grade solution."
                    .into(),
            });
        }

        // Strategy 2: Channel strategy based on signals
        if has_linkedin_channel {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "strategy:linkedin-campaign".into(),
                content: "Launch targeted LinkedIn campaign for Nordic B2B. \
                          Focus on decision-maker personas."
                    .into(),
            });
        }

        // Strategy 3: Self-service strategy
        if has_self_service {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "strategy:self-service".into(),
                content: "Build self-service demo experience. \
                          Reduce friction in buyer journey."
                    .into(),
            });
        }

        // Ensure at least two strategies (required by invariant)
        if facts.len() < 2 {
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "strategy:content-marketing".into(),
                content: "Establish thought leadership through content marketing. \
                          Build trust with educational content."
                    .into(),
            });
            facts.push(Fact {
                key: ContextKey::Strategies,
                id: "strategy:partnership".into(),
                content: "Partner with local system integrators. \
                          Leverage existing relationships."
                    .into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that evaluates and scores strategies.
///
/// Produces ranked evaluations with rationale.
pub struct EvaluationAgent;

impl Agent for EvaluationAgent {
    fn name(&self) -> &str {
        "EvaluationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run once when strategies exist but no evaluations yet
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);

        let mut facts = Vec::new();

        // Evaluate each strategy
        for (i, strategy) in strategies.iter().enumerate() {
            let (score, rationale) = evaluate_strategy(strategy);

            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: format!(
                    "eval:{}",
                    strategy
                        .id
                        .strip_prefix("strategy:")
                        .unwrap_or(&strategy.id)
                ),
                content: format!(
                    "Score: {}/100 | {} | Rationale: {}",
                    score,
                    if i == 0 { "RECOMMENDED" } else { "ALTERNATIVE" },
                    rationale
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Deterministic strategy evaluation function.
fn evaluate_strategy(strategy: &Fact) -> (u32, &'static str) {
    let content = &strategy.content;

    // Score based on strategy characteristics
    if content.contains("SMB") {
        (
            85,
            "Strong competitive differentiation in underserved segment",
        )
    } else if content.contains("LinkedIn") {
        (78, "High-reach channel with proven B2B effectiveness")
    } else if content.contains("self-service") {
        (72, "Reduces friction but requires development investment")
    } else if content.contains("content") {
        (65, "Long-term brand building, slower ROI")
    } else if content.contains("partnership") {
        (60, "Leverages existing relationships, execution dependent")
    } else {
        (50, "Standard approach, moderate differentiation")
    }
}

// =============================================================================
// GROWTH STRATEGY INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: At least two distinct strategies must exist.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Growth strategies are identified
///   When the system converges
///   Then at least two distinct growth strategies exist
/// ```
pub struct RequireMultipleStrategies;

impl Invariant for RequireMultipleStrategies {
    fn name(&self) -> &str {
        "require_multiple_strategies"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);
        if strategies.len() < 2 {
            return InvariantResult::Violated(Violation::new(format!(
                "need at least 2 strategies, found {}",
                strategies.len()
            )));
        }
        InvariantResult::Ok
    }
}

/// Structural invariant: No strategy may contain forbidden content.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Brand and budget safety
///   When a strategy is recommended
///   Then no strategy violates brand safety constraints
/// ```
pub struct BrandSafetyInvariant {
    /// Words that violate brand safety.
    pub forbidden_terms: Vec<&'static str>,
}

impl Default for BrandSafetyInvariant {
    fn default() -> Self {
        Self {
            forbidden_terms: vec!["spam", "misleading", "aggressive", "deceptive", "unethical"],
        }
    }
}

impl Invariant for BrandSafetyInvariant {
    fn name(&self) -> &str {
        "brand_safety"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for strategy in ctx.get(ContextKey::Strategies) {
            let content_lower = strategy.content.to_lowercase();
            for term in &self.forbidden_terms {
                if content_lower.contains(term) {
                    return InvariantResult::Violated(Violation::with_facts(
                        format!("strategy contains forbidden term '{term}'"),
                        vec![strategy.id.clone()],
                    ));
                }
            }
        }
        InvariantResult::Ok
    }
}

/// Acceptance invariant: Every strategy must have an evaluation.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Strategy rationale
///   Then every recommended strategy has an explanation
/// ```
pub struct RequireStrategyEvaluations;

impl Invariant for RequireStrategyEvaluations {
    fn name(&self) -> &str {
        "require_strategy_evaluations"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let evaluations = ctx.get(ContextKey::Evaluations);

        for strategy in strategies {
            // Check if there's an evaluation for this strategy
            let strategy_key = strategy
                .id
                .strip_prefix("strategy:")
                .unwrap_or(&strategy.id);
            let has_eval = evaluations.iter().any(|e| e.id.contains(strategy_key));

            if !has_eval {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("strategy '{}' has no evaluation", strategy.id),
                    vec![strategy.id.clone()],
                ));
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: Evaluations must contain rationale.
///
/// Ensures explanations are not empty placeholders.
pub struct RequireEvaluationRationale;

impl Invariant for RequireEvaluationRationale {
    fn name(&self) -> &str {
        "require_evaluation_rationale"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for eval in ctx.get(ContextKey::Evaluations) {
            if !eval.content.contains("Rationale:") {
                return InvariantResult::Violated(Violation::with_facts(
                    "evaluation missing rationale",
                    vec![eval.id.clone()],
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
    fn market_signal_agent_emits_signals_from_seeds() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("market:nordic-b2b", "Nordic B2B market"));
        engine.register(MarketSignalAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Signals));

        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.contains("nordic")));
    }

    #[test]
    fn competitor_agent_analyzes_signals() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("market:nordic-b2b", "Nordic B2B market"));
        engine.register(MarketSignalAgent);
        engine.register(CompetitorAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Competitors));
    }

    #[test]
    fn strategy_agent_proposes_strategies() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("market:nordic-b2b", "Nordic B2B market"));
        engine.register(MarketSignalAgent);
        engine.register(CompetitorAgent);
        engine.register(StrategyAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));

        let strategies = result.context.get(ContextKey::Strategies);
        assert!(strategies.len() >= 2, "Should have at least 2 strategies");
    }

    #[test]
    fn evaluation_agent_scores_strategies() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("market:nordic-b2b", "Nordic B2B market"));
        engine.register(MarketSignalAgent);
        engine.register(CompetitorAgent);
        engine.register(StrategyAgent);
        engine.register(EvaluationAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));

        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
        assert!(evals.iter().any(|e| e.content.contains("RECOMMENDED")));
    }

    #[test]
    fn full_pipeline_converges_deterministically() {
        let run = || {
            let mut engine = Engine::new();
            engine.register(SeedAgent::new("market:nordic-b2b", "Nordic B2B market"));
            engine.register(SeedAgent::new(
                "product:product-x",
                "Product X - SaaS platform",
            ));
            engine.register(MarketSignalAgent);
            engine.register(CompetitorAgent);
            engine.register(StrategyAgent);
            engine.register(EvaluationAgent);
            engine.run(Context::new()).expect("should converge")
        };

        let r1 = run();
        let r2 = run();

        // Same number of cycles
        assert_eq!(r1.cycles, r2.cycles);

        // Same strategies
        assert_eq!(
            r1.context.get(ContextKey::Strategies),
            r2.context.get(ContextKey::Strategies)
        );

        // Same evaluations
        assert_eq!(
            r1.context.get(ContextKey::Evaluations),
            r2.context.get(ContextKey::Evaluations)
        );
    }
}
