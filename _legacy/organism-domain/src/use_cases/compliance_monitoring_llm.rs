// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled Compliance Monitoring use case.

use converge_domain::llm_utils::{create_mock_llm_agent, requirements};
use converge_domain::mock::{MockProvider, MockResponse};
use converge_core::{ContextKey, Engine};
use std::sync::Arc;

/// Sets up LLM-enabled Compliance Monitoring agents with mock providers.
#[must_use]
pub fn setup_mock_llm_compliance_monitoring(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Regulation Parser Agent: Analysis
    let (agent, provider) = create_mock_llm_agent(
        "RegulationParserAgent",
        "You parse regulations and extract requirements.",
        "Parse regulation: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::analysis(),
        vec![MockResponse::success(
            "Regulation GDPR: Parsed | Requirements: Data privacy, consent, right to deletion",
            0.9,
        )],
    );
    engine.register(agent);
    providers.push(provider);

    // Remediation Proposal Agent: Synthesis
    let (agent, provider) = create_mock_llm_agent(
        "RemediationProposalAgent",
        "You propose remediation plans for compliance violations.",
        "Propose remediation: {context}",
        ContextKey::Evaluations,
        vec![ContextKey::Strategies],
        requirements::synthesis(),
        vec![MockResponse::success(
            "Remediation 1: GDPR | Plan: Implement data access logging, consent management system | Priority: URGENT",
            0.92,
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
    fn mock_llm_compliance_monitoring_converges() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regulations", "GDPR"));

        let _providers = setup_mock_llm_compliance_monitoring(&mut engine);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
    }
}
