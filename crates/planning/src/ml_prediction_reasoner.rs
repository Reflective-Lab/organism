//! ML-prediction reasoner — wraps prism's regression and classification packs.
//!
//! Maps onto the existing `ReasoningSystem::MlPrediction` slot (no enum
//! change). Inputs are pre-trained weights + bias provided at construction
//! time and a feature vector extracted from `IntentPacket::context` at
//! propose-time. Output:
//!
//! - **Regression mode** — a single `Impact` carrying the predicted scalar
//!   in its description and a unit confidence (regression has no native
//!   probability — apps that want graded confidence should layer
//!   `FuzzyReasoner` on top of the regression output).
//! - **Classification mode** — one `Impact` for the predicted class with
//!   `confidence = sigmoid(w·x + b)` (positive class) or `1 - sigmoid` (negative class).

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use converge_pack::gate::{ObjectiveSpec, ProblemSpec};
use organism_intent::IntentPacket;
use prism::UnitFraction;
use prism::packs::classification::{ClassificationInput, LogisticClassifier};
use prism::packs::regression::{LinearRegressionSolver, RegressionInput};

use crate::{Impact, Plan, PlanContribution, Reasoner, ReasoningSystem};

#[derive(Debug, Clone)]
pub enum MlPredictionMode {
    Regression {
        weights: Arc<Vec<f64>>,
        bias: f64,
    },
    Classification {
        weights: Arc<Vec<f64>>,
        bias: f64,
        threshold: UnitFraction,
    },
}

/// A reasoner that produces a plan whose impact is a pre-trained ML model's
/// prediction over a feature vector lifted from the intent's context.
pub struct MlPredictionReasoner {
    name: String,
    mode: MlPredictionMode,
    /// Name of the field in `intent.context` that holds the feature vector
    /// (a JSON array of finite numbers).
    feature_field: String,
}

impl MlPredictionReasoner {
    pub fn regression(name: impl Into<String>, weights: Vec<f64>, bias: f64) -> Self {
        Self {
            name: name.into(),
            mode: MlPredictionMode::Regression {
                weights: Arc::new(weights),
                bias,
            },
            feature_field: "features".into(),
        }
    }

    pub fn classification(
        name: impl Into<String>,
        weights: Vec<f64>,
        bias: f64,
        threshold: UnitFraction,
    ) -> Self {
        Self {
            name: name.into(),
            mode: MlPredictionMode::Classification {
                weights: Arc::new(weights),
                bias,
                threshold,
            },
            feature_field: "features".into(),
        }
    }

    #[must_use]
    pub fn with_feature_field(mut self, field: impl Into<String>) -> Self {
        self.feature_field = field.into();
        self
    }

    fn extract_features(&self, context: &serde_json::Value) -> Option<Vec<f64>> {
        context
            .get(&self.feature_field)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(serde_json::Value::as_f64)
                    .collect::<Vec<f64>>()
            })
            .filter(|v| !v.is_empty() && v.iter().all(|x| x.is_finite()))
    }

    fn problem_spec(&self) -> Result<ProblemSpec> {
        ProblemSpec::builder(format!("ml-prediction:{}", self.name), "ml-prediction")
            .objective(ObjectiveSpec::maximize("prediction"))
            .build()
            .map_err(|e| anyhow::anyhow!("ml-prediction reasoner could not build ProblemSpec: {e}"))
    }

    fn predict(&self, features: Vec<f64>) -> Result<(String, f64)> {
        let spec = self.problem_spec()?;
        match &self.mode {
            MlPredictionMode::Regression { weights, bias } => {
                if features.len() != weights.len() {
                    anyhow::bail!(
                        "regression expected {} features, got {}",
                        weights.len(),
                        features.len()
                    );
                }
                let input = RegressionInput {
                    records: vec![features],
                    weights: weights.as_ref().clone(),
                    bias: *bias,
                };
                let (output, _) = LinearRegressionSolver
                    .solve(&input, &spec)
                    .map_err(|e| anyhow::anyhow!("regression failed: {e}"))?;
                let value = output
                    .predictions
                    .first()
                    .map(|p| p.value)
                    .ok_or_else(|| anyhow::anyhow!("regression produced no predictions"))?;
                Ok((format!("regression: predicted value {value:.6}"), 1.0))
            }
            MlPredictionMode::Classification {
                weights,
                bias,
                threshold,
            } => {
                if features.len() != weights.len() {
                    anyhow::bail!(
                        "classification expected {} features, got {}",
                        weights.len(),
                        features.len()
                    );
                }
                let input = ClassificationInput {
                    records: vec![features],
                    weights: weights.as_ref().clone(),
                    bias: *bias,
                    threshold: *threshold,
                    labels: None,
                };
                let (output, _) = LogisticClassifier
                    .solve(&input, &spec)
                    .map_err(|e| anyhow::anyhow!("classification failed: {e}"))?;
                let prediction = output
                    .predictions
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("classification produced no predictions"))?;
                let threshold_value = threshold.value();
                let positive = prediction.probability >= threshold_value;
                let label = if positive { "positive" } else { "negative" };
                let confidence = if positive {
                    prediction.probability
                } else {
                    1.0 - prediction.probability
                };
                Ok((
                    format!(
                        "classification: predicted {label} (p={:.4}, threshold={:.2})",
                        prediction.probability, threshold_value
                    ),
                    confidence,
                ))
            }
        }
    }
}

#[async_trait]
impl Reasoner for MlPredictionReasoner {
    fn name(&self) -> &str {
        &self.name
    }

    fn system_type(&self) -> ReasoningSystem {
        ReasoningSystem::MlPrediction
    }

    async fn propose(&self, intent: &IntentPacket) -> Result<Plan> {
        let features = self.extract_features(&intent.context).ok_or_else(|| {
            anyhow::anyhow!(
                "ml-prediction reasoner requires '{}' field with a numeric array in intent.context",
                self.feature_field
            )
        })?;

        let (description, confidence) = self.predict(features)?;
        let impact = Impact {
            description: description.clone(),
            confidence,
        };

        let mut plan = Plan::new(intent, format!("ml-prediction: {description}"));
        plan.annotation.impacts = vec![impact];
        plan.contributor = ReasoningSystem::MlPrediction;
        Ok(plan)
    }

    fn contribute(&self, context: &serde_json::Value) -> PlanContribution {
        let suggestions = match self.extract_features(context) {
            Some(features) => match self.predict(features) {
                Ok((desc, confidence)) => vec![format!("{desc} (confidence {confidence:.3})")],
                Err(e) => vec![format!("ml-prediction error: {e}")],
            },
            None => vec![format!(
                "ml-prediction: no '{}' field in context",
                self.feature_field
            )],
        };

        PlanContribution {
            system: ReasoningSystem::MlPrediction,
            suggestions,
            constraints: vec![],
            risks: vec![],
        }
    }
}

/// Discovery descriptor for `MlPredictionReasoner`.
///
/// Reasoners participate in huddles, not in the converge `Suggestor`-based
/// formation registry, so this is a documentation hint rather than something
/// the runtime registry consumes directly. Apps register a constructed
/// `MlPredictionReasoner` instance in their huddle setup.
pub struct MlPredictionReasonerDescriptor {
    pub system: ReasoningSystem,
    pub name: &'static str,
    pub description: &'static str,
}

pub const ML_PREDICTION_REASONER_META: MlPredictionReasonerDescriptor =
    MlPredictionReasonerDescriptor {
        system: ReasoningSystem::MlPrediction,
        name: "ml-prediction-reasoner",
        description: "Pre-trained linear regression and logistic classification \
                  via prism::packs::regression and prism::packs::classification; \
                  features are lifted from intent.context.",
    };

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    #[tokio::test]
    async fn regression_reasoner_predicts_known_value() {
        // Model: y = 2*x1 + 3*x2 + 1
        // Features [4, 5] → 2*4 + 3*5 + 1 = 8 + 15 + 1 = 24
        let reasoner = MlPredictionReasoner::regression("test-reg", vec![2.0, 3.0], 1.0);
        let intent = IntentPacket::new("predict y", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "features": [4.0, 5.0] }));

        let plan = reasoner.propose(&intent).await.unwrap();
        assert_eq!(plan.contributor, ReasoningSystem::MlPrediction);
        assert_eq!(plan.annotation.impacts.len(), 1);
        let impact = &plan.annotation.impacts[0];
        assert!(impact.description.contains("24"));
        assert!((impact.confidence - 1.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn classification_reasoner_predicts_positive() {
        // Model: sigmoid(3*x1 + 0*x2 - 1.5)
        // Features [1, 0] → 3 - 1.5 = 1.5 → sigmoid(1.5) ≈ 0.8176 → positive (>= 0.5)
        let reasoner = MlPredictionReasoner::classification(
            "test-cls",
            vec![3.0, 0.0],
            -1.5,
            UnitFraction::new(0.5).unwrap(),
        );
        let intent = IntentPacket::new("classify", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "features": [1.0, 0.0] }));

        let plan = reasoner.propose(&intent).await.unwrap();
        assert_eq!(plan.contributor, ReasoningSystem::MlPrediction);
        let impact = &plan.annotation.impacts[0];
        assert!(impact.description.contains("positive"));
        // Confidence ≈ 0.8176 since predicted positive
        let expected = 1.0 / (1.0 + (-1.5_f64).exp());
        assert!((impact.confidence - expected).abs() < 1e-6);
    }

    #[tokio::test]
    async fn classification_reasoner_predicts_negative() {
        // Features [0, 0] → 0 - 1.5 = -1.5 → sigmoid(-1.5) ≈ 0.1824 → negative
        let reasoner = MlPredictionReasoner::classification(
            "test-cls",
            vec![3.0, 0.0],
            -1.5,
            UnitFraction::new(0.5).unwrap(),
        );
        let intent = IntentPacket::new("classify", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "features": [0.0, 0.0] }));

        let plan = reasoner.propose(&intent).await.unwrap();
        let impact = &plan.annotation.impacts[0];
        assert!(impact.description.contains("negative"));
        // Confidence for negative class = 1 - sigmoid(-1.5) ≈ 0.8176
        let prob = 1.0 / (1.0 + 1.5_f64.exp());
        let expected = 1.0 - prob;
        assert!((impact.confidence - expected).abs() < 1e-6);
    }

    #[tokio::test]
    async fn missing_features_field_yields_error() {
        let reasoner = MlPredictionReasoner::regression("test", vec![1.0], 0.0);
        let intent = IntentPacket::new("predict", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "wrong_field": [1.0] }));
        assert!(reasoner.propose(&intent).await.is_err());
    }

    #[tokio::test]
    async fn dimension_mismatch_yields_error() {
        let reasoner = MlPredictionReasoner::regression("test", vec![1.0, 2.0], 0.0);
        let intent = IntentPacket::new("predict", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "features": [3.0] })); // only 1 feature
        assert!(reasoner.propose(&intent).await.is_err());
    }

    #[test]
    fn contribute_returns_prediction_summary() {
        let reasoner = MlPredictionReasoner::regression("test", vec![2.0, 3.0], 1.0);
        let context = serde_json::json!({ "features": [4.0, 5.0] });
        let contribution = reasoner.contribute(&context);
        assert_eq!(contribution.system, ReasoningSystem::MlPrediction);
        assert!(contribution.suggestions[0].contains("24"));
    }

    #[test]
    fn contribute_handles_missing_features() {
        let reasoner = MlPredictionReasoner::regression("test", vec![1.0], 0.0);
        let context = serde_json::json!({});
        let contribution = reasoner.contribute(&context);
        assert!(contribution.suggestions[0].contains("no 'features' field"));
    }
}
