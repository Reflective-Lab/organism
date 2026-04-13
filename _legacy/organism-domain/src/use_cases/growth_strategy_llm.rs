// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Growth Strategy use case.
//!
//! This module demonstrates how to set up LLM agents with model selection
//! for the Growth Strategy use case. It follows the Converge pattern:
//!
//! 1. Agents specify requirements (cost, latency, capabilities)
//! 2. Model selector chooses appropriate models
//! 3. Providers are created from selected models
//! 4. LLM agents are instantiated with providers
//!
//! For testing, use `create_mock_llm_agent` from `llm_utils`.

use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};

// NOTE: setup_llm_growth_strategy (using ProviderRegistry + create_llm_agent) is temporarily
// disabled. converge-provider's LlmProvider trait diverged from converge-core's in core 1.0.2.
// See REF-36 for context.

/// Sets up LLM-enabled Growth Strategy agents with mock providers (for testing).
///
/// Returns the mock providers so you can configure their responses.
#[must_use]
pub fn setup_mock_llm_growth_strategy(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Market Signal Agent
    let (agent, provider) = create_mock_llm_agent(
        "MarketSignalAgent",
        "You are a market analyst.",
        "Extract market signals: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "Market signal: Growing demand in Nordic B2B SaaS sector",
            0.8,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Competitor Agent
    let (agent, provider) = create_mock_llm_agent(
        "CompetitorAgent",
        "You are a competitive intelligence analyst.",
        "Analyze competitors: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::analysis(),
        vec![MockResponse::success(
            "Competitor analysis: Major players focusing on enterprise segment",
            0.85,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Strategy Agent
    let (agent, provider) = create_mock_llm_agent(
        "StrategyAgent",
        "You are a strategic planner.",
        "Synthesize strategies: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals],
        requirements::synthesis(),
        vec![MockResponse::success(
            "Strategy 1: Expand into Nordic markets with localized offerings",
            0.9,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Evaluation Agent
    let (agent, provider) = create_mock_llm_agent(
        "EvaluationAgent",
        "You are a strategy evaluator.",
        "Evaluate strategies: {context}",
        ContextKey::Evaluations,
        vec![ContextKey::Strategies],
        requirements::deep_research(),
        vec![MockResponse::success(
            "Evaluation: Strategy 1 shows high potential with moderate risk. Rationale: Strong market signals and competitive positioning.",
            0.88,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    providers
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Context;
    use converge_core::agents::SeedAgent;

    #[test]
    fn mock_llm_growth_strategy_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("market", "Nordic B2B SaaS"));

        let _providers = setup_mock_llm_growth_strategy(&mut engine);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        // LLM agents emit proposals to ContextKey::Proposals
        // At least the first agent (MarketSignalAgent) should run since it depends on Seeds
        let proposals = result.context.get(ContextKey::Proposals);
        assert!(
            !proposals.is_empty(),
            "At least one LLM agent should have produced proposals"
        );
    }

    #[test]
    fn llm_agents_use_appropriate_requirements() {
        // Verify that different agents use different requirements
        let reqs_fast = requirements::fast_extraction();
        let reqs_synthesis = requirements::synthesis();
        let reqs_deep = requirements::deep_research();

        // Fast extraction should be cheaper
        assert!(reqs_fast.max_cost_class < reqs_synthesis.max_cost_class);
        // Deep research should require reasoning
        assert!(reqs_deep.requires_reasoning);
        // Synthesis should have higher quality threshold
        assert!(reqs_synthesis.min_quality > reqs_fast.min_quality);
    }
}
