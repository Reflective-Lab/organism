// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Provenance types for intelligence-provider calls.
//!
//! These describe a third-party provider call and the audit metadata
//! that comes back with the response: vendor, model, latency, cost
//! estimate, token usage, raw payload. They are **not** the same
//! concept as `converge_core::Observation` (which is a
//! content-addressed kernel observation tied to `ObservationId` /
//! `ContentHash` / `CaptureContext`) or anything in
//! `converge_provider` — those serve the kernel's audit and capture
//! flow. These types serve `organism-intelligence`'s API-call layer
//! and are intentionally distinct: an `Observation<T>` here wraps a
//! typed `content: T` (e.g. `SocialProfile`, `WebDocument`,
//! `PatentResult`) together with cost and usage metadata that the
//! kernel does not need to model.

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

/// SHA-256 digest of `input`, hex-encoded. Used for content-addressed provenance.
pub fn content_hash(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    digest.iter().fold(String::with_capacity(64), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}
