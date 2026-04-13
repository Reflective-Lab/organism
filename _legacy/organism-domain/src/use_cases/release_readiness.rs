// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Release Readiness agents for engineering dependency and quality checks.
//!
//! This module implements a release readiness use case that demonstrates
//! parallel quality gates, consolidation, and explicit convergence criteria.
//!
//! # Agent Pipeline
//!
//! ```text
//! Seeds (release candidate, dependencies)
//!    │
//!    ├─► DependencyGraphAgent → Signals (dependency analysis)
//!    ├─► TestCoverageAgent → Signals (coverage metrics)
//!    ├─► SecurityScanAgent → Signals (vulnerability reports)
//!    ├─► PerformanceRegressionAgent → Signals (performance metrics)
//!    └─► DocumentationAgent → Signals (docs status)
//!    │
//!    ▼
//! RiskSummaryAgent → Strategies (risk assessments)
//!    │
//!    ▼
//! ReleaseReadyAgent → Evaluations (go/no-go decision)
//! ```
//!
//! # Example
//!
//! ```
//! use converge_core::{Engine, Context, ContextKey};
//! use converge_core::agents::SeedAgent;
//! use converge_domain::release_readiness::{
//!     DependencyGraphAgent, TestCoverageAgent, SecurityScanAgent,
//!     PerformanceRegressionAgent, DocumentationAgent, RiskSummaryAgent, ReleaseReadyAgent,
//! };
//!
//! let mut engine = Engine::new();
//! engine.register(SeedAgent::new("release:v1.2.0", "Release candidate v1.2.0"));
//! engine.register(DependencyGraphAgent);
//! engine.register(TestCoverageAgent);
//! engine.register(SecurityScanAgent);
//! engine.register(PerformanceRegressionAgent);
//! engine.register(DocumentationAgent);
//! engine.register(RiskSummaryAgent);
//! engine.register(ReleaseReadyAgent);
//!
//! let result = engine.run(Context::new()).expect("should converge");
//! assert!(result.converged);
//! ```

// Agent trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Agent that analyzes dependency graph for issues.
pub struct DependencyGraphAgent;

impl Agent for DependencyGraphAgent {
    fn name(&self) -> &str {
        "DependencyGraphAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("dependency:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        // Check if we have a release seed
        let has_release = seeds.iter().any(|s| s.id.contains("release"));

        if has_release {
            // Simulate dependency analysis
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "dependency:graph".into(),
                content:
                    "Dependency graph: 45 direct, 234 transitive | No circular deps | All pinned"
                        .into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "dependency:outdated".into(),
                content: "Outdated deps: 2 minor updates available | No security patches needed"
                    .into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that checks test coverage metrics.
pub struct TestCoverageAgent;

impl Agent for TestCoverageAgent {
    fn name(&self) -> &str {
        "TestCoverageAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("coverage:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let has_release = seeds.iter().any(|s| s.id.contains("release"));

        if has_release {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "coverage:unit".into(),
                content: "Unit test coverage: 87% | 1,234 tests | All passing".into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "coverage:integration".into(),
                content: "Integration test coverage: 72% | 89 tests | All passing".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that performs security vulnerability scanning.
pub struct SecurityScanAgent;

impl Agent for SecurityScanAgent {
    fn name(&self) -> &str {
        "SecurityScanAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("security:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let has_release = seeds.iter().any(|s| s.id.contains("release"));

        if has_release {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "security:vulnerabilities".into(),
                content: "Security scan: 0 critical, 2 medium, 5 low | All in dev deps only".into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "security:licenses".into(),
                content: "License check: All compatible | No GPL dependencies".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that checks for performance regressions.
pub struct PerformanceRegressionAgent;

impl Agent for PerformanceRegressionAgent {
    fn name(&self) -> &str {
        "PerformanceRegressionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("performance:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let has_release = seeds.iter().any(|s| s.id.contains("release"));

        if has_release {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "performance:benchmarks".into(),
                content: "Benchmarks: All within 5% of baseline | No regressions detected".into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "performance:memory".into(),
                content: "Memory usage: Stable | No leaks detected".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that verifies documentation completeness.
pub struct DocumentationAgent;

impl Agent for DocumentationAgent {
    fn name(&self) -> &str {
        "DocumentationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("docs:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        let has_release = seeds.iter().any(|s| s.id.contains("release"));

        if has_release {
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "docs:api".into(),
                content: "API docs: 95% coverage | All public APIs documented".into(),
            });
            facts.push(Fact {
                key: ContextKey::Signals,
                id: "docs:changelog".into(),
                content: "Changelog: Updated | Breaking changes documented".into(),
            });
        }

        AgentEffect::with_facts(facts)
    }
}

/// Agent that consolidates all checks into risk assessments.
pub struct RiskSummaryAgent;

impl Agent for RiskSummaryAgent {
    fn name(&self) -> &str {
        "RiskSummaryAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Wait until we have signals from all check agents
        let signals = ctx.get(ContextKey::Signals);
        let has_dependency = signals.iter().any(|s| s.id.starts_with("dependency:"));
        let has_coverage = signals.iter().any(|s| s.id.starts_with("coverage:"));
        let has_security = signals.iter().any(|s| s.id.starts_with("security:"));
        let has_performance = signals.iter().any(|s| s.id.starts_with("performance:"));
        let has_docs = signals.iter().any(|s| s.id.starts_with("docs:"));

        has_dependency
            && has_coverage
            && has_security
            && has_performance
            && has_docs
            && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        // Assess risks from each category
        let mut critical_risks = 0;
        let mut warnings = 0;

        for signal in signals {
            if signal.content.contains("critical") || signal.content.contains("Critical") {
                critical_risks += 1;
            }
            if signal.content.contains("warning") || signal.content.contains("Warning") {
                warnings += 1;
            }
        }

        // Check for specific risk indicators
        let has_vulnerabilities = signals
            .iter()
            .any(|s| s.content.contains("vulnerabilities") && s.content.contains("critical"));
        let has_test_failures = signals
            .iter()
            .any(|s| s.content.contains("failing") || s.content.contains("failed"));
        let has_perf_regression = signals
            .iter()
            .any(|s| s.content.contains("regression") && s.content.contains("performance"));

        let risk_level = if critical_risks > 0 || has_vulnerabilities || has_test_failures {
            "HIGH"
        } else if warnings > 2 || has_perf_regression {
            "MEDIUM"
        } else {
            "LOW"
        };

        facts.push(Fact {
            key: ContextKey::Strategies,
            id: "risk:summary".into(),
            content: format!(
                "Risk Assessment: {} | Critical: {} | Warnings: {} | Status: {}",
                risk_level,
                critical_risks,
                warnings,
                if risk_level == "HIGH" {
                    "BLOCKED"
                } else if risk_level == "MEDIUM" {
                    "REVIEW"
                } else {
                    "CLEAR"
                }
            ),
        });

        AgentEffect::with_facts(facts)
    }
}

/// Agent that makes the final go/no-go decision.
pub struct ReleaseReadyAgent;

impl Agent for ReleaseReadyAgent {
    fn name(&self) -> &str {
        "ReleaseReadyAgent"
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

        // Find risk summary
        let risk_summary = strategies
            .iter()
            .find(|s| s.id.starts_with("risk:"))
            .map_or("", |s| s.content.as_str());

        let is_ready = !risk_summary.contains("BLOCKED") && !risk_summary.contains("HIGH");

        let (status, rationale) = if is_ready {
            (
                "READY",
                "All checks passed | No critical risks | Ready for release",
            )
        } else {
            (
                "BLOCKED",
                "Critical issues detected | Release blocked until resolved",
            )
        };

        facts.push(Fact {
            key: ContextKey::Evaluations,
            id: "eval:release-readiness".into(),
            content: format!("Status: {status} | Rationale: {rationale}"),
        });

        AgentEffect::with_facts(facts)
    }
}

// =============================================================================
// RELEASE READINESS INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: All required checks must be completed.
pub struct RequireAllChecksComplete;

impl Invariant for RequireAllChecksComplete {
    fn name(&self) -> &str {
        "require_all_checks_complete"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);

        let required_checks = [
            "dependency:",
            "coverage:",
            "security:",
            "performance:",
            "docs:",
        ];

        let mut missing = Vec::new();
        for check in &required_checks {
            if !signals.iter().any(|s| s.id.starts_with(check)) {
                missing.push(check.strip_suffix(':').unwrap_or(check));
            }
        }

        if !missing.is_empty() {
            return InvariantResult::Violated(Violation::new(format!(
                "missing required checks: {}",
                missing.join(", ")
            )));
        }

        InvariantResult::Ok
    }
}

/// Structural invariant: No critical vulnerabilities allowed.
pub struct RequireNoCriticalVulnerabilities;

impl Invariant for RequireNoCriticalVulnerabilities {
    fn name(&self) -> &str {
        "require_no_critical_vulnerabilities"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);

        for signal in signals {
            if signal.id.starts_with("security:")
                && signal.content.contains("critical")
                && !signal.content.contains("0 critical")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    "critical security vulnerabilities detected",
                    vec![signal.id.clone()],
                ));
            }
        }

        InvariantResult::Ok
    }
}

/// Semantic invariant: Test coverage must meet minimum threshold.
pub struct RequireMinimumCoverage;

impl Invariant for RequireMinimumCoverage {
    fn name(&self) -> &str {
        "require_minimum_coverage"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &Context) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);
        const MIN_COVERAGE: u32 = 70;

        for signal in signals {
            if signal.id.starts_with("coverage:") {
                // Extract coverage percentage
                if let Some(percent_str) = signal
                    .content
                    .split('%')
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                {
                    if let Ok(coverage) = percent_str.parse::<u32>() {
                        if coverage < MIN_COVERAGE {
                            return InvariantResult::Violated(Violation::with_facts(
                                format!(
                                    "coverage {coverage}% below minimum threshold of {MIN_COVERAGE}%"
                                ),
                                vec![signal.id.clone()],
                            ));
                        }
                    }
                }
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
    fn all_check_agents_run_in_parallel() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("release:v1.0.0", "Release candidate"));
        engine.register(DependencyGraphAgent);
        engine.register(TestCoverageAgent);
        engine.register(SecurityScanAgent);
        engine.register(PerformanceRegressionAgent);
        engine.register(DocumentationAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("dependency:")));
        assert!(signals.iter().any(|s| s.id.starts_with("coverage:")));
        assert!(signals.iter().any(|s| s.id.starts_with("security:")));
        assert!(signals.iter().any(|s| s.id.starts_with("performance:")));
        assert!(signals.iter().any(|s| s.id.starts_with("docs:")));
    }

    #[test]
    fn risk_summary_waits_for_all_checks() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("release:v1.0.0", "Release candidate"));
        engine.register(DependencyGraphAgent);
        engine.register(TestCoverageAgent);
        engine.register(SecurityScanAgent);
        engine.register(PerformanceRegressionAgent);
        engine.register(DocumentationAgent);
        engine.register(RiskSummaryAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));
        let strategies = result.context.get(ContextKey::Strategies);
        assert!(strategies.iter().any(|s| s.id.starts_with("risk:")));
    }

    #[test]
    fn release_ready_agent_makes_decision() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("release:v1.0.0", "Release candidate"));
        engine.register(DependencyGraphAgent);
        engine.register(TestCoverageAgent);
        engine.register(SecurityScanAgent);
        engine.register(PerformanceRegressionAgent);
        engine.register(DocumentationAgent);
        engine.register(RiskSummaryAgent);
        engine.register(ReleaseReadyAgent);

        let result = engine.run(Context::new()).expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));
        let evals = result.context.get(ContextKey::Evaluations);
        assert!(evals.iter().any(|e| e.id == "eval:release-readiness"));
        assert!(
            evals
                .iter()
                .any(|e| e.content.contains("READY") || e.content.contains("BLOCKED"))
        );
    }

    #[test]
    fn invariants_enforce_quality_gates() {
        let mut engine = Engine::new();
        engine.register(SeedAgent::new("release:v1.0.0", "Release candidate"));
        engine.register(DependencyGraphAgent);
        engine.register(TestCoverageAgent);
        engine.register(SecurityScanAgent);
        engine.register(PerformanceRegressionAgent);
        engine.register(DocumentationAgent);
        engine.register(RiskSummaryAgent);
        engine.register(ReleaseReadyAgent);

        engine.register_invariant(RequireAllChecksComplete);
        engine.register_invariant(RequireNoCriticalVulnerabilities);
        engine.register_invariant(RequireMinimumCoverage);

        let result = engine.run(Context::new());

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.converged);
    }

    #[test]
    fn full_pipeline_converges_deterministically() {
        let run = || {
            let mut engine = Engine::new();
            engine.register(SeedAgent::new("release:v1.0.0", "Release candidate"));
            engine.register(DependencyGraphAgent);
            engine.register(TestCoverageAgent);
            engine.register(SecurityScanAgent);
            engine.register(PerformanceRegressionAgent);
            engine.register(DocumentationAgent);
            engine.register(RiskSummaryAgent);
            engine.register(ReleaseReadyAgent);
            engine.run(Context::new()).expect("should converge")
        };

        let r1 = run();
        let r2 = run();

        assert_eq!(r1.cycles, r2.cycles);
        assert_eq!(
            r1.context.get(ContextKey::Signals),
            r2.context.get(ContextKey::Signals)
        );
        assert_eq!(
            r1.context.get(ContextKey::Evaluations),
            r2.context.get(ContextKey::Evaluations)
        );
    }
}
