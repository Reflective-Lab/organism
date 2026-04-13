// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Strategic Sourcing use case.

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};
use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled Strategic Sourcing agents with mock providers.
#[must_use]
pub fn setup_mock_llm_strategic_sourcing(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Supplier Discovery Agent: Fast extraction
    let (agent, provider) = create_mock_llm_agent(
        "SupplierDiscoveryAgent",
        "You discover and profile suppliers.",
        "Discover suppliers: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "Supplier: VendorA | Location: Domestic | Capacity: 500 units/month",
            0.8,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Compliance Agent: Validation
    let (agent, provider) = create_mock_llm_agent(
        "ComplianceAgent",
        "You check supplier compliance status.",
        "Check compliance: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::validation(),
        vec![MockResponse::success(
            "Compliance VendorA: COMPLIANT | Certifications: ISO 9001, ISO 14001",
            0.9,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // ESG Scoring Agent: Analysis
    let (agent, provider) = create_mock_llm_agent(
        "ESGScoringAgent",
        "You score suppliers on ESG factors.",
        "Score ESG: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::analysis(),
        vec![MockResponse::success(
            "ESG VendorA: Score 95/100 | Environmental: 95 | Social: 90 | Governance: 100",
            0.85,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Sourcing Strategy Agent: Synthesis
    let (agent, provider) = create_mock_llm_agent(
        "SourcingStrategyAgent",
        "You synthesize sourcing strategies from all assessments.",
        "Synthesize strategy: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals],
        requirements::synthesis(),
        vec![MockResponse::success(
            "Shortlist 1: VendorA | Composite score: 285 | ESG: 95 | Price: $100",
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
    fn mock_llm_strategic_sourcing_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("suppliers", "VendorA, VendorB"));

        let _providers = setup_mock_llm_strategic_sourcing(&mut engine);

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
