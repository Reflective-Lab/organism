// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! SDR Sales Runtime — Qualification and Outreach
//!
//! This module implements agents for the SDR Sales use case:
//! - Lead discovery (market scanning, signal extraction, deduplication)
//! - Qualification (fit, timing, need, risk evidence)
//! - Message strategy generation
//! - Channel and timing decisions
//! - Human-in-the-loop approval gates
//! - Execution and learning

use converge_core::{Agent, AgentEffect, Context, ContextKey};

/// Market Scan Agent — Scans market for candidate companies
///
/// Reads: Seeds (ICP definition)
/// Writes: Signals (candidate leads)
pub struct MarketScanAgent {
    name: String,
}

impl MarketScanAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for MarketScanAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Precondition: ICP definition must exist in Seeds
        ctx.has(ContextKey::Seeds)
            && ctx
                .get(ContextKey::Seeds)
                .iter()
                .any(|f| f.content.contains("ICP") || f.content.contains("ideal customer"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract ICP from Seeds
        let icp_info: Vec<String> = ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.content.contains("ICP") || f.content.contains("ideal customer"))
            .map(|f| f.content.clone())
            .collect();

        if icp_info.is_empty() {
            return AgentEffect::empty();
        }

        // Generate candidate leads based on ICP
        // In production, this would call APIs, scrapers, or LLMs
        let candidates = vec![
            "Company A - B2B SaaS, 50-200 employees, Nordic region",
            "Company B - Tech company, 100-500 employees, uses competitor X",
            "Company C - B2B software, 20-100 employees, recently hired Head of RevOps",
        ];

        let facts: Vec<converge_core::Fact> = candidates
            .into_iter()
            .enumerate()
            .map(|(i, candidate)| converge_core::Fact {
                key: ContextKey::Signals,
                id: format!("{}-candidate-{}", self.name, i),
                content: format!("Candidate lead: {candidate}"),
            })
            .collect();

        AgentEffect::with_facts(facts)
    }
}

/// Signal Extraction Agent — Extracts weak signals from candidate leads
///
/// Reads: Signals (candidate leads)
/// Writes: Signals (enhanced with signals)
pub struct SignalExtractionAgent {
    name: String,
}

impl SignalExtractionAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for SignalExtractionAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Precondition: candidate leads must exist
        ctx.has(ContextKey::Signals)
            && ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|f| f.id.contains("candidate"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract signals from candidate leads
        let candidates: Vec<&converge_core::Fact> = ctx
            .get(ContextKey::Signals)
            .iter()
            .filter(|f| f.id.contains("candidate"))
            .collect();

        let mut signals = Vec::new();

        for candidate in candidates {
            // Extract weak signals (in production, this would use LLMs or scrapers)
            if candidate.content.contains("recently hired") {
                signals.push(converge_core::Fact {
                    key: ContextKey::Signals,
                    id: format!("{}-signal-{}", self.name, candidate.id),
                    content: format!("Timing signal: {}", candidate.content),
                });
            }
            if candidate.content.contains("uses competitor") {
                signals.push(converge_core::Fact {
                    key: ContextKey::Signals,
                    id: format!("{}-signal-{}", self.name, candidate.id),
                    content: format!("Competitive signal: {}", candidate.content),
                });
            }
        }

        AgentEffect::with_facts(signals)
    }
}

/// Deduplication Agent — Removes duplicates and normalizes identities
///
/// Reads: Signals (candidate leads, signals)
/// Writes: Signals (deduplicated)
pub struct DeduplicationAgent {
    name: String,
}

impl DeduplicationAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for DeduplicationAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Signals)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Simple deduplication: mark duplicates (in production, use fuzzy matching)
        let signals = ctx.get(ContextKey::Signals);
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();

        for signal in signals {
            // Extract company name (simplified)
            let company = signal.content.split(" - ").next().unwrap_or("").to_string();

            if !seen.contains(&company) && !company.is_empty() {
                seen.insert(company.clone());
                deduped.push(converge_core::Fact {
                    key: ContextKey::Signals,
                    id: format!("{}-deduped-{}", self.name, company),
                    content: format!("Deduplicated: {}", signal.content),
                });
            }
        }

        AgentEffect::with_facts(deduped)
    }
}

/// Fit Evidence Agent — Checks ICP match
///
/// Reads: Signals (candidate leads)
/// Writes: Hypotheses (fit evidence)
pub struct FitEvidenceAgent {
    name: String,
}

impl FitEvidenceAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for FitEvidenceAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Precondition: candidate leads and ICP definition must exist
        ctx.has(ContextKey::Signals)
            && ctx.has(ContextKey::Seeds)
            && ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|f| f.id.contains("deduped"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract ICP criteria from Seeds
        let icp_criteria: Vec<String> = ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.content.contains("ICP") || f.content.contains("ideal customer"))
            .map(|f| f.content.clone())
            .collect();

        // Extract deduplicated leads
        let leads: Vec<&converge_core::Fact> = ctx
            .get(ContextKey::Signals)
            .iter()
            .filter(|f| f.id.contains("deduped"))
            .collect();

        let mut evidence = Vec::new();

        for lead in leads {
            // Simple ICP matching (in production, use LLMs or rules)
            let mut match_score = 0;
            let mut match_reasons = Vec::new();

            if icp_criteria
                .iter()
                .any(|c| c.contains("B2B") && lead.content.contains("B2B"))
            {
                match_score += 1;
                match_reasons.push("B2B match");
            }
            if icp_criteria
                .iter()
                .any(|c| c.contains("Nordic") && lead.content.contains("Nordic"))
            {
                match_score += 1;
                match_reasons.push("Geography match");
            }
            if icp_criteria
                .iter()
                .any(|c| c.contains("50-200") || c.contains("100-500"))
            {
                match_score += 1;
                match_reasons.push("Company size match");
            }

            if match_score > 0 {
                evidence.push(converge_core::Fact {
                    key: ContextKey::Hypotheses,
                    id: format!("{}-fit-{}", self.name, lead.id),
                    content: format!(
                        "Fit evidence: {} matches (reasons: {})",
                        match_score,
                        match_reasons.join(", ")
                    ),
                });
            }
        }

        AgentEffect::with_facts(evidence)
    }
}

/// Timing Evidence Agent — Finds recent events
///
/// Reads: Signals (candidate leads)
/// Writes: Hypotheses (timing evidence)
pub struct TimingEvidenceAgent {
    name: String,
}

impl TimingEvidenceAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for TimingEvidenceAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Signals)
            && ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|f| f.content.contains("recently") || f.content.contains("Timing signal"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract timing signals
        let timing_signals: Vec<&converge_core::Fact> = ctx
            .get(ContextKey::Signals)
            .iter()
            .filter(|f| f.content.contains("recently") || f.content.contains("Timing signal"))
            .collect();

        let mut evidence = Vec::new();

        for signal in timing_signals {
            evidence.push(converge_core::Fact {
                key: ContextKey::Hypotheses,
                id: format!("{}-timing-{}", self.name, signal.id),
                content: format!("Timing evidence: {}", signal.content),
            });
        }

        AgentEffect::with_facts(evidence)
    }
}

/// Need Evidence Agent — Identifies pain signals
///
/// Reads: Signals (candidate leads)
/// Writes: Hypotheses (need evidence)
pub struct NeedEvidenceAgent {
    name: String,
}

impl NeedEvidenceAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for NeedEvidenceAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Signals)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract need signals (in production, use LLMs or scrapers)
        let signals = ctx.get(ContextKey::Signals);
        let mut evidence = Vec::new();

        for signal in signals {
            // Simple need detection (in production, use LLMs)
            if signal.content.contains("Head of RevOps") {
                evidence.push(converge_core::Fact {
                    key: ContextKey::Hypotheses,
                    id: format!("{}-need-{}", self.name, signal.id),
                    content: "Need evidence: Hiring RevOps suggests operational pain".to_string(),
                });
            }
            if signal.content.contains("uses competitor") {
                evidence.push(converge_core::Fact {
                    key: ContextKey::Hypotheses,
                    id: format!("{}-need-{}", self.name, signal.id),
                    content: "Need evidence: Using competitor suggests active evaluation"
                        .to_string(),
                });
            }
        }

        AgentEffect::with_facts(evidence)
    }
}

/// Risk Evidence Agent — Flags brand mismatches and compliance issues
///
/// Reads: Signals (candidate leads), Seeds (constraints)
/// Writes: Hypotheses (risk evidence)
pub struct RiskEvidenceAgent {
    name: String,
}

impl RiskEvidenceAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for RiskEvidenceAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Signals)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract brand safety constraints from Seeds
        let constraints: Vec<String> = ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.content.contains("brand") || f.content.contains("compliance"))
            .map(|f| f.content.clone())
            .collect();

        let signals = ctx.get(ContextKey::Signals);
        let mut evidence = Vec::new();

        for signal in signals {
            // Simple risk detection (in production, use LLMs or rules)
            // For now, assume no risks (in production, check against blacklists, compliance rules)
            if constraints
                .iter()
                .any(|c| c.contains("no gambling") && signal.content.contains("casino"))
            {
                evidence.push(converge_core::Fact {
                    key: ContextKey::Hypotheses,
                    id: format!("{}-risk-{}", self.name, signal.id),
                    content: "Risk evidence: Brand mismatch detected".to_string(),
                });
            }
        }

        AgentEffect::with_facts(evidence)
    }
}

/// Message Hypothesis Agent — Generates message angles
///
/// Reads: Hypotheses (qualification evidence)
/// Writes: Strategies (message hypotheses)
pub struct MessageHypothesisAgent {
    name: String,
}

impl MessageHypothesisAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for MessageHypothesisAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Precondition: qualification evidence must exist
        ctx.has(ContextKey::Hypotheses)
            && ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id.contains("fit") || f.id.contains("timing") || f.id.contains("need"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract value propositions from Seeds
        let _value_props: Vec<String> = ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.content.contains("value") || f.content.contains("proposition"))
            .map(|f| f.content.clone())
            .collect();

        // Extract qualification evidence
        let evidence: Vec<&converge_core::Fact> = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .filter(|f| f.id.contains("fit") || f.id.contains("timing") || f.id.contains("need"))
            .collect();

        let mut strategies = Vec::new();

        for ev in evidence {
            // Generate message angles (in production, use LLMs)
            if ev.content.contains("Timing") {
                strategies.push(converge_core::Fact {
                    key: ContextKey::Strategies,
                    id: format!("{}-message-{}", self.name, ev.id),
                    content: "Message angle: Recent event suggests urgency - focus on timing and opportunity".to_string(),
                });
            }
            if ev.content.contains("Need") {
                strategies.push(converge_core::Fact {
                    key: ContextKey::Strategies,
                    id: format!("{}-message-{}", self.name, ev.id),
                    content: "Message angle: Pain signal detected - focus on problem-solving"
                        .to_string(),
                });
            }
            if ev.content.contains("Fit") {
                strategies.push(converge_core::Fact {
                    key: ContextKey::Strategies,
                    id: format!("{}-message-{}", self.name, ev.id),
                    content: "Message angle: Strong ICP match - focus on value proposition"
                        .to_string(),
                });
            }
        }

        AgentEffect::with_facts(strategies)
    }
}

/// Channel Decision Agent — Evaluates cost vs. value per channel
///
/// Reads: Strategies (message hypotheses), Constraints
/// Writes: Evaluations (channel decisions)
pub struct ChannelDecisionAgent {
    name: String,
}

impl ChannelDecisionAgent {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl Agent for ChannelDecisionAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        // Extract constraints
        let constraints = ctx.get(ContextKey::Constraints);
        let _calls_per_day = constraints
            .iter()
            .find(|f| f.content.contains("calls/day"))
            .and_then(|f| f.content.split(':').nth(1))
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(10);

        let strategies = ctx.get(ContextKey::Strategies);
        let mut evaluations = Vec::new();

        for strategy in strategies {
            // Simple channel decision (in production, use LLMs or optimizers)
            // For high-confidence strategies, recommend call; otherwise email
            let channel = if strategy.content.contains("Strong") {
                "call"
            } else {
                "email"
            };

            evaluations.push(converge_core::Fact {
                key: ContextKey::Evaluations,
                id: format!("{}-channel-{}", self.name, strategy.id),
                content: format!(
                    "Channel decision: {channel} (cost: medium, value: high, rationale: message confidence)"
                ),
            });
        }

        AgentEffect::with_facts(evaluations)
    }
}

// ============================================================================
// Invariants
// ============================================================================

use converge_core::invariant::{Invariant, InvariantClass, InvariantResult, Violation};

/// Structural invariant: Require valid ICP definition
pub struct RequireValidICP;

impl Invariant for RequireValidICP {
    fn name(&self) -> &'static str {
        "require_valid_icp"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let seeds = ctx.get(ContextKey::Seeds);
        let has_icp = seeds
            .iter()
            .any(|f| f.content.contains("ICP") || f.content.contains("ideal customer"));

        if !has_icp {
            return InvariantResult::Violated(Violation::new(
                "ICP definition must exist in Seeds".to_string(),
            ));
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: Require evidence for qualification
pub struct RequireQualificationEvidence {
    min_evidence_categories: usize,
}

impl RequireQualificationEvidence {
    #[must_use]
    pub fn new(min_evidence_categories: usize) -> Self {
        Self {
            min_evidence_categories,
        }
    }
}

impl Invariant for RequireQualificationEvidence {
    fn name(&self) -> &'static str {
        "require_qualification_evidence"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let hypotheses = ctx.get(ContextKey::Hypotheses);

        // Count evidence categories per lead
        let mut evidence_by_lead: std::collections::HashMap<
            String,
            std::collections::HashSet<String>,
        > = std::collections::HashMap::new();

        for hyp in hypotheses {
            // Extract lead ID from hypothesis ID
            let lead_id = hyp.id.split('-').nth(1).unwrap_or("unknown").to_string();

            // Determine evidence category
            let category = if hyp.id.contains("fit") {
                "fit"
            } else if hyp.id.contains("timing") {
                "timing"
            } else if hyp.id.contains("need") {
                "need"
            } else if hyp.id.contains("risk") {
                "risk"
            } else {
                "unknown"
            };

            evidence_by_lead
                .entry(lead_id)
                .or_default()
                .insert(category.to_string());
        }

        // Check if any lead has insufficient evidence
        for (lead_id, categories) in &evidence_by_lead {
            if categories.len() < self.min_evidence_categories {
                return InvariantResult::Violated(Violation::new(format!(
                    "Lead {} has only {} evidence categories, need at least {}",
                    lead_id,
                    categories.len(),
                    self.min_evidence_categories
                )));
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: Require message strategy before contact
pub struct RequireMessageStrategy {
    _min_confidence: f64,
}

impl RequireMessageStrategy {
    #[must_use]
    pub fn new(min_confidence: f64) -> Self {
        Self {
            _min_confidence: min_confidence,
        }
    }
}

impl Invariant for RequireMessageStrategy {
    fn name(&self) -> &'static str {
        "require_message_strategy"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let evaluations = ctx.get(ContextKey::Evaluations);

        // Check if any evaluation (channel decision) exists without a strategy
        for eval in evaluations {
            if eval.id.contains("channel") {
                // Find corresponding strategy
                let has_strategy = strategies.iter().any(|s| {
                    eval.id.contains(&s.id)
                        || s.id.contains(eval.id.split('-').nth(1).unwrap_or(""))
                });

                if !has_strategy {
                    return InvariantResult::Violated(Violation::new(format!(
                        "Channel decision {} has no corresponding message strategy",
                        eval.id
                    )));
                }
            }
        }

        InvariantResult::Ok
    }
}

/// Acceptance invariant: Require explicit qualification decisions
pub struct RequireExplicitQualification;

impl Invariant for RequireExplicitQualification {
    fn name(&self) -> &'static str {
        "require_explicit_qualification"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        let evaluations = ctx.get(ContextKey::Evaluations);

        // Extract unique leads from hypotheses
        let mut leads: std::collections::HashSet<String> = std::collections::HashSet::new();
        for hyp in hypotheses {
            let lead_id = hyp.id.split('-').nth(1).unwrap_or("unknown").to_string();
            leads.insert(lead_id);
        }

        // Check if all leads have explicit status (qualified, rejected, or stalled)
        // In a full implementation, this would check for explicit status facts
        // For now, we just verify that leads with evaluations have strategies
        for lead_id in &leads {
            let has_evaluation = evaluations.iter().any(|e| e.id.contains(lead_id));
            let has_strategy = ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|s| s.id.contains(lead_id));

            // If a lead has an evaluation but no strategy, it's ambiguous
            if has_evaluation && !has_strategy {
                return InvariantResult::Violated(Violation::new(format!(
                    "Lead {lead_id} has evaluation but no strategy - ambiguous state"
                )));
            }
        }

        InvariantResult::Ok
    }
}
