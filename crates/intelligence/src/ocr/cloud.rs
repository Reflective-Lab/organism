// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! OCR / Document AI providers.
//!
//! This module provides integration with OCR models for document understanding,
//! text extraction, and structured content parsing from PDFs, scans, and images.
//!
//! # Available Providers
//!
//! - [`MistralOcrProvider`] - Mistral OCR 3 (GDPR-compliant, EU)
//! - [`DeepSeekOcrProvider`] - `DeepSeek` OCR 2 (Visual Causal Flow)
//! - [`LightOnOcrProvider`] - LightOnOCR-2-1B (Efficient, open-source)
//!
//! # Example
//!
//! ```ignore
//! use organism_intelligence::ocr::{OcrProvider, MistralOcrProvider, OcrRequest};
//!
//! let provider = MistralOcrProvider::from_env()?;
//! let request = OcrRequest::from_pdf_bytes(pdf_bytes);
//! let result = provider.extract(&request)?;
//!
//! println!("Extracted text: {}", result.text);
//! for table in result.tables {
//!     println!("Table: {:?}", table);
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Error type for OCR operations.
#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    /// Network/HTTP error.
    #[error("Network error: {0}")]
    Network(String),

    /// API authentication error.
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// API response parsing error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Invalid input (unsupported format, etc.).
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// General API error.
    #[error("API error: {0}")]
    Api(String),
}

/// Input type for OCR processing.
#[derive(Debug, Clone)]
pub enum OcrInput {
    /// PDF document as bytes.
    PdfBytes(Vec<u8>),
    /// Image as bytes (PNG, JPEG, etc.).
    ImageBytes(Vec<u8>),
    /// URL to a document or image.
    Url(String),
    /// Base64-encoded document or image.
    Base64(String),
}

/// OCR extraction request.
#[derive(Debug, Clone)]
pub struct OcrRequest {
    /// Input document or image.
    pub input: OcrInput,
    /// Output format preference.
    pub output_format: OcrOutputFormat,
    /// Language hints (ISO 639-1 codes).
    pub languages: Vec<String>,
    /// Whether to extract tables.
    pub extract_tables: bool,
    /// Whether to extract images/figures.
    pub extract_images: bool,
    /// Page range (for multi-page documents).
    pub page_range: Option<(usize, usize)>,
}

impl OcrRequest {
    /// Creates a request from PDF bytes.
    #[must_use]
    pub fn from_pdf_bytes(bytes: Vec<u8>) -> Self {
        Self {
            input: OcrInput::PdfBytes(bytes),
            output_format: OcrOutputFormat::Markdown,
            languages: vec![],
            extract_tables: true,
            extract_images: false,
            page_range: None,
        }
    }

    /// Creates a request from image bytes.
    #[must_use]
    pub fn from_image_bytes(bytes: Vec<u8>) -> Self {
        Self {
            input: OcrInput::ImageBytes(bytes),
            output_format: OcrOutputFormat::Markdown,
            languages: vec![],
            extract_tables: true,
            extract_images: false,
            page_range: None,
        }
    }

    /// Creates a request from a URL.
    #[must_use]
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            input: OcrInput::Url(url.into()),
            output_format: OcrOutputFormat::Markdown,
            languages: vec![],
            extract_tables: true,
            extract_images: false,
            page_range: None,
        }
    }

    /// Sets the output format.
    #[must_use]
    pub fn with_output_format(mut self, format: OcrOutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Adds language hints.
    #[must_use]
    pub fn with_languages(mut self, languages: Vec<String>) -> Self {
        self.languages = languages;
        self
    }

    /// Sets whether to extract tables.
    #[must_use]
    pub fn with_extract_tables(mut self, extract: bool) -> Self {
        self.extract_tables = extract;
        self
    }

    /// Sets whether to extract images.
    #[must_use]
    pub fn with_extract_images(mut self, extract: bool) -> Self {
        self.extract_images = extract;
        self
    }

    /// Sets the page range for multi-page documents.
    #[must_use]
    pub fn with_page_range(mut self, start: usize, end: usize) -> Self {
        self.page_range = Some((start, end));
        self
    }
}

/// Output format for OCR results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OcrOutputFormat {
    /// Plain text.
    Text,
    /// Markdown with structure preserved.
    #[default]
    Markdown,
    /// HTML with table reconstruction.
    Html,
    /// JSON with structured data.
    Json,
}

/// A detected table in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTable {
    /// Page number (0-indexed).
    pub page: usize,
    /// Table as HTML or markdown.
    pub content: String,
    /// Bounding box (x, y, width, height) if available.
    pub bbox: Option<(f64, f64, f64, f64)>,
}

/// A detected image/figure in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrImage {
    /// Page number (0-indexed).
    pub page: usize,
    /// Image description or alt text.
    pub description: Option<String>,
    /// Bounding box (x, y, width, height).
    pub bbox: Option<(f64, f64, f64, f64)>,
    /// Base64-encoded image data (if extracted).
    pub data: Option<String>,
}

/// Provenance information for OCR results.
///
/// Captures everything needed for reproducibility and tracing:
/// - Tool version and configuration
/// - Input/output hashes for trace links
/// - Preprocessing parameters applied
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OcrProvenance {
    /// Provider/tool name (e.g., "tesseract", "mistral-ocr", "deepseek-ocr").
    pub provider: String,
    /// Model or engine version (e.g., "5.3.0", "mistral-ocr-2512").
    pub version: String,
    /// Language pack(s) used (e.g., ["eng", "deu"]).
    pub languages: Vec<String>,
    /// Preprocessing parameters applied.
    pub preprocessing: OcrPreprocessing,
    /// SHA-256 hash of input bytes (for trace links).
    pub input_hash: Option<String>,
    /// SHA-256 hash of output text (for trace links).
    pub output_hash: Option<String>,
    /// Additional metadata (tool-specific).
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Preprocessing parameters applied before OCR.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OcrPreprocessing {
    /// DPI used for rendering (for PDFs).
    pub dpi: Option<u32>,
    /// Whether binarization was applied.
    pub binarized: bool,
    /// Whether deskewing was applied.
    pub deskewed: bool,
    /// Whether noise removal was applied.
    pub denoised: bool,
    /// Page segmentation mode (Tesseract-specific).
    pub psm: Option<u32>,
    /// OCR engine mode (Tesseract-specific).
    pub oem: Option<u32>,
}

/// Confidence summary for OCR results.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OcrConfidence {
    /// Overall mean confidence (0.0-1.0).
    pub mean: f64,
    /// Minimum word confidence.
    pub min: f64,
    /// Maximum word confidence.
    pub max: f64,
    /// Standard deviation of confidence scores.
    pub std_dev: Option<f64>,
    /// Number of words with confidence below threshold.
    pub low_confidence_words: usize,
    /// Threshold used for low confidence (default 0.6).
    pub threshold: f64,
}

/// A word or text span with position and confidence.
///
/// For Tesseract, this comes from TSV or hOCR output.
/// Useful for validation: you can check where each word came from
/// and flag low-confidence regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrSpan {
    /// The text content of this span.
    pub text: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
    /// Page number (0-indexed).
    pub page: usize,
    /// Bounding box: (x, y, width, height) in pixels.
    pub bbox: Option<(i32, i32, i32, i32)>,
    /// Block number (page segmentation unit).
    pub block_num: Option<i32>,
    /// Paragraph number within block.
    pub par_num: Option<i32>,
    /// Line number within paragraph.
    pub line_num: Option<i32>,
    /// Word number within line.
    pub word_num: Option<i32>,
}

impl OcrSpan {
    /// Creates a new span with text and confidence.
    #[must_use]
    pub fn new(text: impl Into<String>, confidence: f64) -> Self {
        Self {
            text: text.into(),
            confidence,
            page: 0,
            bbox: None,
            block_num: None,
            par_num: None,
            line_num: None,
            word_num: None,
        }
    }

    /// Sets the bounding box.
    #[must_use]
    pub fn with_bbox(mut self, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.bbox = Some((x, y, w, h));
        self
    }

    /// Sets the page number.
    #[must_use]
    pub fn with_page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Checks if this span has low confidence (below threshold).
    #[must_use]
    pub fn is_low_confidence(&self, threshold: f64) -> bool {
        self.confidence < threshold
    }
}

/// Tesseract-specific output format.
///
/// Controls what kind of output to request from Tesseract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TesseractOutputFormat {
    /// Plain text (default).
    #[default]
    Text,
    /// TSV with word-level confidence and bounding boxes.
    /// Columns: level, `page_num`, `block_num`, `par_num`, `line_num`, `word_num`,
    ///          left, top, width, height, conf, text
    Tsv,
    /// hOCR HTML format with bounding boxes.
    /// Useful for downstream table/layout analysis.
    Hocr,
    /// ALTO XML format (common in libraries/archives).
    Alto,
}

/// OCR extraction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    /// Extracted text content.
    pub text: String,
    /// Number of pages processed.
    pub pages: usize,
    /// Word/text spans with positions and confidence.
    /// Populated when using TSV or hOCR output format (Tesseract).
    /// Useful for validation: check where each word came from.
    #[serde(default)]
    pub spans: Vec<OcrSpan>,
    /// Detected tables.
    pub tables: Vec<OcrTable>,
    /// Detected images/figures.
    pub images: Vec<OcrImage>,
    /// Confidence summary (per-word statistics).
    pub confidence: Option<OcrConfidence>,
    /// Processing time in milliseconds.
    pub processing_time_ms: Option<u64>,
    /// Provenance for reproducibility and tracing.
    pub provenance: OcrProvenance,
}

/// Trait for OCR providers.
pub trait OcrProvider: Send + Sync {
    /// Returns the provider name.
    fn name(&self) -> &'static str;

    /// Returns the model being used.
    fn model(&self) -> &str;

    /// Extracts text and structure from a document.
    ///
    /// # Errors
    ///
    /// Returns error if extraction fails.
    fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError>;
}

// =============================================================================
// Mistral OCR Provider
// =============================================================================

/// Mistral OCR 3 provider.
///
/// Mistral OCR 3 is designed for document AI at scale, handling forms, invoices,
/// complex tables, handwriting, and low-quality scans. It outputs structured
/// text/HTML suitable for RAG and agent workflows.
///
/// # Features
/// - 74% win rate over OCR 2 on forms, handwriting, tables
/// - Markdown output with HTML table reconstruction
/// - GDPR-compliant (France)
/// - $2 per 1000 pages ($1 with batch API)
///
/// # Example
///
/// ```ignore
/// use converge_provider::ocr::{MistralOcrProvider, OcrRequest};
///
/// let provider = MistralOcrProvider::from_env()?;
/// let result = provider.extract(&OcrRequest::from_pdf_bytes(pdf_bytes))?;
/// ```
pub struct MistralOcrProvider {
    api_key: crate::secret::SecretString,
    model: String,
    base_url: String,
    client: reqwest::blocking::Client,
}

impl MistralOcrProvider {
    /// Creates a new Mistral OCR provider.
    #[must_use]
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: crate::secret::SecretString::new(api_key),
            model: model.into(),
            base_url: "https://api.mistral.ai/v1".to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Creates a provider using the `MISTRAL_API_KEY` environment variable.
    ///
    /// Uses `mistral-ocr-latest` as the default model.
    ///
    /// # Errors
    ///
    /// Returns error if the environment variable is not set.
    pub fn from_env() -> Result<Self, OcrError> {
        let api_key = std::env::var("MISTRAL_API_KEY").map_err(|_| {
            OcrError::Auth("MISTRAL_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key, "mistral-ocr-latest"))
    }

    /// Creates a provider with a specific model.
    ///
    /// # Errors
    ///
    /// Returns error if the environment variable is not set.
    pub fn from_env_with_model(model: impl Into<String>) -> Result<Self, OcrError> {
        let api_key = std::env::var("MISTRAL_API_KEY").map_err(|_| {
            OcrError::Auth("MISTRAL_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key, model))
    }

    /// Uses a custom base URL.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl OcrProvider for MistralOcrProvider {
    fn name(&self) -> &'static str {
        "mistral-ocr"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError> {
        // Build the request body based on input type
        let document = match &request.input {
            OcrInput::PdfBytes(bytes) => {
                serde_json::json!({
                    "type": "document_url",
                    "document_url": format!("data:application/pdf;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes))
                })
            }
            OcrInput::ImageBytes(bytes) => {
                serde_json::json!({
                    "type": "image_url",
                    "image_url": format!("data:image/png;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes))
                })
            }
            OcrInput::Url(url) => {
                if std::path::Path::new(url)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
                {
                    serde_json::json!({
                        "type": "document_url",
                        "document_url": url
                    })
                } else {
                    serde_json::json!({
                        "type": "image_url",
                        "image_url": url
                    })
                }
            }
            OcrInput::Base64(data) => {
                serde_json::json!({
                    "type": "document_url",
                    "document_url": format!("data:application/pdf;base64,{}", data)
                })
            }
        };

        let body = serde_json::json!({
            "model": self.model,
            "document": document,
            "include_image_base64": request.extract_images
        });

        let response = self
            .client
            .post(format!("{}/ocr", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| OcrError::Network(format!("Request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().unwrap_or_default();
            return match status.as_u16() {
                401 | 403 => Err(OcrError::Auth(format!(
                    "Authentication failed: {error_text}"
                ))),
                429 => Err(OcrError::RateLimit("Rate limit exceeded".to_string())),
                _ => Err(OcrError::Api(format!("API error ({status}): {error_text}"))),
            };
        }

        let api_response: MistralOcrResponse = response
            .json()
            .map_err(|e| OcrError::Parse(format!("Failed to parse response: {e}")))?;

        // Convert to our result format
        let mut tables = vec![];
        let mut images = vec![];
        let mut text = String::new();

        for (page_idx, page) in api_response.pages.iter().enumerate() {
            text.push_str(&page.markdown);
            text.push_str("\n\n");

            // Extract tables from markdown (simplified)
            // In practice, Mistral returns tables as HTML within markdown
            if page.markdown.contains("<table") {
                tables.push(OcrTable {
                    page: page_idx,
                    content: page.markdown.clone(),
                    bbox: None,
                });
            }

            // Extract images if present
            for img in &page.images {
                images.push(OcrImage {
                    page: page_idx,
                    description: None,
                    bbox: None,
                    data: img.image_base64.clone(),
                });
            }
        }

        Ok(OcrResult {
            text: text.trim().to_string(),
            pages: api_response.pages.len(),
            spans: vec![], // Mistral OCR doesn't provide word-level spans
            tables,
            images,
            confidence: None,
            processing_time_ms: None,
            provenance: OcrProvenance {
                provider: "mistral-ocr".to_string(),
                version: self.model.clone(),
                languages: request.languages.clone(),
                preprocessing: OcrPreprocessing::default(),
                input_hash: None,  // TODO: compute from input
                output_hash: None, // TODO: compute from output
                metadata: std::collections::HashMap::new(),
            },
        })
    }
}

#[derive(Debug, Deserialize)]
struct MistralOcrResponse {
    pages: Vec<MistralOcrPage>,
}

#[derive(Debug, Deserialize)]
struct MistralOcrPage {
    markdown: String,
    #[serde(default)]
    images: Vec<MistralOcrImage>,
}

#[derive(Debug, Deserialize)]
struct MistralOcrImage {
    #[serde(default)]
    image_base64: Option<String>,
}

// =============================================================================
// DeepSeek OCR Provider
// =============================================================================

/// `DeepSeek` OCR 2 provider.
///
/// `DeepSeek` OCR 2 is a 3B-parameter vision-language model with the `DeepEncoder` V2
/// architecture featuring Visual Causal Flow for human-like reading order.
///
/// # Features
/// - SOTA on document understanding benchmarks
/// - Human-like visual reading order
/// - Semantic visual reasoning
/// - 16x token compression
///
/// # Example
///
/// ```ignore
/// use converge_provider::ocr::{DeepSeekOcrProvider, OcrRequest};
///
/// let provider = DeepSeekOcrProvider::from_env()?;
/// let result = provider.extract(&OcrRequest::from_image_bytes(image_bytes))?;
/// ```
pub struct DeepSeekOcrProvider {
    api_key: crate::secret::SecretString,
    model: String,
    base_url: String,
    client: reqwest::blocking::Client,
}

impl DeepSeekOcrProvider {
    /// Creates a new `DeepSeek` OCR provider.
    #[must_use]
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: crate::secret::SecretString::new(api_key),
            model: model.into(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Creates a provider using the `DEEPSEEK_API_KEY` environment variable.
    ///
    /// Uses `deepseek-ocr-2` as the default model.
    ///
    /// # Errors
    ///
    /// Returns error if the environment variable is not set.
    pub fn from_env() -> Result<Self, OcrError> {
        let api_key = std::env::var("DEEPSEEK_API_KEY").map_err(|_| {
            OcrError::Auth("DEEPSEEK_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key, "deepseek-ocr-2"))
    }

    /// Uses a custom base URL.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl OcrProvider for DeepSeekOcrProvider {
    fn name(&self) -> &'static str {
        "deepseek-ocr"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError> {
        // DeepSeek OCR uses a chat-like API with vision capabilities
        let image_content = match &request.input {
            OcrInput::ImageBytes(bytes) => {
                format!(
                    "data:image/png;base64,{}",
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
                )
            }
            OcrInput::PdfBytes(bytes) => {
                // DeepSeek OCR expects images; for PDF, we'd need to convert pages
                // For now, treat as base64 document
                format!(
                    "data:application/pdf;base64,{}",
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes)
                )
            }
            OcrInput::Url(url) => url.clone(),
            OcrInput::Base64(data) => format!("data:image/png;base64,{data}"),
        };

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": image_content
                        }
                    },
                    {
                        "type": "text",
                        "text": "Extract all text from this document, preserving structure, tables, and reading order. Output in markdown format."
                    }
                ]
            }],
            "max_tokens": 8192
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .map_err(|e| OcrError::Network(format!("Request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().unwrap_or_default();
            return match status.as_u16() {
                401 | 403 => Err(OcrError::Auth(format!(
                    "Authentication failed: {error_text}"
                ))),
                429 => Err(OcrError::RateLimit("Rate limit exceeded".to_string())),
                _ => Err(OcrError::Api(format!("API error ({status}): {error_text}"))),
            };
        }

        let api_response: DeepSeekOcrResponse = response
            .json()
            .map_err(|e| OcrError::Parse(format!("Failed to parse response: {e}")))?;

        let text = api_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(OcrResult {
            text,
            pages: 1,      // DeepSeek processes one image at a time
            spans: vec![], // DeepSeek OCR doesn't provide word-level spans
            tables: vec![],
            images: vec![],
            confidence: None,
            processing_time_ms: None,
            provenance: OcrProvenance {
                provider: "deepseek-ocr".to_string(),
                version: self.model.clone(),
                languages: request.languages.clone(),
                preprocessing: OcrPreprocessing::default(),
                input_hash: None,
                output_hash: None,
                metadata: std::collections::HashMap::new(),
            },
        })
    }
}

#[derive(Debug, Deserialize)]
struct DeepSeekOcrResponse {
    choices: Vec<DeepSeekOcrChoice>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekOcrChoice {
    message: DeepSeekOcrMessage,
}

#[derive(Debug, Deserialize)]
struct DeepSeekOcrMessage {
    content: Option<String>,
}

// =============================================================================
// LightOn OCR Provider
// =============================================================================

/// LightOnOCR-2-1B provider.
///
/// LightOnOCR-2 is an efficient 1B-parameter vision-language model that achieves
/// SOTA on OlmOCR-Bench while being 9x smaller than competitors.
///
/// # Features
/// - 1B parameters, 9x smaller than competitors
/// - 5.71 pages/s on H100 (~493k pages/day)
/// - <$0.01 per 1000 pages
/// - Apache 2.0 license, open weights
/// - GDPR-compliant (France)
///
/// # Example
///
/// ```ignore
/// use converge_provider::ocr::{LightOnOcrProvider, OcrRequest};
///
/// let provider = LightOnOcrProvider::from_env()?;
/// let result = provider.extract(&OcrRequest::from_pdf_bytes(pdf_bytes))?;
/// ```
pub struct LightOnOcrProvider {
    api_key: crate::secret::SecretString,
    model: String,
    base_url: String,
    client: reqwest::blocking::Client,
}

impl LightOnOcrProvider {
    /// Creates a new `LightOn` OCR provider.
    #[must_use]
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: crate::secret::SecretString::new(api_key),
            model: model.into(),
            base_url: "https://api-inference.huggingface.co/models".to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Creates a provider using the `HUGGINGFACE_API_KEY` environment variable.
    ///
    /// Uses `lightonai/LightOnOCR-2-1B` as the default model.
    ///
    /// # Errors
    ///
    /// Returns error if the environment variable is not set.
    pub fn from_env() -> Result<Self, OcrError> {
        let api_key = std::env::var("HUGGINGFACE_API_KEY").map_err(|_| {
            OcrError::Auth("HUGGINGFACE_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key, "lightonai/LightOnOCR-2-1B"))
    }

    /// Creates a provider with the bbox variant for figure extraction.
    ///
    /// # Errors
    ///
    /// Returns error if the environment variable is not set.
    pub fn from_env_with_bbox() -> Result<Self, OcrError> {
        let api_key = std::env::var("HUGGINGFACE_API_KEY").map_err(|_| {
            OcrError::Auth("HUGGINGFACE_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key, "lightonai/LightOnOCR-2-1B-bbox"))
    }

    /// Uses a custom base URL.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

impl OcrProvider for LightOnOcrProvider {
    fn name(&self) -> &'static str {
        "lighton-ocr"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError> {
        // LightOnOCR uses HuggingFace Inference API
        let image_bytes = match &request.input {
            OcrInput::ImageBytes(bytes) => bytes.clone(),
            OcrInput::PdfBytes(_) => {
                return Err(OcrError::InvalidInput(
                    "LightOnOCR requires image input. Convert PDF pages to images first."
                        .to_string(),
                ));
            }
            OcrInput::Url(url) => {
                // Fetch the image
                let response = self
                    .client
                    .get(url)
                    .send()
                    .map_err(|e| OcrError::Network(format!("Failed to fetch image: {e}")))?;
                response
                    .bytes()
                    .map_err(|e| OcrError::Network(format!("Failed to read image: {e}")))?
                    .to_vec()
            }
            OcrInput::Base64(data) => {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
                    .map_err(|e| OcrError::Parse(format!("Invalid base64: {e}")))?
            }
        };

        let response = self
            .client
            .post(format!("{}/{}", self.base_url, self.model))
            .header("Authorization", format!("Bearer {}", self.api_key.expose()))
            .header("Content-Type", "application/octet-stream")
            .body(image_bytes)
            .send()
            .map_err(|e| OcrError::Network(format!("Request failed: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().unwrap_or_default();
            return match status.as_u16() {
                401 | 403 => Err(OcrError::Auth(format!(
                    "Authentication failed: {error_text}"
                ))),
                429 => Err(OcrError::RateLimit("Rate limit exceeded".to_string())),
                503 => Err(OcrError::Api("Model is loading, please retry".to_string())),
                _ => Err(OcrError::Api(format!("API error ({status}): {error_text}"))),
            };
        }

        // LightOnOCR returns the extracted text directly
        let text = response
            .text()
            .map_err(|e| OcrError::Parse(format!("Failed to read response: {e}")))?;

        Ok(OcrResult {
            text,
            pages: 1,
            spans: vec![], // LightOnOCR doesn't provide word-level spans via HF API
            tables: vec![],
            images: vec![],
            confidence: None,
            processing_time_ms: None,
            provenance: OcrProvenance {
                provider: "lighton-ocr".to_string(),
                version: self.model.clone(),
                languages: request.languages.clone(),
                preprocessing: OcrPreprocessing::default(),
                input_hash: None,
                output_hash: None,
                metadata: std::collections::HashMap::new(),
            },
        })
    }
}

// =============================================================================
// Tesseract OCR Provider (Feature-gated, Local)
// =============================================================================
//
// Tesseract is the "boring, reliable" OCR workhorse: classic OCR engine,
// open source, runs fully locally, and easy to wrap in a Converge-style
// Provider boundary.
//
// =============================================================================
// WHAT TESSERACT IS
// =============================================================================
//
// - An OCR engine originally from HP, maintained under open source.
// - Takes images (PNG/JPG/TIFF etc) and outputs text, optionally with layout.
// - Can run with different language packs (English, Swedish, etc).
// - NOT a "big neural multimodal model" - it's a tool-like OCR system with
//   modern recognition components but still very deterministic.
//
// =============================================================================
// WHERE IT SHINES
// =============================================================================
//
// - Clean scans, printed documents, forms, invoices, manuals, receipts
// - High-contrast screenshots
// - Simple page layouts
// - Deterministic runs: same input + same version + same settings = same output
//
// =============================================================================
// WHERE IT STRUGGLES
// =============================================================================
//
// - Handwriting (varies, usually weak vs modern DL OCR)
// - Low-quality photos (blur, perspective, glare)
// - Complex layouts with tables/columns (unless you guide it well with PSM)
// - Mixed languages without explicit config
//
// If your primary use case is handwriting, camera photos with glare, or dense
// multi-column PDFs with complex tables, consider a DL-based OCR instead.
//
// =============================================================================
// OUTPUT FORMATS
// =============================================================================
//
// Tesseract can produce:
// - Plain text: Just the extracted text
// - TSV: Word-level info with confidence and bounding boxes
// - hOCR: HTML-like format with bounding boxes (useful for validation)
// - ALTO XML: Common in libraries/archives
//
// For Converge, hOCR/TSV is useful because you can validate "evidence":
// - Bounding boxes (where each word came from)
// - Per-word confidence
// - Page segmentation decisions
//
// =============================================================================
// KEY KNOBS
// =============================================================================
//
// 1. Page Segmentation Mode (PSM) - THE BIGGEST PRACTICAL LEVER
//    Tells Tesseract what kind of page it's looking at:
//    - 0 = OSD only (orientation and script detection)
//    - 1 = Automatic page segmentation with OSD
//    - 3 = Fully automatic page segmentation (default)
//    - 4 = Single column of variable sizes
//    - 6 = Uniform block of text
//    - 7 = Single text line
//    - 8 = Single word
//    - 11 = Sparse text
//    If you set the wrong mode, accuracy tanks.
//
// 2. OCR Engine Mode (OEM)
//    Chooses which internal engine strategy to use:
//    - 0 = Legacy engine only
//    - 1 = Neural nets LSTM engine only
//    - 2 = Legacy + LSTM engines
//    - 3 = Default (auto-select best available)
//    Defaults are usually fine, but pin for reproducibility.
//
// 3. Language Packs
//    Set -l eng / -l swe etc. DON'T leave language detection implicit.
//
// 4. Preprocessing
//    Tesseract is EXTREMELY sensitive to:
//    - Resolution (DPI) - 300 DPI is typical minimum
//    - Binarization (thresholding)
//    - Denoise
//    - Deskew
//    - Contrast normalization
//
//    This is where "Rust purity" can shine: do deterministic preprocessing
//    in Rust (image crate) and then pass a cleaned image to Tesseract.
//
// =============================================================================
// CONVERGE-STYLE INTEGRATION PATTERN
// =============================================================================
//
// Treat OCR as a provider that returns a PROPOSAL, never truth.
//
// Shape:
//   DocumentBytes → ProposedTextExtraction → Validators → Facts/StructuredFields
//
// Provider output (recommended):
//   - text: extracted text
//   - spans: optional words/lines with bounding boxes (from TSV/hOCR)
//   - confidence: summary stats (mean, min, histogram)
//   - tool_provenance:
//       - engine = "tesseract"
//       - tesseract_version
//       - lang
//       - psm, oem
//       - preprocess_pipeline_hash
//   - trace_link:
//       - input hash (bytes)
//       - output hash
//       - settings hash
//
// Validators (examples):
//   - min_confidence >= 0.75 else STOP or WARN
//   - required_fields_present (invoice number/date/amount)
//   - layout sanity (if table expected, require hOCR structure)
//   - PII redaction gate before storage
//
// =============================================================================
// PACKAGING AND DEPLOYMENT
// =============================================================================
//
// Tesseract is a native dependency. Manage cleanly:
//
// Best practice for "one binary experience":
//   - Ship your Rust binary
//   - Vendor/bundle Tesseract in installer (or provide "cz doctor" check)
//   - Pin versions for reproducibility
//
// On macOS: Most people install via Homebrew, but for deterministic
// environments, package with your app or use Nix.
//
// =============================================================================
// ARCHITECTURE (Rust-first compromise)
// =============================================================================
//
// Tesseract integration follows the "Rust-first compromise" pattern:
// - Pure Converge architecture (providers, traces, gates, promotion)
// - OCR runs locally with no cloud data exposure
// - Accepts native dependency (tesseract + leptonica)
//
// Integration options (in order of preference):
// 1. Sidecar binary: invoke `tesseract` CLI via std::process::Command
// 2. FFI binding: link against libtesseract (more complex, faster)
// 3. System dependency: require tesseract installed (brew, apt, nix)
//
// The provider returns:
// - Extracted text
// - Confidence summary (per-word statistics)
// - Provenance: tool version, language pack, preprocessing params
// - Trace link hashes of input bytes and output
//
// Determinism: Stable for same input image + same Tesseract version.
//
// When to use:
// - Scanned PDFs, clean prints, forms, invoices, receipts
// - "Extract text so downstream validators can reason"
// - GDPR/data sovereignty requirements (no cloud exposure)
//
// Future: Can be swapped with Burn/candle-based OCR model without
// changing the core contracts (OcrProvider trait).
//
// =============================================================================

/// Configuration for Tesseract OCR provider.
///
/// # Feature Gate
///
/// This provider requires the `tesseract` feature:
/// ```toml
/// [dependencies]
/// converge-provider = { version = "0.2", features = ["tesseract"] }
/// ```
///
/// # System Requirements
///
/// Tesseract must be installed on the system:
/// - macOS: `brew install tesseract tesseract-lang`
/// - Ubuntu: `apt install tesseract-ocr tesseract-ocr-eng`
/// - Windows: Download from <https://github.com/UB-Mannheim/tesseract/wiki>
///
/// # Key Knobs
///
/// **Page Segmentation Mode (PSM)** - The biggest practical lever:
/// - 0 = OSD only (orientation and script detection)
/// - 1 = Automatic page segmentation with OSD
/// - 3 = Fully automatic page segmentation (default)
/// - 4 = Single column of variable sizes
/// - 6 = Uniform block of text
/// - 7 = Single text line
/// - 8 = Single word
/// - 11 = Sparse text
///
/// If you set the wrong mode, accuracy tanks.
///
/// **OCR Engine Mode (OEM)**:
/// - 0 = Legacy engine only
/// - 1 = Neural nets LSTM engine only
/// - 2 = Legacy + LSTM engines
/// - 3 = Default (auto-select best available)
///
/// **Preprocessing**: Tesseract is EXTREMELY sensitive to:
/// - Resolution (DPI) - 300 DPI is typical minimum
/// - Binarization, denoise, deskew, contrast normalization
///
/// # Example (Future)
///
/// ```ignore
/// use converge_provider::ocr::{TesseractOcrProvider, TesseractConfig, TesseractOutputFormat, OcrRequest};
///
/// let config = TesseractConfig::new()
///     .with_languages(vec!["eng", "deu"])
///     .with_dpi(300)
///     .with_psm(3)  // Fully automatic
///     .with_output_format(TesseractOutputFormat::Tsv);  // Get bounding boxes
///
/// let provider = TesseractOcrProvider::with_config(config);
/// let result = provider.extract(&OcrRequest::from_pdf_bytes(pdf_bytes))?;
///
/// // Provenance includes tool version, language pack, preprocessing
/// println!("Tesseract version: {}", result.provenance.version);
/// println!("Confidence: {:.2}%", result.confidence.unwrap().mean * 100.0);
///
/// // Check spans for evidence validation
/// for span in &result.spans {
///     if span.is_low_confidence(0.75) {
///         println!("Low confidence word: {} ({:.0}%)", span.text, span.confidence * 100.0);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TesseractConfig {
    /// Path to tesseract binary (default: "tesseract" in PATH).
    pub binary_path: String,
    /// Path to tessdata directory (language files).
    pub tessdata_path: Option<String>,
    /// Languages to use (e.g., ["eng", "deu"]).
    /// DON'T leave language detection implicit!
    pub languages: Vec<String>,
    /// DPI for PDF rendering (default: 300).
    /// 300 DPI is typical minimum for good results.
    pub dpi: u32,
    /// Page segmentation mode (PSM).
    /// 0 = OSD only, 1 = auto + OSD, 3 = fully auto (default), 6 = uniform block, etc.
    /// THIS IS THE BIGGEST PRACTICAL LEVER. Wrong mode = bad accuracy.
    pub psm: u32,
    /// OCR engine mode (OEM).
    /// 0 = Legacy, 1 = Neural LSTM, 2 = Legacy + LSTM, 3 = Default (auto).
    /// Pin for reproducibility.
    pub oem: u32,
    /// Output format (text, TSV, hOCR, ALTO).
    /// Use TSV or hOCR for word-level confidence and bounding boxes.
    pub output_format: TesseractOutputFormat,
    /// Whether to apply preprocessing (deskew, denoise, binarize).
    /// Tesseract is EXTREMELY sensitive to image quality.
    pub preprocess: bool,
    /// Timeout in seconds for OCR operation.
    pub timeout_secs: u64,
}

impl Default for TesseractConfig {
    fn default() -> Self {
        Self {
            binary_path: "tesseract".to_string(),
            tessdata_path: None,
            languages: vec!["eng".to_string()],
            dpi: 300,
            psm: 3, // Fully automatic page segmentation
            oem: 3, // Default (auto-select best available)
            output_format: TesseractOutputFormat::Text,
            preprocess: true,
            timeout_secs: 60,
        }
    }
}

impl TesseractConfig {
    /// Creates a new Tesseract configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path to the tesseract binary.
    #[must_use]
    pub fn with_binary_path(mut self, path: impl Into<String>) -> Self {
        self.binary_path = path.into();
        self
    }

    /// Sets the tessdata directory path.
    #[must_use]
    pub fn with_tessdata_path(mut self, path: impl Into<String>) -> Self {
        self.tessdata_path = Some(path.into());
        self
    }

    /// Sets the languages to use.
    #[must_use]
    pub fn with_languages(mut self, languages: Vec<impl Into<String>>) -> Self {
        self.languages = languages.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the DPI for PDF rendering.
    #[must_use]
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Sets the page segmentation mode.
    #[must_use]
    pub fn with_psm(mut self, psm: u32) -> Self {
        self.psm = psm;
        self
    }

    /// Sets the OCR engine mode.
    #[must_use]
    pub fn with_oem(mut self, oem: u32) -> Self {
        self.oem = oem;
        self
    }

    /// Sets whether to apply preprocessing.
    #[must_use]
    pub fn with_preprocess(mut self, preprocess: bool) -> Self {
        self.preprocess = preprocess;
        self
    }

    /// Sets the timeout in seconds.
    #[must_use]
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Sets the output format.
    /// Use TSV or hOCR for word-level confidence and bounding boxes.
    #[must_use]
    pub fn with_output_format(mut self, format: TesseractOutputFormat) -> Self {
        self.output_format = format;
        self
    }
}

/// Tesseract OCR provider (stub - not yet implemented).
///
/// This is a placeholder for the local Tesseract OCR integration.
/// The actual implementation will be feature-gated behind `tesseract`.
///
/// # Architecture
///
/// ```text
/// TesseractOcrProvider
///     │
///     ├── Input (PDF/Image bytes)
///     │      │
///     │      ▼
///     ├── Preprocessing (optional)
///     │      ├── PDF → Images (pdftoppm/pdf2image)
///     │      ├── Deskew (leptonica)
///     │      ├── Denoise (leptonica)
///     │      └── Binarize (leptonica)
///     │      │
///     │      ▼
///     ├── Tesseract CLI/FFI
///     │      │
///     │      ▼
///     ├── Output
///     │      ├── Text (plain or hOCR/ALTO)
///     │      ├── Confidence (per-word)
///     │      └── Bounding boxes (optional)
///     │      │
///     │      ▼
///     └── OcrResult with Provenance
///            ├── text
///            ├── confidence summary
///            ├── provenance (version, langs, params)
///            └── trace hashes (input/output)
/// ```
///
/// # Future Implementation
///
/// When the `tesseract` feature is enabled:
///
/// ```ignore
/// #[cfg(feature = "tesseract")]
/// impl OcrProvider for TesseractOcrProvider {
///     fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError> {
///         // 1. Hash input for trace links
///         // 2. Preprocess if needed (PDF→image, deskew, etc.)
///         // 3. Invoke tesseract CLI or FFI
///         // 4. Parse output (text + confidence)
///         // 5. Hash output for trace links
///         // 6. Return OcrResult with full provenance
///     }
/// }
/// ```
#[derive(Debug)]
pub struct TesseractOcrProvider {
    config: TesseractConfig,
}

impl TesseractOcrProvider {
    /// Creates a new Tesseract OCR provider with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: TesseractConfig::default(),
        }
    }

    /// Creates a provider with custom configuration.
    #[must_use]
    pub fn with_config(config: TesseractConfig) -> Self {
        Self { config }
    }

    /// Sets the languages to use.
    #[must_use]
    pub fn with_languages(mut self, languages: Vec<impl Into<String>>) -> Self {
        self.config.languages = languages.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the DPI for PDF rendering.
    #[must_use]
    pub fn with_dpi(mut self, dpi: u32) -> Self {
        self.config.dpi = dpi;
        self
    }

    /// Checks if Tesseract is available on the system.
    ///
    /// # Errors
    ///
    /// Returns error if Tesseract is not found or cannot be executed.
    pub fn check_availability(&self) -> Result<String, OcrError> {
        // This is a stub - actual implementation would run `tesseract --version`
        Err(OcrError::Api(
            "Tesseract provider not yet implemented. Enable the 'tesseract' feature.".to_string(),
        ))
    }

    /// Returns the Tesseract version (stub).
    #[must_use]
    pub fn version(&self) -> Option<String> {
        None // Stub - would parse `tesseract --version` output
    }
}

impl Default for TesseractOcrProvider {
    fn default() -> Self {
        Self::new()
    }
}

// Stub implementation - will be replaced when feature is implemented
impl OcrProvider for TesseractOcrProvider {
    fn name(&self) -> &'static str {
        "tesseract"
    }

    fn model(&self) -> &'static str {
        "tesseract-stub"
    }

    fn extract(&self, _request: &OcrRequest) -> Result<OcrResult, OcrError> {
        Err(OcrError::Api(
            "Tesseract OCR provider not yet implemented. \
             This is a placeholder for future local OCR support. \
             For now, use MistralOcrProvider, DeepSeekOcrProvider, or LightOnOcrProvider."
                .to_string(),
        ))
    }
}

// =============================================================================
// Helper functions for provenance
// =============================================================================

/// Computes SHA-256 hash of bytes for trace links.
#[must_use]
pub fn compute_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

/// Computes input/output hashes and returns updated provenance.
#[must_use]
pub fn with_trace_hashes(
    mut provenance: OcrProvenance,
    input: &[u8],
    output: &str,
) -> OcrProvenance {
    provenance.input_hash = Some(compute_hash(input));
    provenance.output_hash = Some(compute_hash(output.as_bytes()));
    provenance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_request_builder() {
        let request = OcrRequest::from_pdf_bytes(vec![1, 2, 3])
            .with_output_format(OcrOutputFormat::Html)
            .with_languages(vec!["en".to_string(), "de".to_string()])
            .with_extract_tables(true)
            .with_extract_images(true)
            .with_page_range(0, 10);

        assert_eq!(request.output_format, OcrOutputFormat::Html);
        assert_eq!(request.languages, vec!["en", "de"]);
        assert!(request.extract_tables);
        assert!(request.extract_images);
        assert_eq!(request.page_range, Some((0, 10)));
    }

    #[test]
    fn test_ocr_output_format_default() {
        let format = OcrOutputFormat::default();
        assert_eq!(format, OcrOutputFormat::Markdown);
    }
}
