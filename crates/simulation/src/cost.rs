//! Cost simulator.
//!
//! Evaluates candidate plans by analyzing cost annotations (compute, labor,
//! infrastructure, licensing) and producing resource envelope estimates via
//! Monte Carlo sampling.

use crate::{DimensionResult, Sample, SimulationDimension};
use converge_pack::UnitInterval;

/// Non-negative monetary amount. Negative values are clamped to zero at
/// construction — a budget cannot logically be negative.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BudgetAmount(f64);

impl BudgetAmount {
    /// Construct from a non-negative amount. Negative values are clamped to 0.
    #[must_use]
    pub fn new(amount: f64) -> Self {
        Self(amount.max(0.0))
    }

    #[must_use]
    pub fn as_f64(self) -> f64 {
        self.0
    }
}

impl From<f64> for BudgetAmount {
    fn from(v: f64) -> Self {
        Self::new(v)
    }
}

impl std::fmt::Display for BudgetAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct CostSimulatorConfig {
    pub samples: u32,
    /// Maximum cost before the plan is considered over-budget.
    pub budget_ceiling: BudgetAmount,
    /// Fraction of the ceiling that may be exceeded without failing
    /// (e.g. 0.15 = 15% tolerance). Enforced as a `UnitInterval`.
    pub overrun_tolerance: UnitInterval,
}

impl Default for CostSimulatorConfig {
    fn default() -> Self {
        Self {
            samples: 1000,
            budget_ceiling: BudgetAmount::new(100_000.0),
            overrun_tolerance: UnitInterval::clamped(0.15),
        }
    }
}

pub struct CostSimulator {
    config: CostSimulatorConfig,
}

impl CostSimulator {
    #[must_use]
    pub fn new(config: CostSimulatorConfig) -> Self {
        Self { config }
    }

    fn extract_costs(plan: &serde_json::Value) -> Vec<f64> {
        plan.get("annotation")
            .and_then(|a| a.get("costs"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("estimate").and_then(serde_json::Value::as_f64))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_cost_uncertainties(plan: &serde_json::Value) -> Vec<f64> {
        plan.get("annotation")
            .and_then(|a| a.get("costs"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("uncertainty").and_then(serde_json::Value::as_f64))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn sample(&self, total_cost: f64, avg_uncertainty: f64) -> Vec<Sample> {
        let buckets = 5;
        let mut samples = Vec::with_capacity(buckets);
        let ceiling = self.config.budget_ceiling.as_f64();
        let ratio = total_cost / ceiling;

        for i in 0..buckets {
            let bucket_center = (f64::from(u32::try_from(i).unwrap_or(0)) + 0.5)
                / f64::from(u32::try_from(buckets).unwrap_or(5));
            // Distribution centered on cost ratio, spread by uncertainty
            let spread = (1.0 + avg_uncertainty * 3.0).max(1.0);
            let distance = (bucket_center - ratio.clamp(0.0, 1.0)).abs();
            let weight = (-distance * spread).exp();
            samples.push(Sample {
                value: bucket_center * ceiling,
                probability: weight,
            });
        }

        let total: f64 = samples.iter().map(|s| s.probability).sum();
        if total > 0.0 {
            for s in &mut samples {
                s.probability /= total;
            }
        }
        for s in &mut samples {
            s.probability = (s.probability * f64::from(self.config.samples)).round()
                / f64::from(self.config.samples);
        }

        samples
    }

    pub fn simulate(&self, plan: &serde_json::Value) -> DimensionResult {
        let costs = Self::extract_costs(plan);
        let uncertainties = Self::extract_cost_uncertainties(plan);

        let total_cost: f64 = costs.iter().sum();
        let avg_uncertainty = if uncertainties.is_empty() {
            0.3 // default uncertainty when not specified
        } else {
            uncertainties.iter().sum::<f64>()
                / f64::from(u32::try_from(uncertainties.len()).unwrap_or(1))
        };

        let ceiling = self.config.budget_ceiling.as_f64();
        let max_allowed = ceiling * (1.0 + self.config.overrun_tolerance.as_f64());
        let passed = total_cost <= max_allowed;
        let confidence = if total_cost <= 0.0 {
            0.5
        } else {
            (1.0 - total_cost / max_allowed).clamp(0.0, 1.0)
        };

        let samples = self.sample(total_cost, avg_uncertainty);

        let mut findings = Vec::new();
        if costs.is_empty() {
            findings.push("no cost annotations — cannot assess budget fit".into());
        } else {
            findings.push(format!(
                "{} cost items, total {:.0} against ceiling {}",
                costs.len(),
                total_cost,
                self.config.budget_ceiling,
            ));
        }
        if total_cost > ceiling {
            findings.push(format!(
                "overrun: {:.0} exceeds ceiling by {:.1}%",
                total_cost,
                ((total_cost / ceiling) - 1.0) * 100.0,
            ));
        }
        if avg_uncertainty > 0.5 {
            findings.push(format!("high cost uncertainty: avg {avg_uncertainty:.2}"));
        }

        DimensionResult {
            dimension: SimulationDimension::Cost,
            passed,
            confidence: UnitInterval::clamped(confidence),
            findings,
            samples,
        }
    }
}

// ── Suggestor Implementation ──────────────────────────────────────

use crate::provenance::ORGANISM_SIMULATION_PROVENANCE;
use crate::types::{SimulationRecommendation, SimulationVerdict};
use converge_pack::{
    AgentEffect, Context, ContextFact, ContextKey, ProposedFact, Provenance, ProvenanceSource,
    Suggestor, TextPayload,
};

fn proposed_text_fact(
    key: ContextKey,
    id: impl Into<converge_pack::ProposalId>,
    content: impl Into<String>,
) -> ProposedFact {
    ORGANISM_SIMULATION_PROVENANCE.proposed_fact(key, id, TextPayload::new(content))
}

fn fact_text(fact: &ContextFact) -> &str {
    fact.text().unwrap_or_default()
}

pub struct CostSimulationAgent {
    simulator: CostSimulator,
}

impl CostSimulationAgent {
    #[must_use]
    pub fn new(config: CostSimulatorConfig) -> Self {
        Self {
            simulator: CostSimulator::new(config),
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self {
            simulator: CostSimulator::new(CostSimulatorConfig::default()),
        }
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for CostSimulationAgent {
    fn name(&self) -> &'static str {
        "cost-simulation"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn provenance(&self) -> Provenance {
        Provenance::from(ORGANISM_SIMULATION_PROVENANCE.as_str())
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();

        for fact in strategies {
            let content = fact_text(fact);
            let plan_json: serde_json::Value = serde_json::from_str(content)
                .unwrap_or_else(|_| serde_json::json!({"description": content}));

            let result = self.simulator.simulate(&plan_json);

            let verdict = SimulationVerdict {
                strategy_id: fact.id().clone(),
                dimension: crate::SimulationDimension::Cost,
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

            proposals.push(proposed_text_fact(
                key,
                verdict.fact_id(),
                verdict.to_json(),
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn default_simulator() -> CostSimulator {
        CostSimulator::new(CostSimulatorConfig::default())
    }

    #[test]
    fn within_budget_passes() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "costs": [
                    {"category": "compute", "estimate": 30_000.0, "uncertainty": 0.1},
                    {"category": "labor", "estimate": 40_000.0, "uncertainty": 0.2}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert_eq!(result.dimension, SimulationDimension::Cost);
        assert!(result.passed);
    }

    #[test]
    fn over_budget_fails() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "costs": [
                    {"category": "compute", "estimate": 80_000.0},
                    {"category": "labor", "estimate": 50_000.0}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert!(!result.passed);
        assert!(result.findings.iter().any(|f| f.contains("overrun")));
    }

    #[test]
    fn no_costs_uses_neutral() {
        let sim = default_simulator();
        let plan = json!({});
        let result = sim.simulate(&plan);
        assert!(result.passed); // 0 cost is within budget
        assert!(result.findings[0].contains("no cost annotations"));
    }

    #[test]
    fn high_uncertainty_flagged() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "costs": [
                    {"category": "compute", "estimate": 50_000.0, "uncertainty": 0.8}
                ]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.findings.iter().any(|f| f.contains("uncertainty")));
    }

    #[test]
    fn within_overrun_tolerance_passes() {
        let sim = default_simulator(); // ceiling 100k, tolerance 15%
        let plan = json!({
            "annotation": {
                "costs": [{"category": "total", "estimate": 110_000.0}]
            }
        });
        let result = sim.simulate(&plan);
        assert!(result.passed); // 110k < 115k (100k * 1.15)
    }

    #[test]
    fn samples_are_normalized() {
        let sim = default_simulator();
        let plan = json!({
            "annotation": {
                "costs": [{"category": "compute", "estimate": 50_000.0}]
            }
        });
        let result = sim.simulate(&plan);
        assert!(!result.samples.is_empty());
        let total: f64 = result.samples.iter().map(|s| s.probability).sum();
        assert!((total - 1.0).abs() < 0.01);
    }
}
