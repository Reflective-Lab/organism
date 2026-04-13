// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! LLM-enabled HR Policy Alignment use case.
//!
//! This module demonstrates how to set up LLM agents with model selection
//! for the HR Policy Alignment use case. It follows the Converge pattern:
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

// NOTE: setup_llm_hr_policy_alignment (using ProviderRegistry + create_llm_agent) is temporarily
// disabled. converge-provider's LlmProvider trait diverged from converge-core's in core 1.0.2.
// See REF-36 for context.

/// Sets up LLM-enabled HR Policy Alignment agents with mock providers (for testing).
///
/// Returns the mock providers so you can configure their responses.
#[must_use]
pub fn setup_mock_llm_hr_policy_alignment(engine: &mut Engine) -> Vec<Arc<MockProvider>> {
    let mut providers = Vec::new();

    // Policy Distribution Agent
    let (dist_agent, dist_provider) = create_mock_llm_agent(
        "PolicyDistributionAgent",
        "You are an HR policy distribution agent.",
        "Identify affected employees: {context}",
        ContextKey::Signals,
        vec![ContextKey::Seeds],
        requirements::fast_extraction(),
        vec![MockResponse::success(
            "employee:emp-001: Affected by policy | Role: Engineer | Manager: mgr-001\nemployee:emp-002: Affected by policy | Role: Sales | Manager: mgr-002",
            0.8,
        )],
    );
    engine.register(dist_agent);
    providers.push(dist_provider);

    // Understanding Signal Agent
    let (understanding_agent, understanding_provider) = create_mock_llm_agent(
        "UnderstandingSignalAgent",
        "You are an HR understanding signal analyst.",
        "Analyze understanding: {context}",
        ContextKey::Signals,
        vec![ContextKey::Signals],
        requirements::analysis(),
        vec![MockResponse::success(
            "understanding:emp-001: clarified | Signal: Question answered\nunderstanding:emp-002: clear | Signal: No action needed",
            0.8,
        )],
    );
    engine.register(understanding_agent);
    providers.push(understanding_provider);

    // Manager Follow-Up Agent
    let (followup_agent, followup_provider) = create_mock_llm_agent(
        "ManagerFollowUpAgent",
        "You are an HR manager follow-up coordinator.",
        "Schedule meetings: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals],
        requirements::synthesis(),
        vec![MockResponse::success(
            "meeting:scheduled:emp-003: Employee: emp-003 | Manager: mgr-001 | Type: 1-on-1 | Status: Scheduled",
            0.8,
        )],
    );
    engine.register(followup_agent);
    providers.push(followup_provider);

    // Escalation Agent
    let (escalation_agent, escalation_provider) = create_mock_llm_agent(
        "EscalationAgent",
        "You are an HR escalation analyst.",
        "Identify escalations: {context}",
        ContextKey::Strategies,
        vec![ContextKey::Signals, ContextKey::Strategies],
        requirements::deep_research(),
        vec![MockResponse::success(
            "escalation:emp-003: No acknowledgement | Priority: High | Action: HR intervention required",
            0.8,
        )],
    );
    engine.register(escalation_agent);
    providers.push(escalation_provider);

    // Alignment Status Agent
    let (status_agent, status_provider) = create_mock_llm_agent(
        "AlignmentStatusAgent",
        "You are an HR alignment evaluator.",
        "Evaluate alignment: {context}",
        ContextKey::Evaluations,
        vec![ContextKey::Signals, ContextKey::Strategies],
        requirements::deep_research(),
        vec![MockResponse::success(
            "alignment-status: PENDING_ESCALATION | Total: 3 | Acknowledged: 2 | Clear: 2 | Escalations: 1 | Convergence: No",
            0.8,
        )],
    );
    engine.register(status_agent);
    providers.push(status_provider);

    providers
}
