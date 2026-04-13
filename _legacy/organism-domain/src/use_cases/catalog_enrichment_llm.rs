// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Catalog Enrichment use case.

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};
use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled Catalog Enrichment agents with mock providers.
#[must_use]
pub fn setup_mock_llm_catalog_enrichment(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Category Inference Agent: Categorization
    let (agent, provider) = create_mock_llm_agent(
        "CategoryInferenceAgent",
        "You infer product categories from attributes.",
        "Infer category: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::categorization(),
        vec![MockResponse::success(
            "Category ProductA: Electronics > Widgets | Confidence: 90%",
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
    fn mock_llm_catalog_enrichment_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("feeds", "ProductA:Widget:99.99"));

        // Add deterministic agents to create Signals first
        use crate::use_cases::catalog_enrichment::{
            AttributeNormalizationAgent, DeduplicationAgent, FeedIngestionAgent,
        };
        engine.register(FeedIngestionAgent);
        engine.register(DeduplicationAgent);
        engine.register(AttributeNormalizationAgent);

        let _providers = setup_mock_llm_catalog_enrichment(&mut engine);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
    }
}
