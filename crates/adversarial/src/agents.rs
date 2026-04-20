//! Adversarial agents — Suggestors that challenge candidate plans.
//!
//! Each agent reads `ContextKey::Strategies`, analyzes plans for weaknesses
//! in its domain, and proposes `ContextKey::Constraints` (blocking) or
//! `ContextKey::Evaluations` (passed scrutiny).

use crate::{Finding, Severity};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

// ── Assumption Breaker ────────────────────────────────────────────

/// Extracts unstated assumptions from plan annotations and challenges each.
/// Plans without explicit assumptions are flagged as higher risk.
pub struct AssumptionBreakerAgent;

impl AssumptionBreakerAgent {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn analyze_assumptions(plan: &serde_json::Value) -> Vec<Finding> {
        let mut findings = Vec::new();

        let assumptions = plan
            .get("annotation")
            .and_then(|a| a.get("assumptions"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let evidence = plan
            .get("annotation")
            .and_then(|a| a.get("evidence"))
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if assumptions.is_empty() {
            findings.push(Finding {
                agent: "assumption-breaker".into(),
                severity: Severity::Warning,
                message: "no assumptions declared — plan may have hidden dependencies".into(),
            });
        }

        for assumption in &assumptions {
            // Check if any evidence supports this assumption
            let supported = evidence
                .iter()
                .any(|e| e.to_lowercase().contains(&assumption.to_lowercase()));

            if !supported {
                findings.push(Finding {
                    agent: "assumption-breaker".into(),
                    severity: Severity::Warning,
                    message: format!("unsupported assumption: '{assumption}'"),
                });
            }
        }

        // Check for implicit assumptions based on plan content
        let content = plan.to_string().to_lowercase();
        let implicit_indicators = [
            ("always", "assumes stable conditions"),
            ("never fails", "assumes perfect reliability"),
            ("guaranteed", "assumes certain outcome"),
            ("trivial", "underestimates complexity"),
            ("obvious", "assumes shared understanding"),
        ];

        for (indicator, assumption) in implicit_indicators {
            if content.contains(indicator) {
                findings.push(Finding {
                    agent: "assumption-breaker".into(),
                    severity: Severity::Advisory,
                    message: format!(
                        "implicit assumption detected: {assumption} (keyword: '{indicator}')"
                    ),
                });
            }
        }

        findings
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for AssumptionBreakerAgent {
    fn name(&self) -> &'static str {
        "assumption-breaker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();

        for fact in strategies {
            let plan_json: serde_json::Value = serde_json::from_str(&fact.content)
                .unwrap_or_else(|_| serde_json::json!({"description": fact.content}));

            let findings = Self::analyze_assumptions(&plan_json);
            let has_blockers = findings.iter().any(|f| f.severity == Severity::Blocker);
            let has_warnings = findings.iter().any(|f| f.severity == Severity::Warning);

            let messages: Vec<String> = findings.iter().map(|f| f.message.clone()).collect();

            if has_blockers {
                proposals.push(ProposedFact::new(
                    ContextKey::Constraints,
                    format!("assumption-block-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "assumption-breaker",
                        "kind": "assumption_breaking",
                        "severity": "blocker",
                        "findings": messages,
                    })
                    .to_string(),
                    "assumption-breaker",
                ));
            } else if has_warnings {
                proposals.push(ProposedFact::new(
                    ContextKey::Evaluations,
                    format!("assumption-warn-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "assumption-breaker",
                        "kind": "assumption_breaking",
                        "passed": true,
                        "warnings": messages,
                    })
                    .to_string(),
                    "assumption-breaker",
                ));
            } else {
                proposals.push(ProposedFact::new(
                    ContextKey::Evaluations,
                    format!("assumption-pass-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "assumption-breaker",
                        "kind": "assumption_breaking",
                        "passed": true,
                        "findings": messages,
                    })
                    .to_string(),
                    "assumption-breaker",
                ));
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Constraint Checker ────────────────────────────────────────────

/// Validates plans against organizational constraints and policies.
/// Checks authority levels, budget limits, compliance requirements.
pub struct ConstraintCheckerAgent {
    org_constraints: Vec<OrgConstraint>,
}

#[derive(Debug, Clone)]
pub struct OrgConstraint {
    pub name: String,
    pub check: ConstraintCheck,
}

#[derive(Debug, Clone)]
pub enum ConstraintCheck {
    MaxBudget(f64),
    RequiredApproval(String),
    ForbiddenAction(String),
    RequiredTag(String),
}

impl ConstraintCheckerAgent {
    #[must_use]
    pub fn new(constraints: Vec<OrgConstraint>) -> Self {
        Self {
            org_constraints: constraints,
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            org_constraints: Vec::new(),
        }
    }

    fn check_plan(&self, plan: &serde_json::Value) -> Vec<Finding> {
        let mut findings = Vec::new();

        let total_cost: f64 = plan
            .get("annotation")
            .and_then(|a| a.get("costs"))
            .and_then(|c| c.as_array())
            .map_or(0.0, |arr| {
                arr.iter()
                    .filter_map(|v| v.get("estimate").and_then(serde_json::Value::as_f64))
                    .sum()
            });

        let actions: Vec<String> = plan
            .get("annotation")
            .and_then(|a| a.get("actions"))
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let tags: Vec<String> = plan
            .get("annotation")
            .and_then(|a| a.get("tags"))
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for constraint in &self.org_constraints {
            match &constraint.check {
                ConstraintCheck::MaxBudget(limit) => {
                    if total_cost > *limit {
                        findings.push(Finding {
                            agent: "constraint-checker".into(),
                            severity: Severity::Blocker,
                            message: format!(
                                "violates '{}': cost {:.0} exceeds limit {:.0}",
                                constraint.name, total_cost, limit,
                            ),
                        });
                    }
                }
                ConstraintCheck::RequiredApproval(approver) => {
                    let approvals: Vec<String> = plan
                        .get("annotation")
                        .and_then(|a| a.get("approvals"))
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    if !approvals.iter().any(|a| a == approver) {
                        findings.push(Finding {
                            agent: "constraint-checker".into(),
                            severity: Severity::Blocker,
                            message: format!(
                                "violates '{}': requires approval from '{approver}'",
                                constraint.name,
                            ),
                        });
                    }
                }
                ConstraintCheck::ForbiddenAction(action) => {
                    if actions.iter().any(|a| a.contains(action.as_str())) {
                        findings.push(Finding {
                            agent: "constraint-checker".into(),
                            severity: Severity::Blocker,
                            message: format!(
                                "violates '{}': forbidden action '{action}'",
                                constraint.name,
                            ),
                        });
                    }
                }
                ConstraintCheck::RequiredTag(tag) => {
                    if !tags.iter().any(|t| t == tag) {
                        findings.push(Finding {
                            agent: "constraint-checker".into(),
                            severity: Severity::Warning,
                            message: format!(
                                "missing tag for '{}': requires '{tag}'",
                                constraint.name,
                            ),
                        });
                    }
                }
            }
        }

        findings
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for ConstraintCheckerAgent {
    fn name(&self) -> &'static str {
        "constraint-checker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();

        for fact in strategies {
            let plan_json: serde_json::Value = serde_json::from_str(&fact.content)
                .unwrap_or_else(|_| serde_json::json!({"description": fact.content}));

            let findings = self.check_plan(&plan_json);
            let has_blockers = findings.iter().any(|f| f.severity == Severity::Blocker);
            let messages: Vec<String> = findings.iter().map(|f| f.message.clone()).collect();

            if has_blockers {
                proposals.push(ProposedFact::new(
                    ContextKey::Constraints,
                    format!("constraint-block-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "constraint-checker",
                        "kind": "constraint_checking",
                        "severity": "blocker",
                        "violations": messages,
                    })
                    .to_string(),
                    "constraint-checker",
                ));
            } else {
                proposals.push(ProposedFact::new(
                    ContextKey::Evaluations,
                    format!("constraint-pass-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "constraint-checker",
                        "kind": "constraint_checking",
                        "passed": true,
                        "findings": messages,
                    })
                    .to_string(),
                    "constraint-checker",
                ));
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Economic Skeptic ──────────────────────────────────────────────

/// Challenges cost and resource assumptions. Runs sensitivity analysis
/// on economic projections and flags unrealistic estimates.
pub struct EconomicSkepticAgent {
    skepticism_threshold: f64,
}

impl EconomicSkepticAgent {
    #[must_use]
    pub fn new(skepticism_threshold: f64) -> Self {
        Self {
            skepticism_threshold,
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            skepticism_threshold: 0.3,
        }
    }

    fn analyze_economics(plan: &serde_json::Value, threshold: f64) -> Vec<Finding> {
        let mut findings = Vec::new();

        let costs: Vec<(String, f64)> = plan
            .get("annotation")
            .and_then(|a| a.get("costs"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let cat = v
                            .get("category")
                            .and_then(|c| c.as_str())
                            .unwrap_or("unknown");
                        let est = v.get("estimate").and_then(serde_json::Value::as_f64)?;
                        Some((cat.to_string(), est))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let revenue: Option<f64> = plan
            .get("annotation")
            .and_then(|a| a.get("expected_revenue"))
            .and_then(serde_json::Value::as_f64);

        let roi: Option<f64> = plan
            .get("annotation")
            .and_then(|a| a.get("roi"))
            .and_then(serde_json::Value::as_f64);

        let total_cost: f64 = costs.iter().map(|(_, c)| c).sum();

        // Flag suspiciously round numbers (likely not based on real data)
        for (cat, cost) in &costs {
            if *cost > 1000.0 && *cost % 1000.0 == 0.0 {
                findings.push(Finding {
                    agent: "economic-skeptic".into(),
                    severity: Severity::Advisory,
                    message: format!(
                        "suspiciously round estimate for '{cat}': {cost:.0} — likely not data-driven",
                    ),
                });
            }
        }

        // Flag unrealistic ROI
        if let Some(r) = roi
            && r > 5.0
        {
            findings.push(Finding {
                agent: "economic-skeptic".into(),
                severity: Severity::Warning,
                message: format!("ROI of {r:.1}x is unusually high — requires strong evidence"),
            });
        }

        // Flag missing cost breakdown
        if costs.is_empty() && total_cost == 0.0 {
            findings.push(Finding {
                agent: "economic-skeptic".into(),
                severity: Severity::Warning,
                message: "no cost estimates provided — economic feasibility unknown".into(),
            });
        }

        // Revenue vs cost check
        if let Some(rev) = revenue
            && total_cost > 0.0
            && rev > 0.0
        {
            let margin = (rev - total_cost) / rev;
            if margin < threshold {
                findings.push(Finding {
                    agent: "economic-skeptic".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "thin margin: {:.1}% — below skepticism threshold of {:.1}%",
                        margin * 100.0,
                        threshold * 100.0,
                    ),
                });
            }
        }

        findings
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for EconomicSkepticAgent {
    fn name(&self) -> &'static str {
        "economic-skeptic"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();

        for fact in strategies {
            let plan_json: serde_json::Value = serde_json::from_str(&fact.content)
                .unwrap_or_else(|_| serde_json::json!({"description": fact.content}));

            let findings = Self::analyze_economics(&plan_json, self.skepticism_threshold);
            let has_blockers = findings.iter().any(|f| f.severity == Severity::Blocker);
            let messages: Vec<String> = findings.iter().map(|f| f.message.clone()).collect();

            if has_blockers {
                proposals.push(ProposedFact::new(
                    ContextKey::Constraints,
                    format!("econ-block-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "economic-skeptic",
                        "kind": "economic_skepticism",
                        "severity": "blocker",
                        "findings": messages,
                    })
                    .to_string(),
                    "economic-skeptic",
                ));
            } else {
                proposals.push(ProposedFact::new(
                    ContextKey::Evaluations,
                    format!("econ-eval-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "economic-skeptic",
                        "kind": "economic_skepticism",
                        "passed": true,
                        "findings": messages,
                    })
                    .to_string(),
                    "economic-skeptic",
                ));
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Operational Skeptic ───────────────────────────────────────────

/// Challenges feasibility given current team/system state. Checks for
/// resource conflicts, timeline pressure, and dependency bottlenecks.
pub struct OperationalSkepticAgent {
    max_parallel_initiatives: u32,
}

impl OperationalSkepticAgent {
    #[must_use]
    pub fn new(max_parallel_initiatives: u32) -> Self {
        Self {
            max_parallel_initiatives,
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            max_parallel_initiatives: 3,
        }
    }

    fn analyze_operations(plan: &serde_json::Value, max_parallel: u32) -> Vec<Finding> {
        let mut findings = Vec::new();

        let team_size: Option<u32> = plan
            .get("annotation")
            .and_then(|a| a.get("team_size"))
            .and_then(serde_json::Value::as_u64)
            .map(|n| u32::try_from(n).unwrap_or(0));

        let parallel_work: Option<u32> = plan
            .get("annotation")
            .and_then(|a| a.get("parallel_initiatives"))
            .and_then(serde_json::Value::as_u64)
            .map(|n| u32::try_from(n).unwrap_or(0));

        let timeline_days: Option<u32> = plan
            .get("annotation")
            .and_then(|a| a.get("timeline_days"))
            .and_then(serde_json::Value::as_u64)
            .map(|n| u32::try_from(n).unwrap_or(0));

        let complexity: Option<&str> = plan
            .get("annotation")
            .and_then(|a| a.get("complexity"))
            .and_then(serde_json::Value::as_str);

        // Check parallel work overload
        if let Some(parallel) = parallel_work
            && parallel > max_parallel
        {
            findings.push(Finding {
                agent: "operational-skeptic".into(),
                severity: Severity::Warning,
                message: format!("{parallel} parallel initiatives exceeds cap of {max_parallel}",),
            });
        }

        // Check team size vs complexity
        match (team_size, complexity) {
            (Some(size), Some("high")) if size < 3 => {
                findings.push(Finding {
                    agent: "operational-skeptic".into(),
                    severity: Severity::Warning,
                    message: format!("high complexity with team of {size} — likely understaffed",),
                });
            }
            (Some(size), Some("critical")) if size < 5 => {
                findings.push(Finding {
                    agent: "operational-skeptic".into(),
                    severity: Severity::Blocker,
                    message: format!(
                        "critical complexity with team of {size} — insufficient capacity",
                    ),
                });
            }
            _ => {}
        }

        // Check timeline realism
        if let (Some(days), Some(comp)) = (timeline_days, complexity) {
            let too_fast = match comp {
                "critical" => days < 90,
                "high" => days < 30,
                "medium" => days < 14,
                _ => false,
            };
            if too_fast {
                findings.push(Finding {
                    agent: "operational-skeptic".into(),
                    severity: Severity::Warning,
                    message: format!(
                        "timeline of {days} days is aggressive for {comp} complexity",
                    ),
                });
            }
        }

        // Check for single points of failure
        let key_people: Vec<String> = plan
            .get("annotation")
            .and_then(|a| a.get("key_people"))
            .and_then(|k| k.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if key_people.len() == 1 {
            findings.push(Finding {
                agent: "operational-skeptic".into(),
                severity: Severity::Advisory,
                message: format!(
                    "single point of failure: only '{}' identified as key person",
                    key_people[0],
                ),
            });
        }

        findings
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for OperationalSkepticAgent {
    fn name(&self) -> &'static str {
        "operational-skeptic"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();

        for fact in strategies {
            let plan_json: serde_json::Value = serde_json::from_str(&fact.content)
                .unwrap_or_else(|_| serde_json::json!({"description": fact.content}));

            let findings = Self::analyze_operations(&plan_json, self.max_parallel_initiatives);
            let has_blockers = findings.iter().any(|f| f.severity == Severity::Blocker);
            let messages: Vec<String> = findings.iter().map(|f| f.message.clone()).collect();

            if has_blockers {
                proposals.push(ProposedFact::new(
                    ContextKey::Constraints,
                    format!("ops-skeptic-block-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "operational-skeptic",
                        "kind": "operational_skepticism",
                        "severity": "blocker",
                        "findings": messages,
                    })
                    .to_string(),
                    "operational-skeptic",
                ));
            } else {
                proposals.push(ProposedFact::new(
                    ContextKey::Evaluations,
                    format!("ops-skeptic-eval-{}", fact.id),
                    serde_json::json!({
                        "strategy_id": fact.id,
                        "agent": "operational-skeptic",
                        "kind": "operational_skepticism",
                        "passed": true,
                        "findings": messages,
                    })
                    .to_string(),
                    "operational-skeptic",
                ));
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Assumption Breaker Tests ──────────────────────────────────

    #[test]
    fn no_assumptions_warns() {
        let plan = json!({"annotation": {}});
        let findings = AssumptionBreakerAgent::analyze_assumptions(&plan);
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("no assumptions"))
        );
    }

    #[test]
    fn unsupported_assumption_warned() {
        let plan = json!({
            "annotation": {
                "assumptions": ["stable market"],
                "evidence": ["customer survey Q1"]
            }
        });
        let findings = AssumptionBreakerAgent::analyze_assumptions(&plan);
        assert!(findings.iter().any(|f| f.message.contains("unsupported")));
    }

    #[test]
    fn supported_assumption_ok() {
        let plan = json!({
            "annotation": {
                "assumptions": ["customer demand"],
                "evidence": ["customer demand survey shows 80% interest"]
            }
        });
        let findings = AssumptionBreakerAgent::analyze_assumptions(&plan);
        assert!(!findings.iter().any(|f| f.message.contains("unsupported")));
    }

    #[test]
    fn implicit_assumptions_detected() {
        let plan = json!({
            "annotation": {
                "assumptions": ["ok"],
                "evidence": ["ok"]
            },
            "description": "This always works and never fails"
        });
        let findings = AssumptionBreakerAgent::analyze_assumptions(&plan);
        assert!(findings.iter().any(|f| f.message.contains("implicit")));
    }

    // ── Constraint Checker Tests ──────────────────────────────────

    #[test]
    fn budget_constraint_blocks() {
        let agent = ConstraintCheckerAgent::new(vec![OrgConstraint {
            name: "dept-budget".into(),
            check: ConstraintCheck::MaxBudget(50_000.0),
        }]);
        let plan = json!({
            "annotation": {
                "costs": [{"category": "compute", "estimate": 60_000.0}]
            }
        });
        let findings = agent.check_plan(&plan);
        assert!(findings.iter().any(|f| f.severity == Severity::Blocker));
    }

    #[test]
    fn within_budget_passes() {
        let agent = ConstraintCheckerAgent::new(vec![OrgConstraint {
            name: "dept-budget".into(),
            check: ConstraintCheck::MaxBudget(100_000.0),
        }]);
        let plan = json!({
            "annotation": {
                "costs": [{"category": "compute", "estimate": 50_000.0}]
            }
        });
        let findings = agent.check_plan(&plan);
        assert!(!findings.iter().any(|f| f.severity == Severity::Blocker));
    }

    #[test]
    fn forbidden_action_blocks() {
        let agent = ConstraintCheckerAgent::new(vec![OrgConstraint {
            name: "no-delete-prod".into(),
            check: ConstraintCheck::ForbiddenAction("delete-production".into()),
        }]);
        let plan = json!({
            "annotation": {
                "actions": ["delete-production-database"]
            }
        });
        let findings = agent.check_plan(&plan);
        assert!(findings.iter().any(|f| f.severity == Severity::Blocker));
    }

    #[test]
    fn required_approval_blocks_when_missing() {
        let agent = ConstraintCheckerAgent::new(vec![OrgConstraint {
            name: "cfo-approval".into(),
            check: ConstraintCheck::RequiredApproval("cfo".into()),
        }]);
        let plan = json!({"annotation": {"approvals": ["eng-lead"]}});
        let findings = agent.check_plan(&plan);
        assert!(findings.iter().any(|f| f.severity == Severity::Blocker));
    }

    // ── Economic Skeptic Tests ────────────────────────────────────

    #[test]
    fn round_numbers_flagged() {
        let findings = EconomicSkepticAgent::analyze_economics(
            &json!({
                "annotation": {
                    "costs": [{"category": "compute", "estimate": 50_000.0}]
                }
            }),
            0.3,
        );
        assert!(findings.iter().any(|f| f.message.contains("round")));
    }

    #[test]
    fn high_roi_flagged() {
        let findings =
            EconomicSkepticAgent::analyze_economics(&json!({"annotation": {"roi": 10.0}}), 0.3);
        assert!(findings.iter().any(|f| f.message.contains("ROI")));
    }

    #[test]
    fn thin_margin_flagged() {
        let findings = EconomicSkepticAgent::analyze_economics(
            &json!({
                "annotation": {
                    "costs": [{"category": "total", "estimate": 90.0}],
                    "expected_revenue": 100.0
                }
            }),
            0.3,
        );
        assert!(findings.iter().any(|f| f.message.contains("margin")));
    }

    // ── Operational Skeptic Tests ─────────────────────────────────

    #[test]
    fn parallel_overload_warned() {
        let findings = OperationalSkepticAgent::analyze_operations(
            &json!({"annotation": {"parallel_initiatives": 5}}),
            3,
        );
        assert!(findings.iter().any(|f| f.message.contains("parallel")));
    }

    #[test]
    fn understaffed_critical_blocks() {
        let findings = OperationalSkepticAgent::analyze_operations(
            &json!({"annotation": {"team_size": 2, "complexity": "critical"}}),
            3,
        );
        assert!(findings.iter().any(|f| f.severity == Severity::Blocker));
    }

    #[test]
    fn aggressive_timeline_warned() {
        let findings = OperationalSkepticAgent::analyze_operations(
            &json!({"annotation": {"timeline_days": 7, "complexity": "high"}}),
            3,
        );
        assert!(findings.iter().any(|f| f.message.contains("aggressive")));
    }

    #[test]
    fn single_key_person_noted() {
        let findings = OperationalSkepticAgent::analyze_operations(
            &json!({"annotation": {"key_people": ["alice"]}}),
            3,
        );
        assert!(findings.iter().any(|f| f.message.contains("single point")));
    }

    #[test]
    fn adequate_team_passes() {
        let findings = OperationalSkepticAgent::analyze_operations(
            &json!({"annotation": {"team_size": 8, "complexity": "high", "timeline_days": 60}}),
            3,
        );
        assert!(!findings.iter().any(|f| f.severity == Severity::Blocker));
    }
}
