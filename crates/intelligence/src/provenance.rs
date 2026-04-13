// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Provenance types for intelligence observations.
//!
//! These are local equivalents of the converge-provider contract types,
//! decoupled from Converge internals.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for a provider call — correlation, timing, metadata.
#[derive(Debug, Clone, Default)]
pub struct CallContext {
    pub correlation_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// An observation from an intelligence provider, with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation<T> {
    pub observation_id: String,
    pub request_hash: String,
    pub vendor: String,
    pub model: String,
    pub latency_ms: u64,
    pub cost_estimate: Option<f64>,
    pub tokens: Option<u64>,
    pub content: T,
    pub raw_response: Option<String>,
}

/// SHA-256 hash for provenance tracking.
pub fn content_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
