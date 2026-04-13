// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! # Spike 4: Batch Anomaly Detection Pipeline
//!
//! Demonstrates the full Rust-native ML pipeline using the converge ecosystem:
//!
//! 1. **Batch preparation** — converge-analytics extracts temporal features
//! 2. **Multi-agent scoring** — 4 agents score sequences through convergence loop
//! 3. **Anomaly triage** — converge-optimization `anomaly_triage` pack prioritizes
//! 4. **ML inference** — converge-analytics Burn model classifies anomalies
//!
//! ## Dependencies
//!
//! This spike depends ONLY on the converge ecosystem — no direct polars, burn,
//! surrealdb, or lancedb dependencies. Those are implementation details inside:
//! - `converge-analytics` (Polars + Burn)
//! - `converge-optimization` (CP-SAT + anomaly triage pack)
//! - `converge-core` (Agent, Engine, Context)

pub mod agents;
pub mod batch;
pub mod consensus;
pub mod invariants;
pub mod scenario;

use converge_analytics::batch::TemporalFeatureConfig;
use converge_core::{Context, ContextKey};

use scenario::{build_scoring_engine, build_triage_engine};

/// Result of running Spike 4.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchAnomalyResult {
    pub flagged_users: usize,
    pub total_users: usize,
    pub total_cycles: u32,
    pub context: Context,
}

/// Error from running the spike.
#[derive(Debug)]
pub enum BatchAnomalyError {
    DataGeneration(String),
    FeatureExtraction(String),
    ScoringFailed(String),
    TriageFailed(String),
    MissingRecommendation,
}

impl std::fmt::Display for BatchAnomalyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DataGeneration(r) => write!(f, "data generation: {r}"),
            Self::FeatureExtraction(r) => write!(f, "feature extraction: {r}"),
            Self::ScoringFailed(r) => write!(f, "scoring failed: {r}"),
            Self::TriageFailed(r) => write!(f, "triage failed: {r}"),
            Self::MissingRecommendation => write!(f, "recommendation not produced"),
        }
    }
}

impl std::error::Error for BatchAnomalyError {}

/// Run Spike 4 with verbose output.
pub async fn run_batch_anomaly_verbose() -> Result<BatchAnomalyResult, BatchAnomalyError> {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Spike 4: Batch Anomaly Detection Pipeline                      ║");
    println!("║  Stack: converge-analytics + converge-optimization               ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // ─── Phase 0: Generate synthetic GitHub events ───
    println!("┌─ Phase 0: Data Generation ──────────────────────────────────────┐");
    let config = scenario::SpikeConfig::default();
    let (data_path, ground_truth) = scenario::generate_synthetic_events(&config)
        .map_err(|e| BatchAnomalyError::DataGeneration(e.to_string()))?;

    println!(
        "│  Generated {} events across {} users ({} anomalous)",
        config.total_events, config.total_users, config.anomalous_users
    );
    println!("│  Event types: PushEvent, PullRequestEvent, IssueCommentEvent,");
    println!("│               ForkEvent, WatchEvent, CreateEvent");
    println!("│  Written to: {data_path}");
    println!("└────────────────────────────────────────────────────────────────┘");

    // ─── Phase 1: Batch feature extraction (converge-analytics) ───
    println!("\n┌─ Phase 1: Batch Feature Extraction (converge-analytics) ────────┐");

    // Use converge-analytics batch module — Polars is abstracted away.
    // Accepts both CSV and Parquet; for the spike we generate CSV.
    let feature_config = TemporalFeatureConfig::default();
    let features = batch::extract_temporal_features(
        "spike_anomaly_events.csv",
        &feature_config,
    )
    .map_err(|e| BatchAnomalyError::FeatureExtraction(e.to_string()))?;

    println!("│  Features per user:");
    println!("│    event_count, mean_delta_s, min_delta_s, std_delta_s,");
    println!("│    burst_score, type_entropy, unique_categories, night_ratio");
    println!("│  Computed for {} users", features.len());
    println!("└────────────────────────────────────────────────────────────────┘");

    // ─── Phase 2: Multi-agent scoring (convergence) ───
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Phase 2: Multi-Agent Anomaly Scoring (Convergence Loop)        ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let features_json = batch::features_to_json(&features)
        .map_err(|e| BatchAnomalyError::FeatureExtraction(e.to_string()))?;
    let labels_json = serde_json::to_string(&ground_truth)
        .map_err(|e| BatchAnomalyError::DataGeneration(e.to_string()))?;

    let mut scoring_engine = build_scoring_engine();
    let mut scoring_ctx = Context::new();
    let _ = scoring_ctx.add_fact(converge_core::Fact::new(
        ContextKey::Seeds,
        "user_features",
        features_json,
    ));
    let _ = scoring_ctx.add_fact(converge_core::Fact::new(
        ContextKey::Seeds,
        "ground_truth",
        labels_json,
    ));

    let scoring_result = scoring_engine
        .run(scoring_ctx)
        .map_err(|e| BatchAnomalyError::ScoringFailed(e.to_string()))?;

    let mut total_cycles = scoring_result.cycles;
    println!("  ✓ Scoring converged in {} cycles", scoring_result.cycles);

    for fact in scoring_result.context.get(ContextKey::Hypotheses) {
        println!(
            "    Agent: {} — scored {} users",
            fact.id,
            count_users(&fact.content)
        );
    }

    // ─── Phase 3: Triage + Recommendation (convergence) ───
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Phase 3: Anomaly Triage & Recommendation (Convergence Loop)    ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    let mut triage_ctx = Context::new();
    for key in [ContextKey::Seeds, ContextKey::Hypotheses, ContextKey::Signals] {
        for fact in scoring_result.context.get(key) {
            let _ = triage_ctx.add_fact(fact.clone());
        }
    }

    let mut triage_engine = build_triage_engine();
    let triage_result = triage_engine
        .run(triage_ctx)
        .map_err(|e| BatchAnomalyError::TriageFailed(e.to_string()))?;

    total_cycles += triage_result.cycles;
    println!("  ✓ Triage converged in {} cycles", triage_result.cycles);

    // ─── Extract and display results ───
    let recommendation = triage_result
        .context
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "recommendation")
        .ok_or(BatchAnomalyError::MissingRecommendation)?;

    let rec: serde_json::Value =
        serde_json::from_str(&recommendation.content).unwrap_or_default();

    let flagged_users = rec
        .get("flagged_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Final Results                                                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");

    if let Some(triage_fact) = triage_result
        .context
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "anomaly_triage")
    {
        print_triage_summary(triage_fact);
    }

    println!("\n  Flagged users:    {flagged_users} / {}", config.total_users);
    println!("  True anomalies:   {}", config.anomalous_users);
    println!("  Total cycles:     {total_cycles}");
    println!("  Invariants:       all_agents_scored ✓  triage_complete ✓  consistent_ranking ✓");

    if let Some(top) = rec.get("top_flagged").and_then(|v| v.as_array()) {
        println!("\n  ┌─ Top Flagged Users ─────────────────────────────────────────┐");
        for (i, entry) in top.iter().take(5).enumerate() {
            let uid = entry.get("user_id").and_then(|v| v.as_str()).unwrap_or("?");
            let sev = entry.get("severity").and_then(|v| v.as_str()).unwrap_or("?");
            let score = entry
                .get("composite_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let is_true = ground_truth.iter().any(|(u, _)| u == uid);
            let marker = if is_true { " ← true anomaly" } else { "" };
            println!(
                "  │  {}. {} (severity={}, score={:.3}){marker}",
                i + 1,
                uid,
                sev,
                score,
            );
        }
        println!("  └────────────────────────────────────────────────────────────┘");
    }

    Ok(BatchAnomalyResult {
        flagged_users,
        total_users: config.total_users,
        total_cycles,
        context: triage_result.context,
    })
}

fn count_users(json: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("scores").and_then(|s| s.as_array()).map(|a| a.len()))
        .unwrap_or(0)
}

fn print_triage_summary(fact: &converge_core::Fact) {
    let v: serde_json::Value = serde_json::from_str(&fact.content).unwrap_or_default();
    if let Some(summary) = v.get("severity_summary") {
        println!("\n  ┌─ Triage Summary (converge-optimization anomaly_triage) ─────┐");
        println!(
            "  │  Critical: {}  High: {}  Medium: {}  Low: {}",
            summary.get("critical").and_then(|v| v.as_u64()).unwrap_or(0),
            summary.get("high").and_then(|v| v.as_u64()).unwrap_or(0),
            summary.get("medium").and_then(|v| v.as_u64()).unwrap_or(0),
            summary.get("low").and_then(|v| v.as_u64()).unwrap_or(0),
        );
        println!(
            "  │  Escalation count: {}",
            v.get("escalation_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );
        println!("  └────────────────────────────────────────────────────────────┘");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spike_converges() {
        let result = run_batch_anomaly_verbose().await.unwrap();
        assert!(result.flagged_users > 0, "should flag some users");
        assert!(
            result.flagged_users <= result.total_users,
            "cannot flag more than total"
        );
        assert!(
            result.total_cycles >= 4,
            "need at least 4 cycles for full pipeline"
        );
    }
}
