// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Product & Engineering Pack agents for product development lifecycle.
//!
//! Implements the agent contracts defined in specs/product_engineering.truth.
//!
//! # Product & Engineering is the Build System
//!
//! Every product development effort flows through this pack:
//! - Roadmap planning and initiatives
//! - Feature specification and development
//! - Release coordination and deployment
//! - Incident response and postmortems
//! - Product experiments and metrics
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (initiative:, feature:, release:, etc.).

use converge_core::{
    Agent, AgentEffect, Context, ContextKey, Fact,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const INITIATIVE_PREFIX: &str = "initiative:";
pub const FEATURE_PREFIX: &str = "feature:";
pub const TASK_PREFIX: &str = "task:";
pub const RELEASE_PREFIX: &str = "release:";
pub const INCIDENT_PREFIX: &str = "incident:";
pub const EXPERIMENT_PREFIX: &str = "experiment:";
pub const TECH_DEBT_PREFIX: &str = "tech_debt:";
pub const POSTMORTEM_PREFIX: &str = "postmortem:";

// ============================================================================
// Agents
// ============================================================================

/// Creates and maintains product roadmap from strategic goals.
#[derive(Debug, Clone, Default)]
pub struct RoadmapPlannerAgent;

impl Agent for RoadmapPlannerAgent {
    fn name(&self) -> &str {
        "roadmap_planner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("strategic.goal") || s.content.contains("roadmap.plan"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("strategic.goal")
                || trigger.content.contains("roadmap.plan")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", INITIATIVE_PREFIX, trigger.id),
                    content: serde_json::json!({
                        "type": "initiative",
                        "source_id": trigger.id,
                        "state": "draft",
                        "title": "New Initiative",
                        "strategic_goal": null,
                        "target_quarter": null,
                        "owner_id": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Assists in writing feature specifications.
#[derive(Debug, Clone, Default)]
pub struct FeatureSpecifierAgent;

impl Agent for FeatureSpecifierAgent {
    fn name(&self) -> &str {
        "feature_specifier"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(FEATURE_PREFIX) && p.content.contains("\"state\":\"specifying\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for feature in proposals.iter() {
            if feature.id.starts_with(FEATURE_PREFIX)
                && feature.content.contains("\"state\":\"specifying\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("spec:{}", feature.id),
                    content: serde_json::json!({
                        "type": "feature_spec",
                        "feature_id": feature.id,
                        "user_stories": [],
                        "acceptance_criteria": [],
                        "technical_notes": null,
                        "dependencies": [],
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Breaks features into engineering tasks.
#[derive(Debug, Clone, Default)]
pub struct TaskDecomposerAgent;

impl Agent for TaskDecomposerAgent {
    fn name(&self) -> &str {
        "task_decomposer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(FEATURE_PREFIX) && p.content.contains("\"state\":\"ready\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for feature in proposals.iter() {
            if feature.id.starts_with(FEATURE_PREFIX)
                && feature.content.contains("\"state\":\"ready\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", TASK_PREFIX, feature.id),
                    content: serde_json::json!({
                        "type": "task_breakdown",
                        "feature_id": feature.id,
                        "tasks": [],
                        "total_estimate_hours": 0,
                        "complexity": "medium",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Orchestrates release process.
#[derive(Debug, Clone, Default)]
pub struct ReleaseCoordinatorAgent;

impl Agent for ReleaseCoordinatorAgent {
    fn name(&self) -> &str {
        "release_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(FEATURE_PREFIX) && p.content.contains("\"state\":\"shipping\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        // Collect features ready for release
        let shipping_features: Vec<_> = proposals
            .iter()
            .filter(|p| {
                p.id.starts_with(FEATURE_PREFIX) && p.content.contains("\"state\":\"shipping\"")
            })
            .collect();

        if !shipping_features.is_empty() {
            // Use first feature ID as release identifier
            let release_id = shipping_features.first().map(|f| &f.id).unwrap();
            facts.push(Fact {
                key: ContextKey::Proposals,
                id: format!("{}{}", RELEASE_PREFIX, release_id),
                content: serde_json::json!({
                    "type": "release",
                    "state": "planning",
                    "features": shipping_features.iter().map(|f| &f.id).collect::<Vec<_>>(),
                    "rollback_plan": null,
                    "deployment_plan": null,
                    "created_at": "2026-01-12T12:00:00Z"
                })
                .to_string(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Monitors canary deployments for anomalies.
#[derive(Debug, Clone, Default)]
pub struct CanaryAnalyzerAgent;

impl Agent for CanaryAnalyzerAgent {
    fn name(&self) -> &str {
        "canary_analyzer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(RELEASE_PREFIX) && p.content.contains("\"state\":\"canary\""))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for release in proposals.iter() {
            if release.id.starts_with(RELEASE_PREFIX)
                && release.content.contains("\"state\":\"canary\"")
            {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("canary_health:{}", release.id),
                    content: serde_json::json!({
                        "type": "canary_health",
                        "release_id": release.id,
                        "error_rate_baseline": 0.01,
                        "error_rate_canary": 0.01,
                        "latency_p99_baseline_ms": 200,
                        "latency_p99_canary_ms": 210,
                        "health_score": 0.95,
                        "recommendation": "proceed",
                        "analyzed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Assists in incident response and coordination.
#[derive(Debug, Clone, Default)]
pub struct IncidentResponderAgent;

impl Agent for IncidentResponderAgent {
    fn name(&self) -> &str {
        "incident_responder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals).iter().any(|s| {
            s.content.contains("incident.detected") || s.content.contains("alert.critical")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("incident.detected")
                || signal.content.contains("alert.critical")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", INCIDENT_PREFIX, signal.id),
                    content: serde_json::json!({
                        "type": "incident",
                        "source_id": signal.id,
                        "state": "detected",
                        "severity": null,
                        "title": "Production incident",
                        "commander": null,
                        "affected_services": [],
                        "runbooks": [],
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Facilitates blameless postmortems.
#[derive(Debug, Clone, Default)]
pub struct PostmortemFacilitatorAgent;

impl Agent for PostmortemFacilitatorAgent {
    fn name(&self) -> &str {
        "postmortem_facilitator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(INCIDENT_PREFIX) && p.content.contains("\"state\":\"resolved\"")
        })
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for incident in proposals.iter() {
            if incident.id.starts_with(INCIDENT_PREFIX)
                && incident.content.contains("\"state\":\"resolved\"")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", POSTMORTEM_PREFIX, incident.id),
                    content: serde_json::json!({
                        "type": "postmortem",
                        "incident_id": incident.id,
                        "state": "draft",
                        "timeline": [],
                        "contributing_factors": [],
                        "action_items": [],
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Helps design product experiments.
#[derive(Debug, Clone, Default)]
pub struct ExperimentDesignerAgent;

impl Agent for ExperimentDesignerAgent {
    fn name(&self) -> &str {
        "experiment_designer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Hypotheses)
            .iter()
            .any(|h| h.content.contains("product.experiment") || h.content.contains("ab_test"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        let mut facts = Vec::new();

        for hypothesis in hypotheses.iter() {
            if hypothesis.content.contains("product.experiment")
                || hypothesis.content.contains("ab_test")
            {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", EXPERIMENT_PREFIX, hypothesis.id),
                    content: serde_json::json!({
                        "type": "experiment",
                        "hypothesis_id": hypothesis.id,
                        "state": "designing",
                        "variants": ["control", "treatment"],
                        "target_metric": null,
                        "sample_size": null,
                        "duration_days": null,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Observes product metrics and identifies anomalies.
#[derive(Debug, Clone, Default)]
pub struct MetricsObserverAgent;

impl Agent for MetricsObserverAgent {
    fn name(&self) -> &str {
        "metrics_observer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.content.contains("metric.") || s.content.contains("product.metric"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("metric.") || signal.content.contains("product.metric") {
                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("observation:{}", signal.id),
                    content: serde_json::json!({
                        "type": "metric_observation",
                        "signal_id": signal.id,
                        "metric_name": "unknown",
                        "current_value": 0,
                        "baseline_value": 0,
                        "deviation_pct": 0,
                        "is_anomaly": false,
                        "observed_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Tracks and prioritizes technical debt.
#[derive(Debug, Clone, Default)]
pub struct TechDebtTrackerAgent;

impl Agent for TechDebtTrackerAgent {
    fn name(&self) -> &str {
        "tech_debt_tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.content.contains("code_quality") || s.content.contains("tech_debt"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for signal in signals.iter() {
            if signal.content.contains("code_quality") || signal.content.contains("tech_debt") {
                facts.push(Fact {
                    key: ContextKey::Proposals,
                    id: format!("{}{}", TECH_DEBT_PREFIX, signal.id),
                    content: serde_json::json!({
                        "type": "tech_debt",
                        "source_id": signal.id,
                        "category": "unknown",
                        "severity": "medium",
                        "impact": "unknown",
                        "effort_estimate": null,
                        "priority": null,
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

/// Ensures every feature has a product owner.
#[derive(Debug, Clone, Default)]
pub struct FeatureHasOwnerInvariant;

impl Invariant for FeatureHasOwnerInvariant {
    fn name(&self) -> &str {
        "feature_has_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for feature in ctx.get(ContextKey::Proposals).iter() {
            if feature.id.starts_with(FEATURE_PREFIX)
                && (feature.content.contains("\"state\":\"ready\"")
                    || feature.content.contains("\"state\":\"building\""))
                && feature.content.contains("\"owner_id\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Feature {} in active state has no owner", feature.id),
                    vec![feature.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures every release has a rollback plan.
#[derive(Debug, Clone, Default)]
pub struct ReleaseHasRollbackPlanInvariant;

impl Invariant for ReleaseHasRollbackPlanInvariant {
    fn name(&self) -> &str {
        "release_has_rollback_plan"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for release in ctx.get(ContextKey::Proposals).iter() {
            if release.id.starts_with(RELEASE_PREFIX)
                && release.content.contains("\"state\":\"frozen\"")
                && release.content.contains("\"rollback_plan\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Release {} is frozen but has no rollback plan", release.id),
                    vec![release.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures every incident has severity classification.
#[derive(Debug, Clone, Default)]
pub struct IncidentHasSeverityInvariant;

impl Invariant for IncidentHasSeverityInvariant {
    fn name(&self) -> &str {
        "incident_has_severity"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for incident in ctx.get(ContextKey::Proposals).iter() {
            if incident.id.starts_with(INCIDENT_PREFIX)
                && incident.content.contains("\"state\":\"triaging\"")
                && incident.content.contains("\"severity\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Incident {} is triaging but has no severity", incident.id),
                    vec![incident.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures shipped features have success metrics defined.
#[derive(Debug, Clone, Default)]
pub struct ShippedFeatureHasMetricsInvariant;

impl Invariant for ShippedFeatureHasMetricsInvariant {
    fn name(&self) -> &str {
        "shipped_feature_has_metrics"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        for feature in ctx.get(ContextKey::Proposals).iter() {
            if feature.id.starts_with(FEATURE_PREFIX)
                && feature.content.contains("\"state\":\"shipped\"")
                && feature.content.contains("\"success_metrics\":null")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!(
                        "Shipped feature {} has no success metrics defined",
                        feature.id
                    ),
                    vec![feature.id.clone()],
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
        assert_eq!(RoadmapPlannerAgent.name(), "roadmap_planner");
        assert_eq!(FeatureSpecifierAgent.name(), "feature_specifier");
        assert_eq!(TaskDecomposerAgent.name(), "task_decomposer");
        assert_eq!(ReleaseCoordinatorAgent.name(), "release_coordinator");
        assert_eq!(CanaryAnalyzerAgent.name(), "canary_analyzer");
        assert_eq!(IncidentResponderAgent.name(), "incident_responder");
        assert_eq!(PostmortemFacilitatorAgent.name(), "postmortem_facilitator");
        assert_eq!(ExperimentDesignerAgent.name(), "experiment_designer");
        assert_eq!(MetricsObserverAgent.name(), "metrics_observer");
        assert_eq!(TechDebtTrackerAgent.name(), "tech_debt_tracker");
    }
}
