// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled CRM Account Health use case.

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};
use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled CRM Account Health agents with mock providers.
#[must_use]
pub fn setup_mock_llm_crm_account_health(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Upsell Opportunity Agent: Analysis
    let (agent, provider) = create_mock_llm_agent(
        "UpsellOpportunityAgent",
        "You identify upsell opportunities from account data.",
        "Identify upsell: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals],
        requirements::analysis(),
        vec![MockResponse::success(
            "Upsell opportunity: High | Potential: $5,000 | Rationale: High feature adoption (78%) + expansion potential",
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
    fn mock_llm_crm_account_health_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("account", "Account123"));

        // Add deterministic agents to create Signals first
        use crate::use_cases::crm_account_health::{RevenueTrendAgent, SupportTicketAgent, UsageSignalAgent};
        engine.register(UsageSignalAgent);
        engine.register(SupportTicketAgent);
        engine.register(RevenueTrendAgent);

        let _providers = setup_mock_llm_crm_account_health(&mut engine);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
    }
}
