//! Statistical-anomaly skeptic — flags plans whose key metrics are outliers
//! in the strategy-set distribution.
//!
//! Wraps `prism::packs::anomaly_detection::ZScoreSolver`. For each candidate
//! plan in `ContextKey::Strategies`, extracts a numeric metric from the
//! plan's annotation (default: total cost) and computes a z-score across all
//! plans. Plans with `|z| > threshold` get a `Constraint` flagging them as
//! anomalous; the rest pass through as `Evaluations`.
//!
//! Unlike the existing skeptics which inspect a plan in isolation, this
//! agent only fires when there are at least three strategies — anomaly
//! detection is meaningless against a sample of one.

use converge_pack::gate::{ObjectiveSpec, ProblemSpec};
use converge_pack::{
    AgentEffect, Context, ContextFact, ContextKey, ProposedFact, ProvenanceSource, Suggestor,
    TextPayload,
};
use prism::ZScoreThreshold;
use prism::packs::anomaly_detection::{AnomalyDetectionInput, ZScoreSolver};

use crate::provenance::ORGANISM_ADVERSARIAL_PROVENANCE;
use crate::{Finding, Severity};

fn proposed_text_fact(
    key: ContextKey,
    id: impl Into<converge_pack::ProposalId>,
    content: impl Into<String>,
) -> ProposedFact {
    ORGANISM_ADVERSARIAL_PROVENANCE.proposed_fact(key, id, TextPayload::new(content))
}

fn fact_text(fact: &ContextFact) -> &str {
    fact.text().unwrap_or_default()
}

/// Z-score skeptic over the active strategy set.
pub struct AnomalySkepticAgent {
    /// |z| above which a plan is flagged.
    threshold: ZScoreThreshold,
    /// JSON path inside the plan to the numeric metric to test. Defaults
    /// to `annotation.total_cost`. Falls back to summed `annotation.costs[*].estimate`
    /// if the direct path is absent.
    metric_field: String,
    /// Minimum strategy count below which the skeptic abstains (anomaly
    /// detection on N<3 is meaningless).
    min_strategies: usize,
}

impl AnomalySkepticAgent {
    #[must_use]
    pub fn new(threshold: ZScoreThreshold) -> Self {
        Self {
            threshold,
            metric_field: "total_cost".into(),
            min_strategies: 3,
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self::new(ZScoreThreshold::new(2.0).expect("2.0 is a valid ZScoreThreshold"))
    }

    #[must_use]
    pub fn with_metric_field(mut self, field: impl Into<String>) -> Self {
        self.metric_field = field.into();
        self
    }

    #[must_use]
    pub fn with_min_strategies(mut self, min: usize) -> Self {
        self.min_strategies = min;
        self
    }

    fn extract_metric(&self, plan: &serde_json::Value) -> Option<f64> {
        // Direct field on annotation.
        if let Some(value) = plan
            .get("annotation")
            .and_then(|a| a.get(&self.metric_field))
            .and_then(serde_json::Value::as_f64)
        {
            return Some(value);
        }
        // Fallback: sum costs[*].estimate.
        plan.get("annotation")
            .and_then(|a| a.get("costs"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("estimate").and_then(serde_json::Value::as_f64))
                    .sum::<f64>()
            })
            .filter(|v| v.is_finite() && *v != 0.0)
    }
}

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound, clippy::too_many_lines)]
impl Suggestor for AnomalySkepticAgent {
    fn name(&self) -> &'static str {
        "anomaly-skeptic"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_ADVERSARIAL_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut effect = AgentEffect::builder();

        // Parse plans + extract metrics. Index alignment is preserved.
        let parsed: Vec<(String, serde_json::Value, Option<f64>)> = strategies
            .iter()
            .map(|fact| {
                let content = fact_text(fact);
                let plan: serde_json::Value = serde_json::from_str(content)
                    .unwrap_or_else(|_| serde_json::json!({"description": content}));
                let metric = self.extract_metric(&plan);
                (fact.id().to_string(), plan, metric)
            })
            .collect();

        // Abstain when the sample is too small or no metrics are extractable.
        let metrics: Vec<f64> = parsed.iter().filter_map(|(_, _, m)| *m).collect();
        if metrics.len() < self.min_strategies {
            for (id, _, _) in &parsed {
                effect.push(proposed_text_fact(
                    ContextKey::Evaluations,
                    format!("anomaly-skip-{id}"),
                    serde_json::json!({
                        "strategy_id": id,
                        "agent": "anomaly-skeptic",
                        "kind": "statistical_anomaly",
                        "passed": true,
                        "note": format!(
                            "abstained: only {} usable metric{} (need >= {})",
                            metrics.len(),
                            if metrics.len() == 1 { "" } else { "s" },
                            self.min_strategies
                        ),
                    })
                    .to_string(),
                ));
            }
            return effect.build();
        }

        // Run Z-score across the metrics that exist.
        let input = AnomalyDetectionInput {
            values: metrics.clone(),
            threshold: self.threshold,
            labels: None,
        };
        let Ok(spec) = ProblemSpec::builder("anomaly-skeptic", "skeptic")
            .objective(ObjectiveSpec::maximize("anomaly_score"))
            .build()
        else {
            return effect.build();
        };

        let Ok((output, _report)) = ZScoreSolver.solve(&input, &spec) else {
            return effect.build();
        };

        // Map metric-index back to strategy-id. The metric vector skips plans
        // whose metric was None, so we walk parsed and consume from output as
        // we go.
        let mut metric_idx = 0usize;
        let threshold_value = self.threshold.value();
        for (id, _plan, metric) in &parsed {
            if metric.is_none() {
                effect.push(proposed_text_fact(
                    ContextKey::Evaluations,
                    format!("anomaly-skip-{id}"),
                    serde_json::json!({
                        "strategy_id": id,
                        "agent": "anomaly-skeptic",
                        "kind": "statistical_anomaly",
                        "passed": true,
                        "note": format!(
                            "no '{}' metric extractable from plan",
                            self.metric_field
                        ),
                    })
                    .to_string(),
                ));
                continue;
            }

            let z = output
                .anomalies
                .iter()
                .find(|a| a.index == metric_idx)
                .map(|a| a.z_score);

            metric_idx += 1;

            let finding = match z {
                Some(z_score) => {
                    let f = Finding {
                        agent: "anomaly-skeptic".into(),
                        severity: if z_score.abs() > threshold_value * 1.5 {
                            Severity::Blocker
                        } else {
                            Severity::Warning
                        },
                        message: format!(
                            "metric '{}' is anomalous (z={:.2}, mean={:.2}, sd={:.2})",
                            self.metric_field, z_score, output.mean, output.std_dev
                        ),
                    };
                    Some(f)
                }
                None => None,
            };

            match finding {
                Some(f) if f.severity == Severity::Blocker => {
                    effect.push(proposed_text_fact(
                        ContextKey::Constraints,
                        format!("anomaly-block-{id}"),
                        serde_json::json!({
                            "strategy_id": id,
                            "agent": "anomaly-skeptic",
                            "kind": "statistical_anomaly",
                            "severity": "blocker",
                            "findings": [f.message],
                        })
                        .to_string(),
                    ));
                }
                Some(f) => {
                    effect.push(proposed_text_fact(
                        ContextKey::Constraints,
                        format!("anomaly-warn-{id}"),
                        serde_json::json!({
                            "strategy_id": id,
                            "agent": "anomaly-skeptic",
                            "kind": "statistical_anomaly",
                            "severity": "warning",
                            "findings": [f.message],
                        })
                        .to_string(),
                    ));
                }
                None => {
                    effect.push(proposed_text_fact(
                        ContextKey::Evaluations,
                        format!("anomaly-pass-{id}"),
                        serde_json::json!({
                            "strategy_id": id,
                            "agent": "anomaly-skeptic",
                            "kind": "statistical_anomaly",
                            "passed": true,
                            "mean": output.mean,
                            "std_dev": output.std_dev,
                        })
                        .to_string(),
                    ));
                }
            }
        }

        effect.build()
    }
}

/// Discovery descriptor — a small local struct mirroring the shape of
/// `organism_pack::AgentMeta` so an app catalog can convert it without
/// `organism-adversarial` having to depend on `organism-pack` (which would
/// be a cycle: `organism-pack` already depends on `organism-adversarial`).
///
/// Apps that wire the runtime registry construct an `AgentMeta` from this
/// descriptor at registration time:
///
/// ```ignore
/// use organism_pack::AgentMeta;
/// use organism_adversarial::ANOMALY_SKEPTIC_META;
/// let meta = AgentMeta {
///     name: ANOMALY_SKEPTIC_META.name,
///     dependencies: ANOMALY_SKEPTIC_META.dependencies,
///     fact_prefix: ANOMALY_SKEPTIC_META.fact_prefix,
///     target_key: organism_pack::ContextKey::Constraints,
///     description: ANOMALY_SKEPTIC_META.description,
/// };
/// registry.register_pack("anomaly-skeptic", &[meta], &[]);
/// ```
pub struct AnomalySkepticDescriptor {
    pub name: &'static str,
    pub dependencies: &'static [ContextKey],
    pub fact_prefix: &'static str,
    pub description: &'static str,
}

pub const ANOMALY_SKEPTIC_META: AnomalySkepticDescriptor = AnomalySkepticDescriptor {
    name: "anomaly-skeptic",
    dependencies: &[ContextKey::Strategies],
    fact_prefix: "anomaly-",
    description: "Z-score outlier detection over the active strategy set; \
                  flags plans whose key metric is statistically anomalous \
                  relative to the others.",
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn metric_only_plan(value: f64) -> serde_json::Value {
        json!({"annotation": {"total_cost": value}})
    }

    fn agent_default() -> AnomalySkepticAgent {
        AnomalySkepticAgent::default_config()
    }

    #[test]
    fn default_config_threshold_is_two() {
        assert!((agent_default().threshold.value() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn metric_extracted_from_direct_field() {
        let agent = agent_default();
        let plan = metric_only_plan(42.0);
        assert!((agent.extract_metric(&plan).unwrap() - 42.0).abs() < 1e-9);
    }

    #[test]
    fn metric_falls_back_to_costs_sum() {
        let agent = agent_default();
        let plan = json!({
            "annotation": {
                "costs": [
                    {"category": "compute", "estimate": 10.0},
                    {"category": "storage", "estimate": 5.5}
                ]
            }
        });
        assert!((agent.extract_metric(&plan).unwrap() - 15.5).abs() < 1e-9);
    }

    #[test]
    fn metric_returns_none_when_absent() {
        let agent = agent_default();
        let plan = json!({"annotation": {}});
        assert!(agent.extract_metric(&plan).is_none());
    }

    #[test]
    fn metric_field_override_works() {
        let agent = AnomalySkepticAgent::new(ZScoreThreshold::new(2.0).unwrap())
            .with_metric_field("custom_score");
        let plan = json!({"annotation": {"custom_score": 99.0}});
        assert!((agent.extract_metric(&plan).unwrap() - 99.0).abs() < 1e-9);
    }
}
