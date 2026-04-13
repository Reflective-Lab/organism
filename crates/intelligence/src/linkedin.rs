// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! LinkedIn provider — professional network research.

use crate::provenance::{CallContext, Observation, content_hash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInGetRequest {
    pub endpoint: String,
    pub query: HashMap<String, String>,
}

impl LinkedInGetRequest {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            query: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInProfile {
    pub profile_id: String,
    pub name: String,
    pub title: Option<String>,
    pub company: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedInGetResponse {
    pub records: Vec<Observation<LinkedInProfile>>,
}

pub trait LinkedInProvider: Send + Sync {
    fn name(&self) -> &str;
    fn get(
        &self,
        request: &LinkedInGetRequest,
        ctx: &CallContext,
    ) -> Result<LinkedInGetResponse, String>;
}

#[derive(Debug, Clone, Default)]
pub struct StubLinkedInProvider;

impl LinkedInProvider for StubLinkedInProvider {
    fn name(&self) -> &'static str {
        "stub_linkedin"
    }

    fn get(
        &self,
        request: &LinkedInGetRequest,
        _ctx: &CallContext,
    ) -> Result<LinkedInGetResponse, String> {
        if request.endpoint.trim().is_empty() {
            return Err("Empty endpoint".to_string());
        }
        let hash_input = format!("{}:{:?}", request.endpoint, request.query);
        let obs = Observation {
            observation_id: format!("obs:linkedin:{}", content_hash(&hash_input)),
            request_hash: content_hash(&hash_input),
            vendor: "stub_linkedin".to_string(),
            model: "stub".to_string(),
            latency_ms: 10,
            cost_estimate: None,
            tokens: None,
            content: LinkedInProfile {
                profile_id: "LI-STUB-001".to_string(),
                name: "Jane Doe".to_string(),
                title: Some("VP Engineering".to_string()),
                company: Some("Acme Corp".to_string()),
                payload: serde_json::json!({"name": "Jane Doe"}),
            },
            raw_response: None,
        };
        Ok(LinkedInGetResponse { records: vec![obs] })
    }
}
