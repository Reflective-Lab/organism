// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Photo ingestion backed by OCR.
//!
//! Migration source: `converge-knowledge/src/ingest/photos.rs` (423 lines).
//! Extracts text from photos using OCR backends, producing structured
//! documents suitable for knowledge base ingestion.

use super::local::{ImageOcrRequest, OcrBackend, OcrDocument, OcrTargetKind};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// A text chunk extracted from a photo via OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoTextChunk {
    pub text: String,
    pub confidence: f64,
    pub source_region: Option<String>,
}

/// A processed photo document with OCR results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoDocument {
    pub path: String,
    pub ocr: OcrDocument,
    pub chunks: Vec<PhotoTextChunk>,
    pub metadata: PhotoMetadata,
}

/// Metadata about the source photo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
}

/// Configuration for photo ingestion.
#[derive(Debug, Clone)]
pub struct PhotoIngesterConfig {
    pub language_hints: Vec<String>,
    pub min_confidence: f64,
    pub ocr_weight: f64,
}

impl Default for PhotoIngesterConfig {
    fn default() -> Self {
        Self {
            language_hints: vec!["en".to_string()],
            min_confidence: 0.3,
            ocr_weight: 1.0,
        }
    }
}

/// Photo ingester — extracts text from photos using an OCR backend.
pub struct PhotoIngester {
    backend: Arc<dyn OcrBackend>,
    config: PhotoIngesterConfig,
}

impl PhotoIngester {
    pub fn new(backend: Arc<dyn OcrBackend>, config: PhotoIngesterConfig) -> Self {
        Self { backend, config }
    }

    /// Ingest a photo file, returning a structured document.
    pub fn ingest(&self, path: &Path) -> Result<PhotoDocument, String> {
        let request = ImageOcrRequest {
            path: path.to_string_lossy().to_string(),
            target_kind: OcrTargetKind::Photo,
            provenance: format!("photo:{}", path.display()),
            language_hints: self.config.language_hints.clone(),
        };

        let ocr = self.backend.extract(&request)?;

        let chunks: Vec<PhotoTextChunk> = ocr
            .blocks
            .iter()
            .filter(|block| block.confidence >= self.config.min_confidence)
            .map(|block| PhotoTextChunk {
                text: block.text.clone(),
                confidence: block.confidence,
                source_region: block
                    .bbox
                    .as_ref()
                    .map(|b| format!("({:.2},{:.2} {:.2}x{:.2})", b.x, b.y, b.width, b.height)),
            })
            .collect();

        Ok(PhotoDocument {
            path: path.to_string_lossy().to_string(),
            ocr,
            chunks,
            metadata: PhotoMetadata {
                width: None,
                height: None,
                format: path.extension().and_then(|e| e.to_str()).map(String::from),
            },
        })
    }
}
