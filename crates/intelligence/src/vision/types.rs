// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Core types for vision understanding.

use serde::{Deserialize, Serialize};

/// Error type for vision operations.
#[derive(Debug, thiserror::Error)]
pub enum VisionError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Invalid image: {0}")]
    InvalidImage(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),
}

/// How the image is provided.
#[derive(Debug, Clone)]
pub enum VisionInput {
    /// Raw image bytes (PNG, JPEG, WebP, GIF).
    Bytes(Vec<u8>),
    /// Base64-encoded image with media type.
    Base64 { data: String, media_type: String },
    /// URL to fetch the image from.
    Url(String),
}

/// A request for vision analysis.
#[derive(Debug, Clone)]
pub struct VisionRequest {
    /// The image to analyze.
    pub input: VisionInput,
    /// Optional prompt to focus the analysis.
    /// e.g. "What products are on this shelf?" or "Describe any safety hazards."
    pub prompt: Option<String>,
    /// Maximum tokens for the response.
    pub max_tokens: u32,
    /// Whether to extract structured objects (slower, more detailed).
    pub extract_objects: bool,
    /// Whether to detect visible text (complements OCR).
    pub detect_text: bool,
}

impl VisionRequest {
    /// Create a request from raw image bytes.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            input: VisionInput::Bytes(bytes.into()),
            prompt: None,
            max_tokens: 1024,
            extract_objects: true,
            detect_text: true,
        }
    }

    /// Create a request from a URL.
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            input: VisionInput::Url(url.into()),
            prompt: None,
            max_tokens: 1024,
            extract_objects: true,
            detect_text: true,
        }
    }

    /// Create a request from base64-encoded data.
    pub fn from_base64(data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self {
            input: VisionInput::Base64 {
                data: data.into(),
                media_type: media_type.into(),
            },
            prompt: None,
            max_tokens: 1024,
            extract_objects: true,
            detect_text: true,
        }
    }

    /// Focus the analysis with a specific prompt.
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Set maximum response tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Disable object extraction (faster, scene-only).
    pub fn without_objects(mut self) -> Self {
        self.extract_objects = false;
        self
    }

    /// Disable text detection.
    pub fn without_text(mut self) -> Self {
        self.detect_text = false;
        self
    }
}

/// A detected object in the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    /// What the object is (e.g. "laptop", "person", "invoice").
    pub label: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
    /// Normalized bounding box (0.0 - 1.0), if available.
    pub bbox: Option<BoundingBox>,
    /// Additional attributes (e.g. "color: red", "brand: Apple").
    pub attributes: Vec<ObjectAttribute>,
}

/// A bounding box in normalized coordinates (0.0 - 1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// An attribute of a detected object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectAttribute {
    pub key: String,
    pub value: String,
}

/// Spatial relationship between objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialRelation {
    pub subject: String,
    pub relation: String,
    pub object: String,
}

/// Text detected in the image (complements OCR).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedText {
    pub text: String,
    pub location: Option<String>,
    pub confidence: f64,
}

/// The result of vision analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionDescription {
    /// Natural language description of the entire scene.
    pub scene: String,
    /// Detected objects with labels, confidence, and bounding boxes.
    pub objects: Vec<DetectedObject>,
    /// Spatial relationships between objects.
    pub relations: Vec<SpatialRelation>,
    /// Text visible in the image.
    pub text: Vec<DetectedText>,
    /// Which provider produced this description.
    pub provider: String,
    /// Which model was used.
    pub model: String,
    /// Raw response from the provider (for debugging).
    pub raw_response: Option<String>,
}

/// The canonical vision trait.
///
/// All backends implement this. Consumers inject `Arc<dyn VisionDescriber>`
/// and don't know which provider they're using.
pub trait VisionDescriber: Send + Sync {
    /// Analyze an image and return a structured description.
    fn describe(&self, request: &VisionRequest) -> Result<VisionDescription, VisionError>;

    /// The provider name (e.g. "anthropic", "gemini").
    fn provider_name(&self) -> &str;

    /// The model name (e.g. "claude-sonnet-4-6", "gemini-2.5-pro").
    fn model_name(&self) -> &str;
}
