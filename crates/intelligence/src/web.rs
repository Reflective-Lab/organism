// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Web capture — URL fetching and public-page metadata extraction.
//!
//! This is the reusable business-intelligence seam for fetching public pages
//! that may later be normalized into application-specific ports. Search and
//! crawl orchestration can stay in Converge, while Organism owns the typed
//! capture result that downstream app logic can enrich or publish.

use crate::provenance::{CallContext, Observation, content_hash};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebCaptureMode {
    Http,
    Browser,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebCaptureRequest {
    pub url: String,
    pub mode: WebCaptureMode,
    pub user_agent: Option<String>,
}

impl WebCaptureRequest {
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            mode: WebCaptureMode::Http,
            user_agent: None,
        }
    }

    #[must_use]
    pub fn with_mode(mut self, mode: WebCaptureMode) -> Self {
        self.mode = mode;
        self
    }

    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebLink {
    pub href: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebDocument {
    pub requested_url: String,
    pub final_url: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub content_type: Option<String>,
    pub status_code: u16,
    pub body: String,
    pub links: Vec<WebLink>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebCaptureResponse {
    pub capture: Observation<WebDocument>,
}

pub trait WebCaptureProvider: Send + Sync {
    fn name(&self) -> &str;
    fn capture(
        &self,
        request: &WebCaptureRequest,
        ctx: &CallContext,
    ) -> Result<WebCaptureResponse, String>;
}

#[derive(Debug, Clone, Default)]
pub struct StubWebCaptureProvider;

impl WebCaptureProvider for StubWebCaptureProvider {
    fn name(&self) -> &str {
        "stub_web_capture"
    }

    fn capture(
        &self,
        request: &WebCaptureRequest,
        _ctx: &CallContext,
    ) -> Result<WebCaptureResponse, String> {
        let url = request.url.trim();
        if url.is_empty() {
            return Err("Empty URL".to_string());
        }

        let hash_input = format!("{}:{:?}", url, request.mode);
        Ok(WebCaptureResponse {
            capture: Observation {
                observation_id: format!("obs:web:{}", content_hash(&hash_input)),
                request_hash: content_hash(&hash_input),
                vendor: "stub_web_capture".to_string(),
                model: "stub".to_string(),
                latency_ms: 5,
                cost_estimate: None,
                tokens: None,
                content: WebDocument {
                    requested_url: url.to_string(),
                    final_url: url.to_string(),
                    canonical_url: Some(url.to_string()),
                    title: Some("Stub Page".to_string()),
                    description: Some("Stub capture result".to_string()),
                    site_name: Some("Stub".to_string()),
                    content_type: Some("text/html".to_string()),
                    status_code: 200,
                    body: "<html><head><title>Stub Page</title></head><body><a href=\"https://example.com\">Example</a></body></html>".to_string(),
                    links: vec![WebLink {
                        href: "https://example.com".to_string(),
                        text: Some("Example".to_string()),
                    }],
                    metadata: serde_json::json!({
                        "provider": "stub_web_capture",
                        "mode": request.mode,
                    }),
                },
                raw_response: None,
            },
        })
    }
}

#[cfg(feature = "web")]
#[derive(Debug, Clone)]
pub struct HttpWebCaptureProvider {
    client: reqwest::blocking::Client,
}

#[cfg(feature = "web")]
impl HttpWebCaptureProvider {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|error| format!("failed to build http capture client: {error}"))?;
        Ok(Self { client })
    }
}

#[cfg(feature = "web")]
impl WebCaptureProvider for HttpWebCaptureProvider {
    fn name(&self) -> &str {
        "http_web_capture"
    }

    fn capture(
        &self,
        request: &WebCaptureRequest,
        _ctx: &CallContext,
    ) -> Result<WebCaptureResponse, String> {
        let requested_url = normalize_url(&request.url)?;
        let mut builder = self.client.get(requested_url.clone());
        if let Some(user_agent) = &request.user_agent {
            builder = builder.header(reqwest::header::USER_AGENT, user_agent);
        }

        let response = builder
            .send()
            .map_err(|error| format!("http capture request failed: {error}"))?;
        let status_code = response.status().as_u16();
        let final_url = response.url().to_string();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = response
            .text()
            .map_err(|error| format!("failed to read capture body: {error}"))?;

        let title = extract_title(&body);
        let description = extract_meta_content(&body, &["og:description", "description"]);
        let site_name = extract_meta_content(&body, &["og:site_name", "twitter:site"]);
        let canonical_url = extract_canonical_url(&body).or_else(|| Some(final_url.clone()));
        let links = extract_links(&body);
        let hash_input = format!("{}:{}", requested_url, final_url);

        Ok(WebCaptureResponse {
            capture: Observation {
                observation_id: format!("obs:web:{}", content_hash(&hash_input)),
                request_hash: content_hash(&hash_input),
                vendor: self.name().to_string(),
                model: "http".to_string(),
                latency_ms: 0,
                cost_estimate: None,
                tokens: None,
                content: WebDocument {
                    requested_url,
                    final_url,
                    canonical_url,
                    title,
                    description,
                    site_name,
                    content_type,
                    status_code,
                    body,
                    links,
                    metadata: serde_json::json!({
                        "mode": request.mode,
                    }),
                },
                raw_response: None,
            },
        })
    }
}

#[cfg(feature = "web")]
fn normalize_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Empty URL".to_string());
    }

    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    url::Url::parse(&candidate)
        .map(|url| url.to_string())
        .map_err(|error| format!("invalid url '{trimmed}': {error}"))
}

fn extract_title(html: &str) -> Option<String> {
    extract_between_ascii_case_insensitive(html, "<title>", "</title>")
}

fn extract_canonical_url(html: &str) -> Option<String> {
    extract_link_href(html, "canonical")
        .or_else(|| extract_meta_content(html, &["og:url", "twitter:url"]))
}

fn extract_meta_content(html: &str, names: &[&str]) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    for name in names {
        let name_lower = name.to_ascii_lowercase();
        for marker in [
            format!("property=\"{name_lower}\""),
            format!("property='{name_lower}'"),
            format!("name=\"{name_lower}\""),
            format!("name='{name_lower}'"),
        ] {
            if let Some(position) = lower.find(&marker) {
                let tail = &html[position
                    ..html[position..]
                        .find('>')
                        .map(|end| position + end)
                        .unwrap_or(html.len())];
                if let Some(content) = extract_attr_value(tail, "content") {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        return Some(html_unescape(trimmed));
                    }
                }
            }
        }
    }
    None
}

fn extract_link_href(html: &str, rel_name: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let rel_marker = format!("rel=\"{}\"", rel_name.to_ascii_lowercase());
    let rel_marker_alt = format!("rel='{}'", rel_name.to_ascii_lowercase());
    for marker in [rel_marker, rel_marker_alt] {
        if let Some(position) = lower.find(&marker) {
            let tail = &html[position
                ..html[position..]
                    .find('>')
                    .map(|end| position + end)
                    .unwrap_or(html.len())];
            if let Some(href) = extract_attr_value(tail, "href") {
                let trimmed = href.trim();
                if !trimmed.is_empty() {
                    return Some(html_unescape(trimmed));
                }
            }
        }
    }
    None
}

fn extract_links(html: &str) -> Vec<WebLink> {
    let mut links = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search_from = 0;

    while let Some(relative_start) = lower[search_from..].find("<a ") {
        let start = search_from + relative_start;
        let Some(tag_end_relative) = lower[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_relative;
        let tag = &html[start..=tag_end];
        if let Some(href) = extract_attr_value(tag, "href") {
            let href = html_unescape(href.trim());
            if !href.is_empty() {
                let close_tag = lower[tag_end + 1..]
                    .find("</a>")
                    .map(|offset| tag_end + 1 + offset)
                    .unwrap_or(tag_end + 1);
                let text = html[tag_end + 1..close_tag].trim();
                links.push(WebLink {
                    href,
                    text: if text.is_empty() {
                        None
                    } else {
                        Some(html_unescape(text))
                    },
                });
            }
        }
        search_from = tag_end + 1;
    }

    links
}

fn extract_between_ascii_case_insensitive(
    haystack: &str,
    start_marker: &str,
    end_marker: &str,
) -> Option<String> {
    let lower = haystack.to_ascii_lowercase();
    let start_marker_lower = start_marker.to_ascii_lowercase();
    let end_marker_lower = end_marker.to_ascii_lowercase();
    let start = lower.find(&start_marker_lower)? + start_marker.len();
    let end = lower[start..].find(&end_marker_lower)? + start;
    let value = haystack[start..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(html_unescape(value))
    }
}

fn extract_attr_value(tag: &str, attr_name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    for marker in [format!("{attr_name}=\""), format!("{attr_name}='")] {
        if let Some(position) = lower.find(&marker) {
            let quote = marker.chars().last()?;
            let start = position + marker.len();
            let end = tag[start..].find(quote)? + start;
            return Some(tag[start..end].to_string());
        }
    }
    None
}

fn html_unescape(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[cfg(test)]
mod tests {
    use super::{
        StubWebCaptureProvider, WebCaptureProvider, WebCaptureRequest, extract_canonical_url,
        extract_links, extract_meta_content, extract_title,
    };
    use crate::provenance::CallContext;

    #[test]
    fn stub_capture_returns_single_observation() {
        let provider = StubWebCaptureProvider;
        let response = provider
            .capture(
                &WebCaptureRequest::new("https://example.com"),
                &CallContext::default(),
            )
            .expect("capture should succeed");

        assert_eq!(response.capture.content.final_url, "https://example.com");
        assert_eq!(response.capture.content.links.len(), 1);
    }

    #[test]
    fn extracts_basic_metadata_from_html() {
        let html = r#"
            <html>
                <head>
                    <title>Example Title</title>
                    <meta property="og:description" content="Example description" />
                    <meta property="og:site_name" content="Example Site" />
                    <link rel="canonical" href="https://example.com/company" />
                </head>
                <body>
                    <a href="https://example.com/about">About</a>
                </body>
            </html>
        "#;

        assert_eq!(extract_title(html).as_deref(), Some("Example Title"));
        assert_eq!(
            extract_meta_content(html, &["og:description"]).as_deref(),
            Some("Example description")
        );
        assert_eq!(
            extract_meta_content(html, &["og:site_name"]).as_deref(),
            Some("Example Site")
        );
        assert_eq!(
            extract_canonical_url(html).as_deref(),
            Some("https://example.com/company")
        );
        assert_eq!(extract_links(html).len(), 1);
    }
}
