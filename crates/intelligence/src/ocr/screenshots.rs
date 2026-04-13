// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Screenshot ingestion backed by OCR.
//!
//! Migration source: `converge-knowledge/src/ingest/screenshots.rs` (462 lines).
//! Extracts text from screenshots with UI chrome detection, producing
//! structured documents suitable for knowledge base ingestion.

use super::local::{ImageOcrRequest, OcrBackend, OcrBlockKind, OcrDocument, OcrTargetKind};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// A text chunk extracted from a screenshot via OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotTextChunk {
    pub text: String,
    pub confidence: f64,
    pub is_ui_chrome: bool,
    pub source_region: Option<String>,
}

/// A processed screenshot document with OCR results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotDocument {
    pub path: String,
    pub ocr: OcrDocument,
    pub chunks: Vec<ScreenshotTextChunk>,
    pub content_chunks: Vec<ScreenshotTextChunk>,
    pub chrome_chunks: Vec<ScreenshotTextChunk>,
}

/// Configuration for screenshot ingestion.
#[derive(Debug, Clone)]
pub struct ScreenshotIngesterConfig {
    pub language_hints: Vec<String>,
    pub min_confidence: f64,
    pub filter_ui_chrome: bool,
}

impl Default for ScreenshotIngesterConfig {
    fn default() -> Self {
        Self {
            language_hints: vec!["en".to_string()],
            min_confidence: 0.3,
            filter_ui_chrome: true,
        }
    }
}

/// Screenshot ingester — extracts text with UI chrome detection.
pub struct ScreenshotIngester {
    backend: Arc<dyn OcrBackend>,
    config: ScreenshotIngesterConfig,
}

impl ScreenshotIngester {
    pub fn new(backend: Arc<dyn OcrBackend>, config: ScreenshotIngesterConfig) -> Self {
        Self { backend, config }
    }

    /// Ingest a screenshot file, returning a structured document.
    pub fn ingest(&self, path: &Path) -> Result<ScreenshotDocument, String> {
        let request = ImageOcrRequest {
            path: path.to_string_lossy().to_string(),
            target_kind: OcrTargetKind::Screenshot,
            provenance: format!("screenshot:{}", path.display()),
            language_hints: self.config.language_hints.clone(),
        };

        let ocr = self.backend.extract(&request)?;

        let all_chunks: Vec<ScreenshotTextChunk> = ocr
            .blocks
            .iter()
            .filter(|block| block.confidence >= self.config.min_confidence)
            .map(|block| ScreenshotTextChunk {
                text: block.text.clone(),
                confidence: block.confidence,
                is_ui_chrome: block.kind == OcrBlockKind::UiChrome,
                source_region: block
                    .bbox
                    .as_ref()
                    .map(|b| format!("({:.2},{:.2} {:.2}x{:.2})", b.x, b.y, b.width, b.height)),
            })
            .collect();

        let content_chunks: Vec<ScreenshotTextChunk> = all_chunks
            .iter()
            .filter(|c| !c.is_ui_chrome)
            .cloned()
            .collect();

        let chrome_chunks: Vec<ScreenshotTextChunk> = all_chunks
            .iter()
            .filter(|c| c.is_ui_chrome)
            .cloned()
            .collect();

        Ok(ScreenshotDocument {
            path: path.to_string_lossy().to_string(),
            ocr,
            chunks: all_chunks,
            content_chunks,
            chrome_chunks,
        })
    }
}
