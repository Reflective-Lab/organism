//! Operational simulator.
//!
//! Evaluates candidate plans for operational feasibility: team capacity,
//! system load, timeline realism, and dependency availability.

use crate::{DimensionResult, Sample, SimulationDimension};

#[derive(Debug, Clone)]
pub struct OperationalSimulatorConfig {
    pub max_team_utilization: f64,
    pub max_system_load: f64,
    pub confidence_threshold: f64,
}

impl Default for OperationalSimulatorConfig {
    fn default() -> Self {
        Self {
            max_team_utilization: 0.85,
            max_system_load: 0.80,
            confidence_threshold: 0.5,
        }
    }
}

pub struct OperationalSimulator {
    config: OperationalSimulatorConfig,
}

impl OperationalSimulator {
    #[must_use]
    pub fn new(config: OperationalSimulatorConfig) -> Self {
        Self { config }
    }

    fn extract_team_utilization(plan: &serde_json::Value) -> Option<f64> {
        plan.get("annotation")
            .and_then(|a| a.get("team_utilization"))
            .and_then(serde_json::Value::as_f64)
    }

    fn extract_system_load(plan: &serde_json::Value) -> Option<f64> {
        plan.get("annotation")
            .and_then(|a| a.get("system_load"))
            .and_then(serde_json::Value::as_f64)
    }

    fn extract_dependencies(plan: &serde_json::Value) -> Vec<Dependency> {
        plan.get("annotation")
            .and_then(|a| a.get("dependencies"))
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(Dependency {
                            name: v.get("name").and_then(|n| n.as_str())?.to_string(),
                            available: v
                                .get("available")
                                .and_then(serde_json::Value::as_bool)
                                .unwrap_or(true),
                            lead_time_days: v
                                .get("lead_time_days")
                                .and_then(serde_json::Value::as_u64)
                                .map(|n| u32::try_from(n).unwrap_or(0)),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_timeline_days(plan: &serde_json::Value) -> Option<u32> {
        plan.get("annotation")
            .and_then(|a| a.get("timeline_days"))
            .and_then(serde_json::Value::as_u64)
            .map(|n| u32::try_from(n).unwrap_or(0))
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
        let team_util = Self::extract_team_utilization(plan);
        let system_load = Self::extract_system_load(plan);
        let deps = Self::extract_dependencies(plan);
        let timeline = Self::extract_timeline_days(plan);

        let mut findings = Vec::new();
        let mut penalties = 0.0_f64;

        // Team utilization check
        match team_util {
            Some(util) if util > self.config.max_team_utilization => {
                findings.push(format!(
                    "team overloaded: {:.0}% utilization exceeds {:.0}% cap",
                    util * 100.0,
                    self.config.max_team_utilization * 100.0,
                ));
                penalties += (util - self.config.max_team_utilization) * 2.0;
            }
            Some(util) => {
                findings.push(format!(
                    "team utilization {:.0}% within capacity",
                    util * 100.0
                ));
            }
            None => {
                findings.push("no team utilization declared".into());
            }
        }

        // System load check
        match system_load {
            Some(load) if load > self.config.max_system_load => {
                findings.push(format!(
                    "system overloaded: {:.0}% load exceeds {:.0}% cap",
                    load * 100.0,
                    self.config.max_system_load * 100.0,
                ));
                penalties += (load - self.config.max_system_load) * 2.0;
            }
            Some(load) => {
                findings.push(format!("system load {:.0}% within capacity", load * 100.0));
            }
            None => {
                findings.push("no system load declared".into());
            }
        }

        // Dependency availability
        let unavailable: Vec<&Dependency> = deps.iter().filter(|d| !d.available).collect();
        if !unavailable.is_empty() {
            for dep in &unavailable {
                findings.push(format!("dependency unavailable: {}", dep.name));
            }
            penalties += 0.3 * f64::from(u32::try_from(unavailable.len()).unwrap_or(1));
        }

        // Timeline vs dependency lead times
        if let Some(days) = timeline {
            let max_lead = deps
                .iter()
                .filter_map(|d| d.lead_time_days)
                .max()
                .unwrap_or(0);
            if max_lead > days {
                findings.push(format!(
                    "timeline {days} days but dependency needs {max_lead} days lead time",
                ));
                penalties += 0.2;
            }
        }

        let confidence = (1.0 - penalties).clamp(0.0, 1.0);
        let passed = confidence >= self.config.confidence_threshold;
        let samples = Self::sample(confidence);

        if !passed {
            findings.push(format!(
                "below threshold: {confidence:.2} < {:.2}",
                self.config.confidence_threshold,
            ));
        }

        DimensionResult {
            dimension: SimulationDimension::Operational,
            passed,
            confidence,
            findings,
            samples,
        }
    }
}

struct Dependency {
    name: String,
    available: bool,
    lead_time_days: Option<u32>,
}

// ── Suggestor Implementation ──────────────────────────────────────

use crate::types::{SimulationRecommendation, SimulationVerdict};
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

pub struct OperationalSimulationAgent {
    simulator: OperationalSimulator,
}

impl OperationalSimulationAgent {
    #[must_use]
    pub fn new(config: OperationalSimulatorConfig) -> Self {
        Self {
            simulator: OperationalSimulator::new(config),
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            simulator: OperationalSimulator::new(OperationalSimulatorConfig::default()),
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for OperationalSimulationAgent {
    fn name(&self) -> &'static str {
        "operational-simulation"
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
                dimension: crate::SimulationDimension::Operational,
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
                "operational-simulation",
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_simulator() -> OperationalSimulator {
        OperationalSimulator::new(OperationalSimulatorConfig::default())
    }

    #[test]
    fn within_capacity_passes() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "team_utilization": 0.7,
                "system_load": 0.6
            }
        });
        let result = sim.simulate(&plan);
        assert_eq!(result.dimension, SimulationDimension::Operational);
        assert!(result.passed);
    }

    #[test]
    fn team_overload_penalized() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "team_utilization": 0.95,
                "system_load": 0.5
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.findings.iter().any(|f| f.contains("overloaded")));
        assert!(result.confidence < 1.0);
    }

    #[test]
    fn system_overload_penalized() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "team_utilization": 0.5,
                "system_load": 0.95
            }
        });
        let result = sim.simulate(&plan);
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.contains("system overloaded"))
        );
    }

    #[test]
    fn unavailable_dependency_penalized() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "team_utilization": 0.5,
                "system_load": 0.5,
                "dependencies": [
                    {"name": "payment-api", "available": false},
                    {"name": "auth-service", "available": true}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.findings.iter().any(|f| f.contains("unavailable")));
    }

    #[test]
    fn timeline_vs_lead_time() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "team_utilization": 0.5,
                "system_load": 0.5,
                "timeline_days": 14,
                "dependencies": [
                    {"name": "vendor-api", "available": true, "lead_time_days": 30}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.findings.iter().any(|f| f.contains("lead time")));
    }

    #[test]
    fn no_annotations_passes() {
        let sim = default_simulator();
        let plan = json!({});
        let result = sim.simulate(&plan);
        assert!(result.passed);
    }

    #[test]
    fn extreme_overload_fails() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "team_utilization": 1.0,
                "system_load": 1.0,
                "dependencies": [
                    {"name": "a", "available": false},
                    {"name": "b", "available": false}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert!(!result.passed);
    }
}
