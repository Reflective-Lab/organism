// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Multi-agent anomaly scoring for Spike 4.
//!
//! Four independent agents score user sequences through the convergence loop.
//! Each reads features from `Seeds` and emits anomaly scores as `Hypotheses`.
//! The agents fire in parallel (same convergence cycle) because they share
//! the same dependency (`Seeds`) and don't depend on each other.
//!
//! Dependencies: converge-core (Agent trait), converge-analytics (batch, model).
//! No direct polars or burn dependency.

use converge_analytics::batch::{self, TemporalFeatures};
use converge_analytics::model::{ModelConfig, run_batch_inference};
use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

/// Per-user anomaly score from a single agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserScore {
    pub user_id: String,
    pub z_score: f64,
}

fn parse_features(ctx: &Context) -> Vec<TemporalFeatures> {
    ctx.get(ContextKey::Seeds)
        .iter()
        .find(|f| f.id == "user_features")
        .and_then(|f| serde_json::from_str(&f.content).ok())
        .unwrap_or_default()
}

fn emit_scores(fact_id: &str, scores: Vec<UserScore>) -> AgentEffect {
    AgentEffect::with_fact(Fact::new(
        ContextKey::Hypotheses,
        fact_id.to_string(),
        serde_json::json!({ "scores": scores }).to_string(),
    ))
}

// ─── Agent 1: Temporal Anomaly ──────────────────────────────────────────────

/// Scores users based on inter-event timing.
///
/// Low mean delta + low min delta → suspicious (too-fast activity).
/// Uses z-score: higher absolute z = more anomalous.
pub struct TemporalAnomalyAgent;

impl Agent for TemporalAnomalyAgent {
    fn name(&self) -> &str {
        "TemporalAnomalyAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == "user_features")
            && !ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id == "temporal_scores")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let features = parse_features(ctx);
        if features.is_empty() {
            return AgentEffect::with_fact(Fact::new(
                ContextKey::Diagnostic,
                "temporal_error",
                "no features found",
            ));
        }

        let mean_deltas: Vec<f64> = features.iter().map(|f| f.mean_delta_s).collect();
        let z = batch::z_scores(&mean_deltas);

        let scores: Vec<UserScore> = features
            .iter()
            .zip(z.iter())
            .map(|(f, &z)| UserScore {
                user_id: f.entity_id.clone(),
                z_score: -z, // negate: low delta → high positive z
            })
            .collect();

        emit_scores("temporal_scores", scores)
    }
}

// ─── Agent 2: Burst Anomaly ─────────────────────────────────────────────────

/// Scores users based on burst activity (events with < 60s gap).
pub struct BurstAnomalyAgent;

impl Agent for BurstAnomalyAgent {
    fn name(&self) -> &str {
        "BurstAnomalyAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == "user_features")
            && !ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id == "burst_scores")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let features = parse_features(ctx);
        let burst_vals: Vec<f64> = features.iter().map(|f| f64::from(f.burst_score)).collect();
        let z = batch::z_scores(&burst_vals);

        let scores: Vec<UserScore> = features
            .iter()
            .zip(z.iter())
            .map(|(f, &z)| UserScore {
                user_id: f.entity_id.clone(),
                z_score: z,
            })
            .collect();

        emit_scores("burst_scores", scores)
    }
}

// ─── Agent 3: Behavior Anomaly ──────────────────────────────────────────────

/// Scores users based on behavioral patterns: event type entropy + night ratio.
pub struct BehaviorAnomalyAgent;

impl Agent for BehaviorAnomalyAgent {
    fn name(&self) -> &str {
        "BehaviorAnomalyAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == "user_features")
            && !ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id == "behavior_scores")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let features = parse_features(ctx);

        let entropies: Vec<f64> = features.iter().map(|f| f.type_entropy).collect();
        let nights: Vec<f64> = features.iter().map(|f| f.night_ratio).collect();
        let z_entropy = batch::z_scores(&entropies);
        let z_night = batch::z_scores(&nights);

        let scores: Vec<UserScore> = features
            .iter()
            .enumerate()
            .map(|(i, f)| UserScore {
                user_id: f.entity_id.clone(),
                z_score: -z_entropy[i] + z_night[i],
            })
            .collect();

        emit_scores("behavior_scores", scores)
    }
}

// ─── Agent 4: Burn ML Scoring ───────────────────────────────────────────────

/// Burn-based neural network for anomaly classification.
///
/// Uses `converge_analytics::model::run_batch_inference()` — never touches
/// Burn types directly. The spike only sees `ModelConfig` and `FeatureVector`.
pub struct BurnScoringAgent;

impl Agent for BurnScoringAgent {
    fn name(&self) -> &str {
        "BurnScoringAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == "user_features")
            && !ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id == "ml_scores")
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let features = parse_features(ctx);
        if features.is_empty() {
            return AgentEffect::with_fact(Fact::new(
                ContextKey::Diagnostic,
                "ml_error",
                "no features for inference",
            ));
        }

        // Convert to FeatureVector [n, 8] using converge-analytics helper.
        let fv = match batch::temporal_to_feature_vector(&features) {
            Ok(fv) => fv,
            Err(e) => {
                return AgentEffect::with_fact(Fact::new(
                    ContextKey::Diagnostic,
                    "ml_error",
                    format!("feature conversion failed: {e}"),
                ));
            }
        };

        // Run inference via converge-analytics model abstraction.
        let config = ModelConfig::new(8, 16, 1);
        let raw = match run_batch_inference(&config, &fv) {
            Ok(r) => r,
            Err(e) => {
                return AgentEffect::with_fact(Fact::new(
                    ContextKey::Diagnostic,
                    "ml_error",
                    format!("inference failed: {e}"),
                ));
            }
        };

        // Z-score normalize the raw model output.
        let raw_f64: Vec<f64> = raw.iter().map(|v| f64::from(*v)).collect();
        let z = batch::z_scores(&raw_f64);

        let scores: Vec<UserScore> = features
            .iter()
            .enumerate()
            .map(|(i, f)| UserScore {
                user_id: f.entity_id.clone(),
                z_score: z[i],
            })
            .collect();

        emit_scores("ml_scores", scores)
    }
}
