// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Vision backends — LLM providers with image understanding.
//!
//! Each backend wraps an LLM provider's vision API with a structured
//! extraction prompt. The prompt asks for JSON output with scene description,
//! objects, spatial relations, and visible text.

use super::types::*;
use crate::secret::SecretString;

// ── System prompt for structured vision extraction ──

const VISION_SYSTEM_PROMPT: &str = r#"You are a precise visual analysis system. Analyze the provided image and respond with a JSON object containing:

{
  "scene": "Natural language description of the entire scene",
  "objects": [
    {
      "label": "object name",
      "confidence": 0.95,
      "bbox": {"x": 0.1, "y": 0.2, "width": 0.3, "height": 0.4},
      "attributes": [{"key": "color", "value": "red"}]
    }
  ],
  "relations": [
    {"subject": "laptop", "relation": "on top of", "object": "desk"}
  ],
  "text": [
    {"text": "visible text", "location": "top-left corner", "confidence": 0.9}
  ]
}

Rules:
- Bounding boxes use normalized coordinates (0.0 to 1.0). Omit if you cannot estimate position.
- Confidence is 0.0 to 1.0. Be honest — lower confidence for uncertain detections.
- Include ALL visible objects, not just prominent ones.
- For text detection, include any readable text in the image.
- If a specific analysis prompt is provided, focus on that but still report the full scene.
- Respond ONLY with valid JSON. No markdown, no explanation."#;

fn build_user_prompt(request: &VisionRequest) -> String {
    let mut parts = vec!["Analyze this image.".to_string()];

    if let Some(ref prompt) = request.prompt {
        parts.push(format!("Focus on: {prompt}"));
    }

    if !request.extract_objects {
        parts.push("Skip detailed object detection. Scene description only.".to_string());
    }

    if !request.detect_text {
        parts.push("Skip text detection.".to_string());
    }

    parts.join(" ")
}

fn parse_vision_response(
    raw: &str,
    provider: &str,
    model: &str,
) -> Result<VisionDescription, VisionError> {
    // Try to extract JSON from the response (handle markdown code blocks)
    let json_str = if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            &raw[start..=end]
        } else {
            raw
        }
    } else {
        raw
    };

    #[derive(serde::Deserialize)]
    struct RawResponse {
        scene: Option<String>,
        objects: Option<Vec<RawObject>>,
        relations: Option<Vec<SpatialRelation>>,
        text: Option<Vec<DetectedText>>,
    }

    #[derive(serde::Deserialize)]
    struct RawObject {
        label: String,
        confidence: Option<f64>,
        bbox: Option<BoundingBox>,
        attributes: Option<Vec<ObjectAttribute>>,
    }

    let parsed: RawResponse = serde_json::from_str(json_str)
        .map_err(|e| VisionError::Provider(format!("Failed to parse vision response: {e}")))?;

    Ok(VisionDescription {
        scene: parsed.scene.unwrap_or_default(),
        objects: parsed
            .objects
            .unwrap_or_default()
            .into_iter()
            .map(|o| DetectedObject {
                label: o.label,
                confidence: o.confidence.unwrap_or(0.5),
                bbox: o.bbox,
                attributes: o.attributes.unwrap_or_default(),
            })
            .collect(),
        relations: parsed.relations.unwrap_or_default(),
        text: parsed.text.unwrap_or_default(),
        provider: provider.to_string(),
        model: model.to_string(),
        raw_response: Some(raw.to_string()),
    })
}

fn encode_image_base64(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn guess_media_type(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        "image/jpeg"
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else {
        "image/png" // fallback
    }
}

// ── Anthropic Vision (Claude) ──

/// Vision backend using Anthropic's Claude with image input.
pub struct AnthropicVision {
    api_key: SecretString,
    model: String,
}

impl AnthropicVision {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            model: model.into(),
        }
    }

    pub fn from_env() -> Result<Self, VisionError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| VisionError::Authentication("ANTHROPIC_API_KEY not set".into()))?;
        Ok(Self::new(api_key, "claude-sonnet-4-6"))
    }

    fn call_api(&self, request: &VisionRequest) -> Result<String, VisionError> {
        let (b64_data, media_type) = match &request.input {
            VisionInput::Bytes(bytes) => (
                encode_image_base64(bytes),
                guess_media_type(bytes).to_string(),
            ),
            VisionInput::Base64 { data, media_type } => (data.clone(), media_type.clone()),
            VisionInput::Url(_url) => {
                return Err(VisionError::UnsupportedFormat(
                    "Anthropic does not support URL input — provide bytes or base64".into(),
                ));
            }
        };

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "system": VISION_SYSTEM_PROMPT,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": media_type,
                            "data": b64_data,
                        }
                    },
                    {
                        "type": "text",
                        "text": build_user_prompt(request),
                    }
                ]
            }]
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", self.api_key.expose())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| VisionError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().unwrap_or_default();
            if status == 429 {
                return Err(VisionError::RateLimited {
                    retry_after_ms: 5000,
                });
            }
            return Err(VisionError::Provider(format!("HTTP {status}: {text}")));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| VisionError::Provider(e.to_string()))?;

        json["content"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| VisionError::Provider("No text in response".into()))
    }
}

impl VisionDescriber for AnthropicVision {
    fn describe(&self, request: &VisionRequest) -> Result<VisionDescription, VisionError> {
        let raw = self.call_api(request)?;
        parse_vision_response(&raw, "anthropic", &self.model)
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}

// ── OpenAI Vision (GPT-4o) ──

/// Vision backend using OpenAI's GPT-4o with image input.
pub struct OpenAiVision {
    api_key: SecretString,
    model: String,
}

impl OpenAiVision {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            model: model.into(),
        }
    }

    pub fn from_env() -> Result<Self, VisionError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| VisionError::Authentication("OPENAI_API_KEY not set".into()))?;
        Ok(Self::new(api_key, "gpt-4o"))
    }

    fn call_api(&self, request: &VisionRequest) -> Result<String, VisionError> {
        let image_content = match &request.input {
            VisionInput::Bytes(bytes) => {
                let media_type = guess_media_type(bytes);
                let b64 = encode_image_base64(bytes);
                serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{media_type};base64,{b64}")
                    }
                })
            }
            VisionInput::Base64 { data, media_type } => {
                serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{media_type};base64,{data}")
                    }
                })
            }
            VisionInput::Url(url) => {
                serde_json::json!({
                    "type": "image_url",
                    "image_url": { "url": url }
                })
            }
        };

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": VISION_SYSTEM_PROMPT,
                },
                {
                    "role": "user",
                    "content": [
                        image_content,
                        {
                            "type": "text",
                            "text": build_user_prompt(request),
                        }
                    ]
                }
            ]
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| VisionError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().unwrap_or_default();
            if status == 429 {
                return Err(VisionError::RateLimited {
                    retry_after_ms: 5000,
                });
            }
            return Err(VisionError::Provider(format!("HTTP {status}: {text}")));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| VisionError::Provider(e.to_string()))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| VisionError::Provider("No content in response".into()))
    }
}

impl VisionDescriber for OpenAiVision {
    fn describe(&self, request: &VisionRequest) -> Result<VisionDescription, VisionError> {
        let raw = self.call_api(request)?;
        parse_vision_response(&raw, "openai", &self.model)
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}

// ── Gemini Vision ──

/// Vision backend using Google's Gemini with image input.
pub struct GeminiVision {
    api_key: SecretString,
    model: String,
}

impl GeminiVision {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            model: model.into(),
        }
    }

    pub fn from_env() -> Result<Self, VisionError> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| VisionError::Authentication("GEMINI_API_KEY not set".into()))?;
        Ok(Self::new(api_key, "gemini-2.5-pro"))
    }

    fn call_api(&self, request: &VisionRequest) -> Result<String, VisionError> {
        let (b64_data, media_type) = match &request.input {
            VisionInput::Bytes(bytes) => (
                encode_image_base64(bytes),
                guess_media_type(bytes).to_string(),
            ),
            VisionInput::Base64 { data, media_type } => (data.clone(), media_type.clone()),
            VisionInput::Url(_) => {
                return Err(VisionError::UnsupportedFormat(
                    "Gemini inline: provide bytes or base64".into(),
                ));
            }
        };

        let body = serde_json::json!({
            "system_instruction": {
                "parts": [{"text": VISION_SYSTEM_PROMPT}]
            },
            "contents": [{
                "parts": [
                    {
                        "inline_data": {
                            "mime_type": media_type,
                            "data": b64_data,
                        }
                    },
                    {
                        "text": build_user_prompt(request),
                    }
                ]
            }],
            "generationConfig": {
                "maxOutputTokens": request.max_tokens,
            }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model,
            self.api_key.expose()
        );

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| VisionError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().unwrap_or_default();
            if status == 429 {
                return Err(VisionError::RateLimited {
                    retry_after_ms: 5000,
                });
            }
            return Err(VisionError::Provider(format!("HTTP {status}: {text}")));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| VisionError::Provider(e.to_string()))?;

        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| VisionError::Provider("No text in response".into()))
    }
}

impl VisionDescriber for GeminiVision {
    fn describe(&self, request: &VisionRequest) -> Result<VisionDescription, VisionError> {
        let raw = self.call_api(request)?;
        parse_vision_response(&raw, "gemini", &self.model)
    }

    fn provider_name(&self) -> &str {
        "gemini"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}

// ── Mistral Vision (Pixtral) ──

/// Vision backend using Mistral's Pixtral with image input.
pub struct MistralVision {
    api_key: SecretString,
    model: String,
}

impl MistralVision {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::new(api_key),
            model: model.into(),
        }
    }

    pub fn from_env() -> Result<Self, VisionError> {
        let api_key = std::env::var("MISTRAL_API_KEY")
            .map_err(|_| VisionError::Authentication("MISTRAL_API_KEY not set".into()))?;
        Ok(Self::new(api_key, "pixtral-large-latest"))
    }

    fn call_api(&self, request: &VisionRequest) -> Result<String, VisionError> {
        let image_content = match &request.input {
            VisionInput::Bytes(bytes) => {
                let media_type = guess_media_type(bytes);
                let b64 = encode_image_base64(bytes);
                serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{media_type};base64,{b64}")
                    }
                })
            }
            VisionInput::Base64 { data, media_type } => {
                serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{media_type};base64,{data}")
                    }
                })
            }
            VisionInput::Url(url) => {
                serde_json::json!({
                    "type": "image_url",
                    "image_url": { "url": url }
                })
            }
        };

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": VISION_SYSTEM_PROMPT,
                },
                {
                    "role": "user",
                    "content": [
                        image_content,
                        {
                            "type": "text",
                            "text": build_user_prompt(request),
                        }
                    ]
                }
            ]
        });

        let client = reqwest::blocking::Client::new();
        let response = client
            .post("https://api.mistral.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| VisionError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().unwrap_or_default();
            if status == 429 {
                return Err(VisionError::RateLimited {
                    retry_after_ms: 5000,
                });
            }
            return Err(VisionError::Provider(format!("HTTP {status}: {text}")));
        }

        let json: serde_json::Value = response
            .json()
            .map_err(|e| VisionError::Provider(e.to_string()))?;

        json["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| VisionError::Provider("No content in response".into()))
    }
}

impl VisionDescriber for MistralVision {
    fn describe(&self, request: &VisionRequest) -> Result<VisionDescription, VisionError> {
        let raw = self.call_api(request)?;
        parse_vision_response(&raw, "mistral", &self.model)
    }

    fn provider_name(&self) -> &str {
        "mistral"
    }
    fn model_name(&self) -> &str {
        &self.model
    }
}
