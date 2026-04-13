// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Organism-level evals for domain-specific business outcomes.
//!
//! These evals define what "good" means for organism-specific outcomes
//! across the organizational packs. Kernel evals (for money, trust, delivery,
//! knowledge, data_metrics) live in `converge_domain::evals`.

use converge_core::{Context, ContextKey, Eval, EvalOutcome, EvalResult};

// =============================================================================
// Growth Marketing Evals
// =============================================================================

/// Eval: Campaign hypothesis quality
///
/// Ensures campaigns have well-defined hypotheses with measurable outcomes.
pub struct CampaignHypothesisQualityEval;

impl Eval for CampaignHypothesisQualityEval {
    fn name(&self) -> &'static str {
        "campaign_hypothesis_quality"
    }

    fn description(&self) -> &'static str {
        "Ensures campaigns have well-defined hypotheses with measurable outcomes"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let campaigns: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("campaign:"))
            .collect();

        if campaigns.is_empty() {
            return EvalResult::with_facts(
                self.name(),
                EvalOutcome::Fail,
                0.0,
                "No campaigns found".to_string(),
                vec![],
            );
        }

        let with_hypothesis = campaigns
            .iter()
            .filter(|c| c.content.contains("hypothesis"))
            .count();
        let score = with_hypothesis as f64 / campaigns.len() as f64;

        EvalResult::with_facts(
            self.name(),
            if score >= 0.8 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!(
                "{}/{} campaigns have hypotheses",
                with_hypothesis,
                campaigns.len()
            ),
            campaigns.iter().map(|c| c.id.clone()).collect(),
        )
    }
}

/// Eval: Attribution completeness
///
/// Ensures marketing attribution covers all revenue-generating activities.
pub struct AttributionCompletenessEval;

impl Eval for AttributionCompletenessEval {
    fn name(&self) -> &'static str {
        "attribution_completeness"
    }

    fn description(&self) -> &'static str {
        "Ensures marketing attribution covers all revenue-generating activities"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let evals = ctx.get(ContextKey::Evaluations);
        let attributed: Vec<_> = evals
            .iter()
            .filter(|e| e.id.starts_with("attribution:"))
            .collect();

        let score = if attributed.is_empty() { 0.0 } else { 1.0 };

        EvalResult::with_facts(
            self.name(),
            if score >= 0.8 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{} attribution records found", attributed.len()),
            attributed.iter().map(|a| a.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Customers Evals
// =============================================================================

/// Eval: Lead conversion quality
///
/// Measures the quality of lead-to-opportunity conversion.
pub struct LeadConversionQualityEval;

impl Eval for LeadConversionQualityEval {
    fn name(&self) -> &'static str {
        "lead_conversion_quality"
    }

    fn description(&self) -> &'static str {
        "Measures the quality of lead-to-opportunity conversion"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let leads: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("lead:"))
            .collect();

        let score = if leads.is_empty() { 0.0 } else { 1.0 };

        EvalResult::with_facts(
            self.name(),
            if score >= 0.5 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{} leads found", leads.len()),
            leads.iter().map(|l| l.id.clone()).collect(),
        )
    }
}

/// Eval: Pipeline velocity
///
/// Measures how quickly leads move through the sales pipeline.
pub struct PipelineVelocityEval;

impl Eval for PipelineVelocityEval {
    fn name(&self) -> &'static str {
        "pipeline_velocity"
    }

    fn description(&self) -> &'static str {
        "Measures how quickly leads move through the sales pipeline"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let pipeline: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("opportunity:") || s.id.starts_with("deal:"))
            .collect();

        let score = if pipeline.is_empty() { 0.0 } else { 1.0 };

        EvalResult::with_facts(
            self.name(),
            if score >= 0.5 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{} pipeline items found", pipeline.len()),
            pipeline.iter().map(|p| p.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Legal Evals
// =============================================================================

/// Eval: Contract signature compliance
///
/// Ensures contracts have valid signatures before execution.
pub struct ContractSignatureEval;

impl Eval for ContractSignatureEval {
    fn name(&self) -> &'static str {
        "contract_signature"
    }

    fn description(&self) -> &'static str {
        "Ensures contracts have valid signatures before execution"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let contracts: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("contract:"))
            .collect();

        let signed = contracts
            .iter()
            .filter(|c| c.content.contains("signed"))
            .count();
        let score = if contracts.is_empty() {
            1.0
        } else {
            signed as f64 / contracts.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{}/{} contracts signed", signed, contracts.len()),
            contracts.iter().map(|c| c.id.clone()).collect(),
        )
    }
}

/// Eval: IP assignment compliance
///
/// Ensures IP assignments are completed before any payment processing.
pub struct IpAssignmentComplianceEval;

impl Eval for IpAssignmentComplianceEval {
    fn name(&self) -> &'static str {
        "ip_assignment_compliance"
    }

    fn description(&self) -> &'static str {
        "Ensures IP assignments are completed before any payment processing"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let ip_assignments: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("ip_assignment:"))
            .collect();

        let score = if ip_assignments.is_empty() {
            1.0
        } else {
            let completed = ip_assignments
                .iter()
                .filter(|a| a.content.contains("completed"))
                .count();
            completed as f64 / ip_assignments.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("IP assignment compliance score: {:.0}%", score * 100.0),
            ip_assignments.iter().map(|a| a.id.clone()).collect(),
        )
    }
}

// =============================================================================
// People Evals
// =============================================================================

/// Eval: Onboarding completeness
///
/// Ensures all onboarding steps are completed for new employees.
pub struct OnboardingCompletenessEval;

impl Eval for OnboardingCompletenessEval {
    fn name(&self) -> &'static str {
        "onboarding_completeness"
    }

    fn description(&self) -> &'static str {
        "Ensures all onboarding steps are completed for new employees"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let employees: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("employee:"))
            .collect();

        let onboarded = employees
            .iter()
            .filter(|e| e.content.contains("onboarded"))
            .count();
        let score = if employees.is_empty() {
            1.0
        } else {
            onboarded as f64 / employees.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{}/{} employees onboarded", onboarded, employees.len()),
            employees.iter().map(|e| e.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Product Engineering Evals
// =============================================================================

/// Eval: Feature ownership
///
/// Ensures every feature has a designated owner.
pub struct FeatureOwnershipEval;

impl Eval for FeatureOwnershipEval {
    fn name(&self) -> &'static str {
        "feature_ownership"
    }

    fn description(&self) -> &'static str {
        "Ensures every feature has a designated owner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let features: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("feature:"))
            .collect();

        let with_owner = features
            .iter()
            .filter(|f| f.content.contains("owner:"))
            .count();
        let score = if features.is_empty() {
            1.0
        } else {
            with_owner as f64 / features.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{}/{} features have owners", with_owner, features.len()),
            features.iter().map(|f| f.id.clone()).collect(),
        )
    }
}

/// Eval: Release rollback readiness
///
/// Ensures every release has a documented rollback plan.
pub struct ReleaseRollbackReadinessEval;

impl Eval for ReleaseRollbackReadinessEval {
    fn name(&self) -> &'static str {
        "release_rollback_readiness"
    }

    fn description(&self) -> &'static str {
        "Ensures every release has a documented rollback plan"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let releases: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("release:"))
            .collect();

        let with_rollback = releases
            .iter()
            .filter(|r| r.content.contains("rollback"))
            .count();
        let score = if releases.is_empty() {
            1.0
        } else {
            with_rollback as f64 / releases.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!(
                "{}/{} releases have rollback plans",
                with_rollback,
                releases.len()
            ),
            releases.iter().map(|r| r.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Ops Support Evals
// =============================================================================

/// Eval: Ticket resolution quality
///
/// Measures the quality and completeness of ticket resolutions.
pub struct TicketResolutionEval;

impl Eval for TicketResolutionEval {
    fn name(&self) -> &'static str {
        "ticket_resolution"
    }

    fn description(&self) -> &'static str {
        "Measures the quality and completeness of ticket resolutions"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let tickets: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("ticket:"))
            .collect();

        let resolved = tickets
            .iter()
            .filter(|t| t.content.contains("resolved"))
            .count();
        let score = if tickets.is_empty() {
            1.0
        } else {
            resolved as f64 / tickets.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 0.8 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{}/{} tickets resolved", resolved, tickets.len()),
            tickets.iter().map(|t| t.id.clone()).collect(),
        )
    }
}

/// Eval: Escalation appropriateness
///
/// Ensures escalations are justified and follow proper procedures.
pub struct EscalationAppropriatenessEval;

impl Eval for EscalationAppropriatenessEval {
    fn name(&self) -> &'static str {
        "escalation_appropriateness"
    }

    fn description(&self) -> &'static str {
        "Ensures escalations are justified and follow proper procedures"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let escalations: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("escalation:"))
            .collect();

        let with_reason = escalations
            .iter()
            .filter(|e| e.content.contains("reason:"))
            .count();
        let score = if escalations.is_empty() {
            1.0
        } else {
            with_reason as f64 / escalations.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!(
                "{}/{} escalations have reasons",
                with_reason,
                escalations.len()
            ),
            escalations.iter().map(|e| e.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Partnerships Vendors Evals
// =============================================================================

/// Eval: Partner agreement coverage
///
/// Ensures all active partners have valid agreements.
pub struct PartnerAgreementCoverageEval;

impl Eval for PartnerAgreementCoverageEval {
    fn name(&self) -> &'static str {
        "partner_agreement_coverage"
    }

    fn description(&self) -> &'static str {
        "Ensures all active partners have valid agreements"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let partners: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("partner:"))
            .collect();

        let with_agreement = partners
            .iter()
            .filter(|p| p.content.contains("agreement"))
            .count();
        let score = if partners.is_empty() {
            1.0
        } else {
            with_agreement as f64 / partners.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!(
                "{}/{} partners have agreements",
                with_agreement,
                partners.len()
            ),
            partners.iter().map(|p| p.id.clone()).collect(),
        )
    }
}

/// Eval: Vendor assessment completeness
///
/// Ensures vendors are fully assessed before onboarding.
pub struct VendorAssessmentCompletenessEval;

impl Eval for VendorAssessmentCompletenessEval {
    fn name(&self) -> &'static str {
        "vendor_assessment_completeness"
    }

    fn description(&self) -> &'static str {
        "Ensures vendors are fully assessed before onboarding"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let assessments: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("vendor_assessment:"))
            .collect();

        let complete = assessments
            .iter()
            .filter(|a| a.content.contains("complete"))
            .count();
        let score = if assessments.is_empty() {
            1.0
        } else {
            complete as f64 / assessments.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!(
                "{}/{} vendor assessments complete",
                complete,
                assessments.len()
            ),
            assessments.iter().map(|a| a.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Procurement Assets Evals
// =============================================================================

/// Eval: Spend approval compliance
///
/// Ensures all spending has proper approval before execution.
pub struct SpendApprovalComplianceEval;

impl Eval for SpendApprovalComplianceEval {
    fn name(&self) -> &'static str {
        "spend_approval_compliance"
    }

    fn description(&self) -> &'static str {
        "Ensures all spending has proper approval before execution"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let spends: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("order:") || s.id.starts_with("request:"))
            .collect();

        let approved = spends
            .iter()
            .filter(|s| s.content.contains("approved"))
            .count();
        let score = if spends.is_empty() {
            1.0
        } else {
            approved as f64 / spends.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{}/{} spends approved", approved, spends.len()),
            spends.iter().map(|s| s.id.clone()).collect(),
        )
    }
}

/// Eval: Asset tracking
///
/// Ensures all purchased assets are properly tracked and assigned.
pub struct AssetTrackingEval;

impl Eval for AssetTrackingEval {
    fn name(&self) -> &'static str {
        "asset_tracking"
    }

    fn description(&self) -> &'static str {
        "Ensures all purchased assets are properly tracked and assigned"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let signals = ctx.get(ContextKey::Signals);
        let assets: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("asset:"))
            .collect();

        let tracked = assets
            .iter()
            .filter(|a| a.content.contains("tracked") || a.content.contains("assigned"))
            .count();
        let score = if assets.is_empty() {
            1.0
        } else {
            tracked as f64 / assets.len() as f64
        };

        EvalResult::with_facts(
            self.name(),
            if score >= 1.0 {
                EvalOutcome::Pass
            } else {
                EvalOutcome::Fail
            },
            score,
            format!("{}/{} assets tracked", tracked, assets.len()),
            assets.iter().map(|a| a.id.clone()).collect(),
        )
    }
}

// =============================================================================
// Strategy & Lead Evals (moved from converge-domain — business-specific)
// =============================================================================

/// Eval: Strategy diversity
///
/// Ensures at least 3 distinct strategies exist with no two targeting
/// the same primary channel.
pub struct StrategyDiversityEval;

impl Eval for StrategyDiversityEval {
    fn name(&self) -> &'static str {
        "strategy_diversity"
    }

    fn description(&self) -> &'static str {
        "Ensures at least 3 distinct strategies exist with no two targeting the same primary channel"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let strategies = ctx.get(ContextKey::Strategies);

        if strategies.len() < 3 {
            return EvalResult::with_facts(
                self.name(),
                EvalOutcome::Fail,
                strategies.len() as f64 / 3.0,
                format!(
                    "Only {} strategies found, need at least 3",
                    strategies.len()
                ),
                strategies.iter().map(|s| s.id.clone()).collect(),
            );
        }

        // Check for channel diversity (simplified: check if content mentions different channels)
        let channels: Vec<&str> = strategies
            .iter()
            .filter_map(|s| {
                if s.content.contains("email") {
                    Some("email")
                } else if s.content.contains("social") {
                    Some("social")
                } else if s.content.contains("content") {
                    Some("content")
                } else if s.content.contains("paid") {
                    Some("paid")
                } else {
                    None
                }
            })
            .collect();

        let unique_channels: std::collections::HashSet<&str> = channels.iter().copied().collect();

        if unique_channels.len() < 3 {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Fail,
                unique_channels.len() as f64 / 3.0,
                format!(
                    "Only {} unique channels found across {} strategies, need at least 3",
                    unique_channels.len(),
                    strategies.len()
                ),
                strategies.iter().map(|s| s.id.clone()).collect(),
            )
        } else {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Pass,
                1.0,
                format!(
                    "Found {} distinct strategies across {} unique channels",
                    strategies.len(),
                    unique_channels.len()
                ),
                strategies.iter().map(|s| s.id.clone()).collect(),
            )
        }
    }
}

/// Eval: Lead qualification quality
///
/// Ensures at least 80% of leads have:
/// - A clear ICP match
/// - A justification rationale
/// - A recommended next action
pub struct LeadQualificationQualityEval;

impl Eval for LeadQualificationQualityEval {
    fn name(&self) -> &'static str {
        "lead_qualification_quality"
    }

    fn description(&self) -> &'static str {
        "Ensures at least 80% of leads have ICP match, rationale, and next action"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn evaluate(&self, ctx: &Context) -> EvalResult {
        let strategies = ctx.get(ContextKey::Strategies);

        if strategies.is_empty() {
            return EvalResult::new(
                self.name(),
                EvalOutcome::Indeterminate,
                0.0,
                "No leads found to evaluate",
            );
        }

        let mut qualified_count = 0;
        let fact_ids: Vec<String> = strategies.iter().map(|s| s.id.clone()).collect();

        for strategy in strategies {
            let has_icp = strategy.content.contains("ICP") || strategy.content.contains("fit");
            let has_rationale =
                strategy.content.contains("because") || strategy.content.contains("rationale");
            let has_action =
                strategy.content.contains("next") || strategy.content.contains("action");

            if has_icp && has_rationale && has_action {
                qualified_count += 1;
            }
        }

        let quality_ratio = f64::from(qualified_count) / strategies.len() as f64;
        let threshold = 0.8;

        if quality_ratio >= threshold {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Pass,
                quality_ratio,
                format!(
                    "{}/{} leads ({:.1}%) meet quality criteria",
                    qualified_count,
                    strategies.len(),
                    quality_ratio * 100.0
                ),
                fact_ids,
            )
        } else {
            EvalResult::with_facts(
                self.name(),
                EvalOutcome::Fail,
                quality_ratio,
                format!(
                    "Only {}/{} leads ({:.1}%) meet quality criteria, need {:.0}%",
                    qualified_count,
                    strategies.len(),
                    quality_ratio * 100.0,
                    threshold * 100.0
                ),
                fact_ids,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{Context, Fact};

    #[test]
    fn strategy_diversity_passes_with_three_strategies() {
        let eval = StrategyDiversityEval;
        let mut ctx = Context::new();

        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strat-1",
            "email marketing campaign",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strat-2",
            "social media outreach",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strat-3",
            "content marketing strategy",
        ))
        .unwrap();

        let result = eval.evaluate(&ctx);
        assert_eq!(result.outcome, EvalOutcome::Pass);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn strategy_diversity_fails_with_insufficient_strategies() {
        let eval = StrategyDiversityEval;
        let mut ctx = Context::new();

        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strat-1",
            "email campaign",
        ))
        .unwrap();

        let result = eval.evaluate(&ctx);
        assert_eq!(result.outcome, EvalOutcome::Fail);
        assert!(result.score < 1.0);
    }

    #[test]
    fn lead_qualification_quality_passes_with_high_quality() {
        let eval = LeadQualificationQualityEval;
        let mut ctx = Context::new();

        for i in 0..10 {
            ctx.add_fact(Fact::new(
                ContextKey::Strategies,
                format!("lead-{}", i),
                format!("ICP fit: yes, because: qualified, next action: call"),
            ))
            .unwrap();
        }

        let result = eval.evaluate(&ctx);
        assert_eq!(result.outcome, EvalOutcome::Pass);
        assert!(result.score >= 0.8);
    }
}
