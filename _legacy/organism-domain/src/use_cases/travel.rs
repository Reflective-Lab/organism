// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Travel pack agents and minimal end-to-end pipeline.
//!
//! This module is intentionally minimal and deterministic to enable
//! early debugging and breakpoint-driven development.
//!
//! # Pipeline
//!
//! ```text
//! Seeds (mission request, policy)
//!    │
//!    ▼
//! TripIntakeAgent → Signals (normalized trip)
//!    │
//!    ▼
//! InventorySearchAgent → Hypotheses (itinerary candidates)
//!    │
//!    ▼
//! PolicyGateAgent → Constraints (policy result)
//!    │
//!    ▼
//! PreferenceScoringAgent → Strategies (ranked itineraries)
//!    │
//!    ▼
//! HoldStrategyAgent → Evaluations (hold decision)
//!    │
//!    ▼
//! BookingDecisionAgent → Evaluations (booking decision)
//!    │
//!    ▼
//! AuditTrailAgent → Diagnostic (audit events)
//! ```
//!
//! # Notes
//! - No real bookings are performed; booking is always mocked.
//! - Each agent logs its inputs and outputs for step-through debugging.

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};
use tracing::info;

fn has_output(ctx: &Context, key: ContextKey, prefix: &str) -> bool {
    ctx.get(key).iter().any(|fact| fact.id.starts_with(prefix))
}

/// Normalizes the trip request into a canonical signal.
pub struct TripIntakeAgent;

impl Agent for TripIntakeAgent {
    fn name(&self) -> &str {
        "TripIntakeAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds) && !has_output(ctx, ContextKey::Signals, "travel:trip:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        info!(
            agent = self.name(),
            count = seeds.len(),
            "Normalizing trip request"
        );

        let content = seeds
            .iter()
            .find(|fact| fact.id == "travel_request")
            .map(|fact| fact.content.clone())
            .unwrap_or_else(|| "travel request: unspecified".to_string());

        let facts = vec![Fact {
            key: ContextKey::Signals,
            id: "travel:trip:normalized".into(),
            content,
        }];

        AgentEffect::with_facts(facts)
    }
}

/// Generates itinerary candidates from the normalized trip.
pub struct InventorySearchAgent;

impl Agent for InventorySearchAgent {
    fn name(&self) -> &str {
        "InventorySearchAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Signals)
            && !has_output(ctx, ContextKey::Hypotheses, "itinerary:candidate:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        info!(
            agent = self.name(),
            count = signals.len(),
            "Searching inventory (mock)"
        );

        let facts = vec![
            Fact {
                key: ContextKey::Hypotheses,
                id: "itinerary:candidate:1".into(),
                content: "Candidate 1: ARN → ICN → DXB → ARN".into(),
            },
            Fact {
                key: ContextKey::Hypotheses,
                id: "itinerary:candidate:2".into(),
                content: "Candidate 2: ARN → ICN → ARN (no stopover)".into(),
            },
        ];

        AgentEffect::with_facts(facts)
    }
}

/// Applies policy constraints to candidates.
pub struct PolicyGateAgent;

impl Agent for PolicyGateAgent {
    fn name(&self) -> &str {
        "PolicyGateAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Hypotheses)
            && !has_output(ctx, ContextKey::Constraints, "policy:decision:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        info!(
            agent = self.name(),
            count = hypotheses.len(),
            "Evaluating policy"
        );

        let decision = Fact {
            key: ContextKey::Constraints,
            id: "policy:decision:ok".into(),
            content: "Policy check: ok (mock)".into(),
        };

        AgentEffect::with_facts(vec![decision])
    }
}

/// Ranks candidates by preferences.
pub struct PreferenceScoringAgent;

impl Agent for PreferenceScoringAgent {
    fn name(&self) -> &str {
        "PreferenceScoringAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Hypotheses)
            && ctx.has(ContextKey::Constraints)
            && !has_output(ctx, ContextKey::Strategies, "itinerary:ranked:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        info!(
            agent = self.name(),
            count = hypotheses.len(),
            "Scoring itinerary preferences"
        );

        let ranked = Fact {
            key: ContextKey::Strategies,
            id: "itinerary:ranked:1".into(),
            content: "Ranked: Candidate 1 preferred (mock)".into(),
        };

        AgentEffect::with_facts(vec![ranked])
    }
}

/// Decides whether to hold inventory.
pub struct HoldStrategyAgent;

impl Agent for HoldStrategyAgent {
    fn name(&self) -> &str {
        "HoldStrategyAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies) && !has_output(ctx, ContextKey::Evaluations, "hold:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        info!(
            agent = self.name(),
            count = strategies.len(),
            "Applying hold strategy"
        );

        let hold = Fact {
            key: ContextKey::Evaluations,
            id: "hold:local".into(),
            content: "Hold inventory for Candidate 1 (mock)".into(),
        };

        AgentEffect::with_facts(vec![hold])
    }
}

/// Chooses the booking decision (mocked).
pub struct BookingDecisionAgent;

impl Agent for BookingDecisionAgent {
    fn name(&self) -> &str {
        "BookingDecisionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Evaluations) && !has_output(ctx, ContextKey::Evaluations, "booking:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evals = ctx.get(ContextKey::Evaluations);
        info!(
            agent = self.name(),
            count = evals.len(),
            "Emitting booking decision (mock)"
        );

        let decision = Fact {
            key: ContextKey::Evaluations,
            id: "booking:mocked".into(),
            content: "Booking decision: mock confirm Candidate 1".into(),
        };

        AgentEffect::with_facts(vec![decision])
    }
}

/// Emits audit events into the diagnostic stream.
pub struct AuditTrailAgent;

impl Agent for AuditTrailAgent {
    fn name(&self) -> &str {
        "AuditTrailAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Evaluations) && !has_output(ctx, ContextKey::Diagnostic, "audit:")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evals = ctx.get(ContextKey::Evaluations);
        info!(
            agent = self.name(),
            count = evals.len(),
            "Writing audit trail"
        );

        let audit = Fact {
            key: ContextKey::Diagnostic,
            id: "audit:travel:decision".into(),
            content: "Audit: booking decision recorded (mock)".into(),
        };

        AgentEffect::with_facts(vec![audit])
    }
}
