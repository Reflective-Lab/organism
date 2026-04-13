// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Social page extraction built on top of web capture.
//!
//! This module normalizes public profile/page URLs from external social systems.
//! It deliberately sits above generic URL fetch, so applications can treat
//! LinkedIn/X/Instagram/Facebook as external ports while sharing one reusable
//! extraction capability in Organism.

use crate::provenance::{CallContext, Observation, content_hash};
use crate::web::{WebCaptureProvider, WebCaptureRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialPlatform {
    Linkedin,
    X,
    Instagram,
    Facebook,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialExtractRequest {
    pub url: String,
}

impl SocialExtractRequest {
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialProfile {
    pub platform: SocialPlatform,
    pub requested_url: String,
    pub canonical_url: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub headline: Option<String>,
    pub description: Option<String>,
    pub outbound_links: Vec<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialExtractResponse {
    pub profile: Observation<SocialProfile>,
}

pub trait SocialExtractProvider: Send + Sync {
    fn name(&self) -> &str;
    fn extract(
        &self,
        request: &SocialExtractRequest,
        ctx: &CallContext,
    ) -> Result<SocialExtractResponse, String>;
}

pub struct WebCaptureSocialExtractProvider {
    capture: Box<dyn WebCaptureProvider>,
}

impl WebCaptureSocialExtractProvider {
    #[must_use]
    pub fn new(capture: Box<dyn WebCaptureProvider>) -> Self {
        Self { capture }
    }
}

impl SocialExtractProvider for WebCaptureSocialExtractProvider {
    fn name(&self) -> &str {
        "web_capture_social_extract"
    }

    fn extract(
        &self,
        request: &SocialExtractRequest,
        ctx: &CallContext,
    ) -> Result<SocialExtractResponse, String> {
        let capture = self
            .capture
            .capture(&WebCaptureRequest::new(request.url.clone()), ctx)?;
        let document = &capture.capture.content;
        let canonical_url = document
            .canonical_url
            .clone()
            .unwrap_or_else(|| document.final_url.clone());
        let platform = detect_platform(&canonical_url);
        let handle = extract_handle(&canonical_url, platform);
        let display_name = document
            .title
            .clone()
            .map(|title| clean_title(&title, platform));
        let hash_input = format!("{}:{}", request.url, canonical_url);

        Ok(SocialExtractResponse {
            profile: Observation {
                observation_id: format!("obs:social:{}", content_hash(&hash_input)),
                request_hash: content_hash(&hash_input),
                vendor: self.capture.name().to_string(),
                model: self.name().to_string(),
                latency_ms: capture.capture.latency_ms,
                cost_estimate: capture.capture.cost_estimate,
                tokens: capture.capture.tokens,
                content: SocialProfile {
                    platform,
                    requested_url: request.url.clone(),
                    canonical_url,
                    handle,
                    display_name,
                    headline: document.site_name.clone(),
                    description: document.description.clone(),
                    outbound_links: document
                        .links
                        .iter()
                        .map(|link| link.href.clone())
                        .collect(),
                    metadata: serde_json::json!({
                        "capture_vendor": self.capture.name(),
                        "status_code": document.status_code,
                    }),
                },
                raw_response: capture.capture.raw_response.clone(),
            },
        })
    }
}

fn detect_platform(url: &str) -> SocialPlatform {
    let lower = url.to_ascii_lowercase();
    if lower.contains("linkedin.com") {
        SocialPlatform::Linkedin
    } else if lower.contains("x.com") || lower.contains("twitter.com") {
        SocialPlatform::X
    } else if lower.contains("instagram.com") {
        SocialPlatform::Instagram
    } else if lower.contains("facebook.com") {
        SocialPlatform::Facebook
    } else {
        SocialPlatform::Unknown
    }
}

fn extract_handle(url: &str, platform: SocialPlatform) -> Option<String> {
    let path_segments = url.split("://").nth(1).map(|rest| {
        rest.split('/')
            .skip(1)
            .map(|segment| segment.split(['?', '#']).next().unwrap_or(segment))
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>()
    })?;
    if path_segments.is_empty() {
        return None;
    }

    match platform {
        SocialPlatform::Linkedin => match path_segments.first().copied() {
            Some("in") | Some("company") => path_segments.get(1).map(|value| (*value).to_string()),
            Some(value) => Some(value.to_string()),
            None => None,
        },
        _ => path_segments.first().map(|value| (*value).to_string()),
    }
}

fn clean_title(title: &str, platform: SocialPlatform) -> String {
    let separators = match platform {
        SocialPlatform::Linkedin => [" | LinkedIn", " - LinkedIn"],
        SocialPlatform::X => [" / X", " / Twitter"],
        SocialPlatform::Instagram => [" • Instagram photos and videos", " | Instagram"],
        SocialPlatform::Facebook => [" | Facebook", " - Facebook"],
        SocialPlatform::Unknown => ["", ""],
    };

    for separator in separators {
        if !separator.is_empty() {
            if let Some((head, _)) = title.split_once(separator) {
                return head.trim().to_string();
            }
        }
    }

    title.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        SocialExtractProvider, SocialExtractRequest, SocialPlatform,
        WebCaptureSocialExtractProvider, clean_title, detect_platform, extract_handle,
    };
    use crate::provenance::CallContext;
    use crate::web::StubWebCaptureProvider;

    #[test]
    fn detects_supported_platforms() {
        assert_eq!(
            detect_platform("https://linkedin.com/in/jane-doe"),
            SocialPlatform::Linkedin
        );
        assert_eq!(detect_platform("https://x.com/jane"), SocialPlatform::X);
        assert_eq!(
            detect_platform("https://instagram.com/jane"),
            SocialPlatform::Instagram
        );
        assert_eq!(
            detect_platform("https://facebook.com/jane"),
            SocialPlatform::Facebook
        );
    }

    #[test]
    fn extracts_handles_and_cleans_titles() {
        assert_eq!(
            extract_handle(
                "https://linkedin.com/company/acme",
                SocialPlatform::Linkedin
            )
            .as_deref(),
            Some("acme")
        );
        assert_eq!(
            clean_title("Jane Doe | LinkedIn", SocialPlatform::Linkedin),
            "Jane Doe"
        );
    }

    #[test]
    fn builds_social_profile_from_capture() {
        let provider = WebCaptureSocialExtractProvider::new(Box::new(StubWebCaptureProvider));
        let response = provider
            .extract(
                &SocialExtractRequest::new("https://linkedin.com/in/jane-doe"),
                &CallContext::default(),
            )
            .expect("extract should succeed");

        assert_eq!(response.profile.content.platform, SocialPlatform::Linkedin);
        assert_eq!(response.profile.content.handle.as_deref(), Some("jane-doe"));
    }
}
