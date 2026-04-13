// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Local OCR backends for photo and screenshot ingestion.
//!
//! Migration source: `converge-knowledge/src/ingest/ocr.rs` (1,751 lines).
//!
//! Backends:
//! - `TesseractOcrBackend` — local Tesseract with TSV/hOCR support
//! - `AppleVisionOcrBackend` — macOS Vision framework
//! - `FixtureOcrBackend` — test/mock backend

use serde::{Deserialize, Serialize};

/// Which OCR engine produced the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrEngine {
    Tesseract,
    AppleVision,
    Mock,
    External,
}

/// What kind of image is being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrTargetKind {
    Screenshot,
    Photo,
    GenericImage,
}

/// Normalized bounding box (0.0..=1.0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Kind of detected text block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrBlockKind {
    Paragraph,
    Line,
    Word,
    UiChrome,
    Unknown,
}

/// A detected text block with position and confidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrTextBlock {
    pub text: String,
    pub kind: OcrBlockKind,
    pub confidence: f64,
    pub bbox: Option<BoundingBox>,
}

/// Request for image OCR extraction.
#[derive(Debug, Clone)]
pub struct ImageOcrRequest {
    pub path: String,
    pub target_kind: OcrTargetKind,
    pub provenance: String,
    pub language_hints: Vec<String>,
}

/// Result of image OCR extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrDocument {
    pub engine: OcrEngine,
    pub full_text: String,
    pub blocks: Vec<OcrTextBlock>,
    pub extracted_at: String,
}

/// Local OCR backend trait.
///
/// Full implementations to be ported from:
/// - `converge-knowledge/src/ingest/ocr.rs` (Tesseract, Apple Vision)
pub trait OcrBackend: Send + Sync {
    fn extract(&self, request: &ImageOcrRequest) -> Result<OcrDocument, String>;
}

/// Tesseract configuration.
#[derive(Debug, Clone)]
pub struct TesseractOcrConfig {
    pub language: String,
    pub dpi: u32,
    pub psm: u8,
    pub oem: u8,
}

impl Default for TesseractOcrConfig {
    fn default() -> Self {
        Self {
            language: "eng".to_string(),
            dpi: 300,
            psm: 3,
            oem: 3,
        }
    }
}

/// Apple Vision recognition level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleVisionRecognitionLevel {
    Accurate,
    Fast,
}

/// Apple Vision configuration.
#[derive(Debug, Clone)]
pub struct AppleVisionOcrConfig {
    pub recognition_level: AppleVisionRecognitionLevel,
    pub language_hints: Vec<String>,
}

impl Default for AppleVisionOcrConfig {
    fn default() -> Self {
        Self {
            recognition_level: AppleVisionRecognitionLevel::Accurate,
            language_hints: vec!["en".to_string()],
        }
    }
}
