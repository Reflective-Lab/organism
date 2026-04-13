// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Continuous Compliance Monitoring agents for regulatory compliance.
//!
//! This module implements continuous compliance monitoring as data changes,
//! demonstrating reactive agents and evidence collection patterns.
//!
//! # Agent Pipeline
//!
//! ```text
//! Seeds (regulations, data)
//!    │
//!    ├─► RegulationParserAgent → Signals (parsed regulations)
//!    ├─► PolicyRuleAgent → Constraints (policy rules)
//!    ├─► EvidenceCollectorAgent → Signals (evidence data)
//!    ├─► ViolationDetectorAgent → Strategies (violation reports)
//!    └─► RemediationProposalAgent → Evaluations (remediation plans)
//! ```

// Agent trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Agent that parses regulations (simulated LLM).
pub struct RegulationParserAgent;

impl Agent for RegulationParserAgent {
    fn name(&self) -> &str {
        "RegulationParserAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("regulation:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let regulations = if let Some(reg_seed) = seeds.iter().find(|s| s.id == "regulations") {
            reg_seed
                .content
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>()
        } else {
            vec!["GDPR", "SOC2", "HIPAA"]
        };

        for reg in regulations {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("regulation:{}", reg.to_lowercase()),
                content: format!(
                    "Regulation {}: Parsed | Requirements: {} | Applicable: Yes",
                    reg,
                    match reg {
                        "GDPR" => "Data privacy, consent, right to deletion",
                        "SOC2" => "Security controls, access management, monitoring",
                        "HIPAA" => "PHI protection, access controls, audit logs",
                        _ => "General compliance requirements",
                    }
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that creates policy rules from regulations.
pub struct PolicyRuleAgent;

impl Agent for PolicyRuleAgent {
    fn name(&self) -> &str {
        "PolicyRuleAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("regulation:"))
            && !ctx.has(ContextKey::Constraints)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for regulation in signals.iter().filter(|s| s.id.starts_with("regulation:")) {
            let reg_name = regulation
                .id
                .strip_prefix("regulation:")
                .unwrap_or("unknown");

            facts.push(Fact {
                key: ContextKey::Constraints,
                id: format!("policy:{reg_name}"),
                content: format!(
                    "Policy {}: {} | Enforcement: Automatic | Severity: High",
                    reg_name,
                    match reg_name {
                        "gdpr" =>
                            "All data access must be logged | Consent required for processing",
                        "soc2" => "Access controls must be enforced | All changes must be audited",
                        "hipaa" => "PHI access must be authorized | Audit trail required",
                        _ => "Compliance with regulation required",
                    }
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that collects evidence of compliance.
pub struct EvidenceCollectorAgent;

impl Agent for EvidenceCollectorAgent {
    fn name(&self) -> &str {
        "EvidenceCollectorAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Constraints)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("evidence:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let constraints = ctx.get(ContextKey::Constraints);
        let mut facts = Vec::new();

        for policy in constraints.iter().filter(|c| c.id.starts_with("policy:")) {
            let reg_name = policy.id.strip_prefix("policy:").unwrap_or("unknown");

            // Simulate evidence collection
            let has_evidence = !reg_name.contains("gdpr") || reg_name.contains("soc2"); // GDPR missing evidence

            facts.push(Fact {
                key: ContextKey::Signals,
                id: format!("evidence:{reg_name}"),
                content: format!(
                    "Evidence {}: {} | Status: {} | Last checked: Today | Next check: Tomorrow",
                    reg_name,
                    if has_evidence { "Found" } else { "Missing" },
                    if has_evidence {
                        "COMPLIANT"
                    } else {
                        "NON-COMPLIANT"
                    }
                ),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that detects violations.
pub struct ViolationDetectorAgent;

impl Agent for ViolationDetectorAgent {
    fn name(&self) -> &str {
        "ViolationDetectorAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let signals = ctx.get(ContextKey::Signals);
        let has_evidence = signals.iter().any(|s| s.id.starts_with("evidence:"));
        let has_constraints = ctx.has(ContextKey::Constraints);

        has_evidence && has_constraints && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let constraints = ctx.get(ContextKey::Constraints);
        let mut facts = Vec::new();

        for policy in constraints.iter().filter(|c| c.id.starts_with("policy:")) {
            let reg_name = policy.id.strip_prefix("policy:").unwrap_or("unknown");
            let evidence = signals
                .iter()
                .find(|s| s.id == format!("evidence:{reg_name}"));

            if let Some(ev) = evidence {
                if ev.content.contains("NON-COMPLIANT") || ev.content.contains("Missing") {
                    facts.push(Fact {
                        key: ContextKey::Strategies,
                        id: format!("violation:{reg_name}"),
                        content: format!("Violation {}: NON-COMPLIANT | Evidence: {} | Severity: High | Action required: Immediate", 
                            reg_name, ev.content),
                    });
                } else {
                    facts.push(Fact {
                        key: ContextKey::Strategies,
                        id: format!("violation:{reg_name}"),
                        content: format!(
                            "Violation {}: COMPLIANT | Evidence: {} | Status: No action needed",
                            reg_name, ev.content
                        ),
                    });
                }
            }
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that proposes remediation plans.
pub struct RemediationProposalAgent;

impl Agent for RemediationProposalAgent {
    fn name(&self) -> &str {
        "RemediationProposalAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut facts = Vec::new();

        let violations: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("violation:") && s.content.contains("NON-COMPLIANT"))
            .collect();

        if violations.is_empty() {
            facts.push(Fact {
                key: ContextKey::Evaluations,
                id: "eval:compliant".into(),
                content: "Status: COMPLIANT | All regulations satisfied | No remediation needed"
                    .into(),
            });
        } else {
            for (i, violation) in violations.iter().enumerate() {
                let reg_name = violation.id.strip_prefix("violation:").unwrap_or("unknown");

                facts.push(Fact {
                    key: ContextKey::Evaluations,
                    id: format!("eval:{}", i + 1),
                    content: format!(
                        "Remediation {}: {} | Plan: {} | Priority: {} | Timeline: {} | {}",
                        i + 1,
                        reg_name,
                        match reg_name {
                            "gdpr" => "Implement data access logging, consent management system",
                            "soc2" => "Enhance access controls, enable audit logging",
                            "hipaa" => "Implement PHI access controls, audit trail system",
                            _ => "Review and implement compliance controls",
                        },
                        "URGENT",
                        "30 days",
                        if i == 0 { "RECOMMENDED" } else { "ALTERNATIVE" }
                    ),
                });
            }
        }

        AgentEffect::with_facts(facts)
    }
}

// =============================================================================
// COMPLIANCE MONITORING INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: All violations must have remediation plans.
pub struct RequireRemediationPlans;

impl Invariant for RequireRemediationPlans {
    fn name(&self) -> &str {
        "require_remediation_plans"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let strategies = ctx.get(ContextKey::Strategies);
        let evaluations = ctx.get(ContextKey::Evaluations);

        let violations: Vec<_> = strategies
            .iter()
            .filter(|s| s.id.starts_with("violation:") && s.content.contains("NON-COMPLIANT"))
            .collect();

        if !violations.is_empty() {
            let has_remediation = evaluations
                .iter()
                .any(|e| e.content.contains("Remediation") && e.content.contains("RECOMMENDED"));

            if !has_remediation {
                return InvariantResult::Violated(Violation::new(format!(
                    "{} violations detected but no remediation plans provided",
                    violations.len()
                )));
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: All regulations must have evidence.
pub struct RequireEvidenceForAllRegulations;

impl Invariant for RequireEvidenceForAllRegulations {
    fn name(&self) -> &str {
        "require_evidence_for_all_regulations"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);
        let constraints = ctx.get(ContextKey::Constraints);

        // Only check if evidence collection has started
        let has_any_evidence = signals.iter().any(|s| s.id.starts_with("evidence:"));
        if !has_any_evidence {
            return InvariantResult::Ok; // Too early, skip check
        }

        let regulations: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("regulation:"))
            .map(|s| s.id.strip_prefix("regulation:").unwrap_or("unknown"))
            .collect();

        for reg in regulations {
            let has_evidence = signals.iter().any(|s| s.id == format!("evidence:{reg}"));
            let has_policy = constraints.iter().any(|c| c.id == format!("policy:{reg}"));

            if !has_evidence || !has_policy {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("regulation {reg} missing evidence or policy"),
                    vec![format!("regulation:{}", reg)],
                ));
            }
        }

        InvariantResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Engine;
    use converge_core::agents::SeedAgent;

    #[test]
    fn regulation_parsing_creates_signals() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regulations", "GDPR, SOC2"));
        engine.register(RegulationParserAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("regulation:")));
    }

    #[test]
    fn policy_rules_created_from_regulations() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regulations", "GDPR"));
        engine.register(RegulationParserAgent);
        engine.register(PolicyRuleAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Constraints));
    }

    #[test]
    fn violations_detected_when_evidence_missing() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regulations", "GDPR, SOC2"));
        engine.register(RegulationParserAgent);
        engine.register(PolicyRuleAgent);
        engine.register(EvidenceCollectorAgent);
        engine.register(ViolationDetectorAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));
        let strategies = result.context.get(ContextKey::Strategies);
        // Should have violations (GDPR missing evidence)
        assert!(strategies.iter().any(|s| s.id.starts_with("violation:")));
    }

    #[test]
    fn remediation_proposed_for_violations() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regulations", "GDPR"));
        engine.register(RegulationParserAgent);
        engine.register(PolicyRuleAgent);
        engine.register(EvidenceCollectorAgent);
        engine.register(ViolationDetectorAgent);
        engine.register(RemediationProposalAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));
        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
    }

    #[test]
    fn invariants_enforce_compliance() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("regulations", "GDPR, SOC2"));
        engine.register(RegulationParserAgent);
        engine.register(PolicyRuleAgent);
        engine.register(EvidenceCollectorAgent);
        engine.register(ViolationDetectorAgent);
        engine.register(RemediationProposalAgent);

        engine.register_invariant(RequireRemediationPlans);
        engine.register_invariant(RequireEvidenceForAllRegulations);

        let result = engine.run(Context::new());

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.converged);
    }

    #[test]
    fn handles_all_compliant_scenario() {
        let mut engine = Engine::new();
        // Use only SOC2 which has evidence
        engine.register(SeedAgent::new("regulations", "SOC2"));
        engine.register(RegulationParserAgent);
        engine.register(PolicyRuleAgent);
        engine.register(EvidenceCollectorAgent);
        engine.register(ViolationDetectorAgent);
        engine.register(RemediationProposalAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let evals = result.context.get(ContextKey::Evaluations);
        // Should indicate compliance
        assert!(
            evals
                .iter()
                .any(|e| e.content.contains("COMPLIANT") || evals.is_empty())
        );
    }

    #[test]
    fn full_pipeline_converges_deterministically() {
        let run = || {
            let mut engine = Engine::new();
            engine.register(SeedAgent::new("regulations", "GDPR, SOC2"));
            engine.register(RegulationParserAgent);
            engine.register(PolicyRuleAgent);
            engine.register(EvidenceCollectorAgent);
            engine.register(ViolationDetectorAgent);
            engine.register(RemediationProposalAgent);
            engine.run(Context::new()).expect("should converge")
        };

        let r1 = run();
        let r2 = run();

        assert_eq!(r1.cycles, r2.cycles);
        assert_eq!(
            r1.context.get(ContextKey::Evaluations),
            r2.context.get(ContextKey::Evaluations)
        );
    }
}
