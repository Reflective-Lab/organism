//! Policy simulator.
//!
//! Evaluates candidate plans against organizational policy constraints:
//! authority levels, compliance requirements, data classification rules,
//! and approval gates.

use crate::{DimensionResult, Sample, SimulationDimension};

#[derive(Debug, Clone)]
pub struct PolicySimulatorConfig {
    pub required_authority_level: u32,
    pub require_compliance_tags: Vec<String>,
    pub block_on_missing_authority: bool,
}

impl Default for PolicySimulatorConfig {
    fn default() -> Self {
        Self {
            required_authority_level: 1,
            require_compliance_tags: Vec::new(),
            block_on_missing_authority: true,
        }
    }
}

pub struct PolicySimulator {
    config: PolicySimulatorConfig,
}

impl PolicySimulator {
    #[must_use]
    pub fn new(config: PolicySimulatorConfig) -> Self {
        Self { config }
    }

    fn extract_authority_level(plan: &serde_json::Value) -> Option<u32> {
        plan.get("annotation")
            .and_then(|a| a.get("authority_level"))
            .and_then(serde_json::Value::as_u64)
            .map(|v| u32::try_from(v).unwrap_or(0))
    }

    fn extract_compliance_tags(plan: &serde_json::Value) -> Vec<String> {
        plan.get("annotation")
            .and_then(|a| a.get("compliance_tags"))
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_approval_gates(plan: &serde_json::Value) -> Vec<String> {
        plan.get("annotation")
            .and_then(|a| a.get("approval_gates"))
            .and_then(|g| g.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn sample(confidence: f64) -> Vec<Sample> {
        let buckets = 5;
        let mut samples = Vec::with_capacity(buckets);

        for i in 0..buckets {
            let bucket_center = (f64::from(u32::try_from(i).unwrap_or(0)) + 0.5)
                / f64::from(u32::try_from(buckets).unwrap_or(5));
            let distance = (bucket_center - confidence).abs();
            let weight = (-distance * 4.0).exp();
            samples.push(Sample {
                value: bucket_center,
                probability: weight,
            });
        }

        let total: f64 = samples.iter().map(|s| s.probability).sum();
        if total > 0.0 {
            for s in &mut samples {
                s.probability /= total;
            }
        }

        samples
    }

    pub fn simulate(&self, plan: &serde_json::Value) -> DimensionResult {
        let authority = Self::extract_authority_level(plan);
        let tags = Self::extract_compliance_tags(plan);
        let gates = Self::extract_approval_gates(plan);

        let mut findings = Vec::new();
        let mut violations = 0u32;

        // Authority check
        match authority {
            Some(level) if level >= self.config.required_authority_level => {
                findings.push(format!("authority level {level} meets requirement"));
            }
            Some(level) => {
                findings.push(format!(
                    "authority level {level} below required {}",
                    self.config.required_authority_level,
                ));
                violations += 1;
            }
            None if self.config.block_on_missing_authority => {
                findings.push("no authority level declared — blocked by policy".into());
                violations += 1;
            }
            None => {
                findings.push("no authority level declared — not required".into());
            }
        }

        // Compliance tag check
        for required in &self.config.require_compliance_tags {
            if tags.iter().any(|t| t == required) {
                findings.push(format!("compliance: {required} satisfied"));
            } else {
                findings.push(format!("compliance: {required} missing"));
                violations += 1;
            }
        }

        // Note approval gates (informational)
        if !gates.is_empty() {
            findings.push(format!(
                "{} approval gate(s) required: {}",
                gates.len(),
                gates.join(", ")
            ));
        }

        let passed = violations == 0;
        let total_checks =
            1 + u32::try_from(self.config.require_compliance_tags.len()).unwrap_or(0);
        let confidence = if total_checks == 0 {
            1.0
        } else {
            f64::from(total_checks - violations.min(total_checks)) / f64::from(total_checks)
        };

        let samples = Self::sample(confidence);

        DimensionResult {
            dimension: SimulationDimension::Policy,
            passed,
            confidence,
            findings,
            samples,
        }
    }
}

// ── Suggestor Implementation ──────────────────────────────────────

use crate::types::{SimulationRecommendation, SimulationVerdict};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

pub struct PolicySimulationAgent {
    simulator: PolicySimulator,
}

impl PolicySimulationAgent {
    #[must_use]
    pub fn new(config: PolicySimulatorConfig) -> Self {
        Self {
            simulator: PolicySimulator::new(config),
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            simulator: PolicySimulator::new(PolicySimulatorConfig::default()),
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for PolicySimulationAgent {
    fn name(&self) -> &'static str {
        "policy-simulation"
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

            let result = self.simulator.simulate(&plan_json);

            let verdict = SimulationVerdict {
                strategy_id: fact.id.clone(),
                dimension: crate::SimulationDimension::Policy,
                passed: result.passed,
                confidence: result.confidence,
                findings: result.findings,
                recommendation: if result.passed {
                    None
                } else {
                    Some(SimulationRecommendation::DoNotProceed)
                },
            };

            let key = if result.passed {
                ContextKey::Evaluations
            } else {
                ContextKey::Constraints
            };

            proposals.push(ProposedFact::new(
                key,
                verdict.fact_id(),
                verdict.to_json(),
                "policy-simulation",
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_simulator() -> PolicySimulator {
        PolicySimulator::new(PolicySimulatorConfig::default())
    }

    #[test]
    fn sufficient_authority_passes() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "authority_level": 2
            }
        });
        let result = sim.simulate(&plan);
        assert_eq!(result.dimension, SimulationDimension::Policy);
        assert!(result.passed);
    }

    #[test]
    fn insufficient_authority_fails() {
        let sim = PolicySimulator::new(PolicySimulatorConfig {
            required_authority_level: 3,
            ..PolicySimulatorConfig::default()
        });
        let plan = json!({
            "annotation": {
                "authority_level": 1
            }
        });
        let result = sim.simulate(&plan);
        assert!(!result.passed);
        assert!(result.findings.iter().any(|f| f.contains("below required")));
    }

    #[test]
    fn missing_authority_blocks_by_default() {
        let sim = default_simulator();
        let plan = json!({});
        let result = sim.simulate(&plan);
        assert!(!result.passed);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.contains("blocked by policy"))
        );
    }

    #[test]
    fn missing_authority_allowed_when_configured() {
        let sim = PolicySimulator::new(PolicySimulatorConfig {
            block_on_missing_authority: false,
            ..PolicySimulatorConfig::default()
        });
        let plan = json!({});
        let result = sim.simulate(&plan);
        assert!(result.passed);
    }

    #[test]
    fn compliance_tags_checked() {
        let sim = PolicySimulator::new(PolicySimulatorConfig {
            require_compliance_tags: vec!["gdpr".into(), "soc2".into()],
            ..PolicySimulatorConfig::default()
        });
        let plan = json!({
            "annotation": {
                "authority_level": 1,
                "compliance_tags": ["gdpr"]
            }
        });
        let result = sim.simulate(&plan);
        assert!(!result.passed); // missing soc2
        assert!(result.findings.iter().any(|f| f.contains("soc2 missing")));
    }

    #[test]
    fn all_compliance_satisfied() {
        let sim = PolicySimulator::new(PolicySimulatorConfig {
            require_compliance_tags: vec!["gdpr".into()],
            ..PolicySimulatorConfig::default()
        });
        let plan = json!({
            "annotation": {
                "authority_level": 1,
                "compliance_tags": ["gdpr", "hipaa"]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.passed);
    }

    #[test]
    fn approval_gates_noted() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "authority_level": 1,
                "approval_gates": ["legal-review", "cfo-sign-off"]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.passed);
        assert!(result.findings.iter().any(|f| f.contains("approval gate")));
    }
}
