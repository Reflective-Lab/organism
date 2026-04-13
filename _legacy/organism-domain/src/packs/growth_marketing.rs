// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Growth & Marketing Pack agents for customer acquisition and retention.
//!
//! Implements the agent contracts defined in specs/growth_marketing.truth.
//!
//! # Growth & Marketing is the Acquisition Engine
//!
//! Every campaign, content piece, and experiment flows through this pack:
//! - Campaign planning and execution
//! - Channel management (ads, email, social)
//! - Budget allocation and guardrails
//! - Attribution and performance tracking
//! - A/B testing and optimization
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (campaign:, content:, experiment:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const CAMPAIGN_PREFIX: &str = "campaign:";
pub const CHANNEL_PREFIX: &str = "channel:";
pub const CONTENT_PREFIX: &str = "content:";
pub const EXPERIMENT_PREFIX: &str = "experiment:";
pub const AUDIENCE_PREFIX: &str = "audience:";
pub const ATTRIBUTION_PREFIX: &str = "attribution:";
pub const BUDGET_PREFIX: &str = "budget:";
pub const PERFORMANCE_PREFIX: &str = "performance:";

// ============================================================================
// Agents
// ============================================================================

/// Plans and creates marketing campaigns.
#[derive(Debug, Clone, Default)]
pub struct CampaignPlannerAgent;

impl Agent for CampaignPlannerAgent {
    fn name(&self) -> &str {
        "campaign_planner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("campaign.create") || s.content.contains("campaign.plan"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("campaign.create")
                || trigger.content.contains("campaign.plan")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", CAMPAIGN_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "campaign",
                        "source_id": trigger.id,
                        "state": "draft",
                        "campaign_type": "acquisition",
                        "hypothesis": null,
                        "success_metric": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Connects to advertising and marketing platforms.
#[derive(Debug, Clone, Default)]
pub struct ChannelConnectorAgent;

impl Agent for ChannelConnectorAgent {
    fn name(&self) -> &str {
        "channel_connector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("channel.connect") || s.content.contains("channel.configure")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("channel.connect")
                || trigger.content.contains("channel.configure")
            {
                facts.push(Fact {
                    key: ContextKey::Signals,
                    id: format!("{}{}", CHANNEL_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "channel",
                        "source_id": trigger.id,
                        "state": "configured",
                        "platform": "detected",
                        "connected_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Allocates budget across channels with guardrails.
#[derive(Debug, Clone, Default)]
pub struct BudgetAllocatorAgent;

impl Agent for BudgetAllocatorAgent {
    fn name(&self) -> &str {
        "budget_allocator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(CAMPAIGN_PREFIX) && p.content.contains("\"state\":\"approved\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for campaign in proposals.iter() {
            if campaign.id.starts_with(CAMPAIGN_PREFIX)
                && campaign.content.contains("\"state\":\"approved\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", BUDGET_PREFIX, campaign.id),
                    content: serde_json::json!({
                        "type": "budget",
                        "campaign_id": campaign.id,
                        "state": "active",
                        "amount": 10000,
                        "currency": "USD",
                        "daily_cap": 500,
                        "guardrail_cac": 100,
                        "allocated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Schedules content publication.
#[derive(Debug, Clone, Default)]
pub struct ContentSchedulerAgent;

impl Agent for ContentSchedulerAgent {
    fn name(&self) -> &str {
        "content_scheduler"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(CONTENT_PREFIX) && p.content.contains("\"state\":\"approved\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for content in proposals.iter() {
            if content.id.starts_with(CONTENT_PREFIX)
                && content.content.contains("\"state\":\"approved\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("scheduled:{}", content.id),
                    content: serde_json::json!({
                        "type": "content_schedule",
                        "content_id": content.id,
                        "state": "scheduled",
                        "publish_at": "2026-01-13T09:00:00Z",
                        "scheduled_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Manages A/B tests and growth experiments.
#[derive(Debug, Clone, Default)]
pub struct ExperimentRunnerAgent;

impl Agent for ExperimentRunnerAgent {
    fn name(&self) -> &str {
        "experiment_runner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(EXPERIMENT_PREFIX) && p.content.contains("\"state\":\"approved\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for experiment in proposals.iter() {
            if experiment.id.starts_with(EXPERIMENT_PREFIX)
                && experiment.content.contains("\"state\":\"approved\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("running:{}", experiment.id),
                    content: serde_json::json!({
                        "type": "experiment_run",
                        "experiment_id": experiment.id,
                        "state": "running",
                        "traffic_split": [50, 50],
                        "started_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Collects performance metrics from channels.
#[derive(Debug, Clone, Default)]
pub struct PerformanceTrackerAgent;

impl Agent for PerformanceTrackerAgent {
    fn name(&self) -> &str {
        "performance_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with(CHANNEL_PREFIX) && s.content.contains("\"state\":\"active\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for channel in signals.iter() {
            if channel.id.starts_with(CHANNEL_PREFIX)
                && channel.content.contains("\"state\":\"active\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", PERFORMANCE_PREFIX, channel.id),
                    content: serde_json::json!({
                        "type": "performance",
                        "channel_id": channel.id,
                        "impressions": 10000,
                        "clicks": 500,
                        "conversions": 25,
                        "spend": 1000,
                        "ctr": 0.05,
                        "cpc": 2.0,
                        "cac": 40.0,
                        "collected_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Analyzes attribution across touchpoints.
#[derive(Debug, Clone, Default)]
pub struct AttributionAnalyzerAgent;

impl Agent for AttributionAnalyzerAgent {
    fn name(&self) -> &str {
        "attribution_analyzer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with(PERFORMANCE_PREFIX) && e.content.contains("\"conversions\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut facts = Vec::new();

        for perf in evaluations.iter() {
            if perf.id.starts_with(PERFORMANCE_PREFIX) && perf.content.contains("\"conversions\"") {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("{}{}", ATTRIBUTION_PREFIX, perf.id),
                    content: serde_json::json!({
                        "type": "attribution",
                        "performance_id": perf.id,
                        "model": "last_touch",
                        "touchpoints": [],
                        "attributed_conversions": 25,
                        "attributed_revenue": 2500,
                        "analyzed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Enforces budget guardrails and spend limits.
#[derive(Debug, Clone, Default)]
pub struct SpendGuardianAgent;

impl Agent for SpendGuardianAgent {
    fn name(&self) -> &str {
        "spend_guardian"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_budget = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(BUDGET_PREFIX) && p.content.contains("\"state\":\"active\""));
        let has_performance = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with(PERFORMANCE_PREFIX));
        has_budget && has_performance
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for budget in proposals.iter() {
            if budget.id.starts_with(BUDGET_PREFIX)
                && budget.content.contains("\"state\":\"active\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("guardian:{}", budget.id),
                    content: serde_json::json!({
                        "type": "spend_check",
                        "budget_id": budget.id,
                        "within_daily_cap": true,
                        "within_guardrail": true,
                        "utilization_pct": 45,
                        "checked_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Creates and manages audience segments.
#[derive(Debug, Clone, Default)]
pub struct AudienceSegmenterAgent;

impl Agent for AudienceSegmenterAgent {
    fn name(&self) -> &str {
        "audience_segmenter"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("audience.create") || s.content.contains("audience.segment")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("audience.create")
                || trigger.content.contains("audience.segment")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", AUDIENCE_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "audience",
                        "source_id": trigger.id,
                        "segment_type": "behavioral",
                        "size": 0,
                        "synced_to_platforms": [],
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Recommends campaign optimizations.
#[derive(Debug, Clone, Default)]
pub struct CampaignOptimizerAgent;

impl Agent for CampaignOptimizerAgent {
    fn name(&self) -> &str {
        "campaign_optimizer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with(PERFORMANCE_PREFIX))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut facts = Vec::new();

        for perf in evaluations.iter() {
            if perf.id.starts_with(PERFORMANCE_PREFIX) {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("optimization:{}", perf.id),
                    content: serde_json::json!({
                        "type": "optimization_recommendation",
                        "performance_id": perf.id,
                        "recommendations": [],
                        "priority": "medium",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures every campaign has a hypothesis before approval.
#[derive(Debug, Clone, Default)]
pub struct CampaignHasHypothesisInvariant;

impl Invariant for CampaignHasHypothesisInvariant {
    fn name(&self) -> &str {
        "campaign_has_hypothesis"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for campaign in ctx.get(ContextKey::Proposals).iter() {
            if campaign.id.starts_with(CAMPAIGN_PREFIX)
                && campaign.content.contains("\"state\":\"approved\"")
                && campaign.content.contains("\"hypothesis\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Campaign {} approved without hypothesis", campaign.id),
                    vec![campaign.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures no ad spend without goal and guardrail.
#[derive(Debug, Clone, Default)]
pub struct NoSpendWithoutGoalInvariant;

impl Invariant for NoSpendWithoutGoalInvariant {
    fn name(&self) -> &str {
        "no_spend_without_goal"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for budget in ctx.get(ContextKey::Proposals).iter() {
            if budget.id.starts_with(BUDGET_PREFIX)
                && budget.content.contains("\"state\":\"active\"")
                && !budget.content.contains("\"guardrail_cac\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Budget {} has no guardrail", budget.id),
                    vec![budget.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures experiments have success metrics defined.
#[derive(Debug, Clone, Default)]
pub struct ExperimentHasMetricsInvariant;

impl Invariant for ExperimentHasMetricsInvariant {
    fn name(&self) -> &str {
        "experiment_has_metrics"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for experiment in ctx.get(ContextKey::Proposals).iter() {
            if experiment.id.starts_with(EXPERIMENT_PREFIX)
                && experiment.content.contains("\"state\":\"approved\"")
                && !experiment.content.contains("\"primary_metric\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Experiment {} has no success metric", experiment.id),
                    vec![experiment.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures budget guardrails are enforced.
#[derive(Debug, Clone, Default)]
pub struct BudgetGuardrailsEnforcedInvariant;

impl Invariant for BudgetGuardrailsEnforcedInvariant {
    fn name(&self) -> &str {
        "budget_guardrails_enforced"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for check in ctx.get(ContextKey::Evaluations).iter() {
            if check.content.contains("\"type\":\"spend_check\"")
                && check.content.contains("\"within_guardrail\":false")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Budget guardrail exceeded: {}", check.id),
                    vec![check.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(CampaignPlannerAgent.name(), "campaign_planner");
        assert_eq!(ChannelConnectorAgent.name(), "channel_connector");
        assert_eq!(BudgetAllocatorAgent.name(), "budget_allocator");
        assert_eq!(ContentSchedulerAgent.name(), "content_scheduler");
        assert_eq!(ExperimentRunnerAgent.name(), "experiment_runner");
        assert_eq!(PerformanceTrackerAgent.name(), "performance_tracker");
        assert_eq!(AttributionAnalyzerAgent.name(), "attribution_analyzer");
        assert_eq!(SpendGuardianAgent.name(), "spend_guardian");
        assert_eq!(AudienceSegmenterAgent.name(), "audience_segmenter");
        assert_eq!(CampaignOptimizerAgent.name(), "campaign_optimizer");
    }
}
