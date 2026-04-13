// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Batch feature extraction — delegates to converge-analytics.
//!
//! This module provides the spike-specific wrappers around
//! `converge_analytics::batch`, demonstrating the "Polars replaces Spark"
//! pattern without exposing Polars types to the spike.

pub use converge_analytics::batch::{
    TemporalFeatureConfig, TemporalFeatures, extract_temporal_features, temporal_to_feature_vector,
    z_scores,
};

/// Convert temporal features to the JSON format expected by the convergence Context.
pub fn features_to_json(features: &[TemporalFeatures]) -> anyhow::Result<String> {
    Ok(serde_json::to_string(features)?)
}
