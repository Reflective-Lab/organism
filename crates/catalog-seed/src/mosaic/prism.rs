//! Descriptors for `converge-prism-analytics` Suggestors.
//!
//! Authored against `converge-prism-analytics = "2.0.0"`. Prism exposes
//! analytics primitives (fuzzy inference, ML prediction, anomaly
//! detection) packaged behind the Suggestor surface.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![fuzzy_inference(), ml_prediction(), anomaly_detection()]
}

#[must_use]
pub fn fuzzy_inference() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "prism-fuzzy-inference",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Signals, ContextKey::Hypotheses],
        domain_tags: vec!["analytics", "fuzzy", "inference"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Fuzzy-logic inference over linguistic variables (low/medium/high).",
        use_when: "When ranking decisions on soft criteria that don't have crisp thresholds.",
        examples: vec![
            "score vendor reliability on soft criteria",
            "rank options by 'fit' rather than hard score",
        ],
        loop_contributions: vec![LoopContribution::Score],
        produces: vec!["prism.analytics.fuzzy-score"],
    })
}

#[must_use]
pub fn ml_prediction() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "prism-ml-prediction",
        role: SuggestorRole::Evaluation,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Evaluations],
        reads: vec![ContextKey::Signals, ContextKey::Hypotheses],
        domain_tags: vec!["analytics", "ml", "prediction"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Score candidates against a trained ML prediction model.",
        use_when: "When a labeled history exists and a learned model can rank better than rules.",
        examples: vec![
            "predict the close probability of this deal",
            "score candidates on conversion likelihood",
        ],
        loop_contributions: vec![LoopContribution::Score],
        produces: vec!["prism.analytics.ml-prediction"],
    })
}

#[must_use]
pub fn anomaly_detection() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "prism-anomaly-detection",
        role: SuggestorRole::Analysis,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Diagnostic, ContextKey::Disagreements],
        reads: vec![ContextKey::Signals],
        domain_tags: vec!["analytics", "anomaly", "outlier"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Flag statistical outliers in numeric signals.",
        use_when: "When signals stream in and unusual values need to be raised for review.",
        examples: vec![
            "is this conversion rate an outlier",
            "detect anomalies in the daily metric stream",
        ],
        loop_contributions: vec![LoopContribution::Observe, LoopContribution::Challenge],
        produces: vec!["prism.analytics.anomaly"],
    })
}
