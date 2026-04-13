// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Patent search provider — IP landscape, competitive intelligence.
//!
//! Migration source: `converge-provider/src/patent.rs` (867 lines).
//! The full implementation includes USPTO, EPO, WIPO, Google Patents,
//! and Lens operators with real API integration.

use crate::provenance::{CallContext, Observation, content_hash};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatentOperator {
    Uspto,
    Epo,
    Wipo,
    GooglePatents,
    Lens,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatentSearchRequest {
    pub query: String,
    pub operators: Vec<PatentOperator>,
    pub max_results: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatentResult {
    pub patent_id: String,
    pub title: String,
    pub abstract_text: String,
    pub assignee: Option<String>,
    pub filing_date: Option<String>,
    pub operator: PatentOperator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatentSearchResponse {
    pub results: Vec<Observation<PatentResult>>,
}

pub trait PatentSearchProvider: Send + Sync {
    fn name(&self) -> &str;
    fn search(
        &self,
        request: &PatentSearchRequest,
        ctx: &CallContext,
    ) -> Result<PatentSearchResponse, String>;
}

#[derive(Debug, Clone, Default)]
pub struct StubPatentProvider;

impl PatentSearchProvider for StubPatentProvider {
    fn name(&self) -> &'static str {
        "stub_patent"
    }

    fn search(
        &self,
        request: &PatentSearchRequest,
        _ctx: &CallContext,
    ) -> Result<PatentSearchResponse, String> {
        let obs = Observation {
            observation_id: format!("obs:patent:{}", content_hash(&request.query)),
            request_hash: content_hash(&request.query),
            vendor: "stub_patent".to_string(),
            model: "stub".to_string(),
            latency_ms: 50,
            cost_estimate: None,
            tokens: None,
            content: PatentResult {
                patent_id: "US-STUB-001".to_string(),
                title: format!("Stub patent for: {}", request.query),
                abstract_text: "Stub abstract".to_string(),
                assignee: Some("Stub Corp".to_string()),
                filing_date: Some("2025-01-01".to_string()),
                operator: PatentOperator::Uspto,
            },
            raw_response: None,
        };
        Ok(PatentSearchResponse { results: vec![obs] })
    }
}
