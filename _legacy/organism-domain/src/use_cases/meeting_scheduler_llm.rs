// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Meeting Scheduler use case.

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};
use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled Meeting Scheduler agents with mock providers (for testing).
#[must_use]
pub fn setup_mock_llm_meeting_scheduler(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Availability Retrieval Agent: Fast extraction
    let (agent, provider) = create_mock_llm_agent(
        "AvailabilityRetrievalAgent",
        "You extract participant availability from calendars.",
        "Extract availability: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "Alice: Available Mon 10-12, Tue 14-16 | Bob: Available Mon 9-11, Wed 10-12",
            0.8,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Time Zone Normalization Agent: Fast processing
    let (agent, provider) = create_mock_llm_agent(
        "TimeZoneNormalizationAgent",
        "You normalize time zones to UTC.",
        "Normalize time zones: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "Normalized: All times in UTC | Alice: UTC+1, Bob: UTC-5",
            0.85,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Slot Optimization Agent: Analysis for optimization
    let (agent, provider) = create_mock_llm_agent(
        "SlotOptimizationAgent",
        "You optimize meeting time slots based on availability.",
        "Optimize slots: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals, ContextKey::Constraints],
        requirements::analysis(),
        vec![MockResponse::success(
            "Optimal slot: Mon 10:00 UTC | Participants: Alice, Bob | Duration: 60 min",
            0.9,
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
    fn mock_llm_meeting_scheduler_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("participants", "Alice, Bob"));

        let _providers = setup_mock_llm_meeting_scheduler(&mut engine);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        // LLM agents emit proposals to ContextKey::Proposals
        let proposals = result.context.get(ContextKey::Proposals);
        assert!(
            !proposals.is_empty(),
            "At least one LLM agent should have produced proposals"
        );
    }
}
