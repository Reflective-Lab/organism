// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Inventory Rebalancing use case.

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};
use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled Inventory Rebalancing agents with mock providers.
#[must_use]
pub fn setup_mock_llm_inventory_rebalancing(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Forecast Agent: Analysis (forecasting)
    let (agent, provider) = create_mock_llm_agent(
        "ForecastAgent",
        "You generate demand forecasts based on sales velocity and inventory.",
        "Forecast demand: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::analysis(),
        vec![MockResponse::success(
            "Region North: 30-day forecast 450 units | Days until stockout: 2 | Confidence: 85%",
            0.85,
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
    fn mock_llm_inventory_rebalancing_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regions", "North, South"));

        let _providers = setup_mock_llm_inventory_rebalancing(&mut engine);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
    }
}
