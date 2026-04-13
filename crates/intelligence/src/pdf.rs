// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! PDF text extraction for Organism.
//!
//! This module is the generic home for text-native PDF ingestion and chunking.
//! OCR of scanned PDFs belongs in [`crate::ocr`]; direct PDF parsing lives here.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Result type for PDF extraction.
pub type Result<T> = std::result::Result<T, PdfError>;

/// Error type for PDF extraction.
#[derive(Debug, thiserror::Error)]
pub enum PdfError {
    #[error("I/O failed for {path}: {message}")]
    Io { path: String, message: String },
    #[error("PDF parse failed for {path}: {message}")]
    Parse { path: String, message: String },
    #[error("invalid PDF input: {0}")]
    InvalidInput(String),
}

/// A parsed PDF with extracted text and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfDocument {
    pub path: PathBuf,
    pub title: Option<String>,
    pub author: Option<String>,
    pub metadata: HashMap<String, String>,
    pub chunks: Vec<PdfChunk>,
    pub page_count: usize,
}

impl PdfDocument {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            title: None,
            author: None,
            metadata: HashMap::new(),
            chunks: Vec::new(),
            page_count: 0,
        }
    }

    pub fn total_chars(&self) -> usize {
        self.chunks.iter().map(|chunk| chunk.content.len()).sum()
    }

    pub fn full_text(&self) -> String {
        self.chunks
            .iter()
            .map(|chunk| chunk.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn chunks_for_page(&self, page: usize) -> Vec<&PdfChunk> {
        self.chunks
            .iter()
            .filter(|chunk| chunk.page_number == page)
            .collect()
    }
}

/// A chunk of extracted text from a PDF page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfChunk {
    pub content: String,
    pub page_number: usize,
    pub chunk_index: usize,
}

impl PdfChunk {
    fn new(content: String, page_number: usize, chunk_index: usize) -> Self {
        Self {
            content,
            page_number,
            chunk_index,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }
}

/// PDF ingestion configuration.
#[derive(Debug, Clone)]
pub struct PdfIngesterConfig {
    pub max_chunk_size: usize,
    pub preserve_line_breaks: bool,
    pub extract_metadata: bool,
    pub min_chunk_size: usize,
}

impl Default for PdfIngesterConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 4_000,
            preserve_line_breaks: false,
            extract_metadata: true,
            min_chunk_size: 100,
        }
    }
}

impl PdfIngesterConfig {
    #[must_use]
    pub fn with_max_chunk_size(mut self, size: usize) -> Self {
        self.max_chunk_size = size;
        self
    }

    #[must_use]
    pub fn with_preserve_line_breaks(mut self, preserve: bool) -> Self {
        self.preserve_line_breaks = preserve;
        self
    }

    #[must_use]
    pub fn with_extract_metadata(mut self, extract: bool) -> Self {
        self.extract_metadata = extract;
        self
    }

    #[must_use]
    pub fn with_min_chunk_size(mut self, size: usize) -> Self {
        self.min_chunk_size = size;
        self
    }
}

/// Text-native PDF ingester.
#[derive(Debug, Clone)]
pub struct PdfIngester {
    config: PdfIngesterConfig,
}

impl Default for PdfIngester {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfIngester {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: PdfIngesterConfig::default(),
        }
    }

    #[must_use]
    pub fn with_config(config: PdfIngesterConfig) -> Self {
        Self { config }
    }

    pub fn ingest_file(&self, path: &Path) -> Result<PdfDocument> {
        if !path.exists() {
            return Err(PdfError::Io {
                path: path.display().to_string(),
                message: "file not found".to_string(),
            });
        }

        let data = fs::read(path).map_err(|error| PdfError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        self.ingest_bytes(&data, path.to_path_buf())
    }

    pub fn ingest_bytes(&self, data: &[u8], path: PathBuf) -> Result<PdfDocument> {
        let mut document = PdfDocument::new(path.clone());
        let text = pdf_extract::extract_text_from_mem(data).map_err(|error| PdfError::Parse {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;

        if self.config.extract_metadata {
            self.extract_metadata_from_bytes(data, &mut document);
        }

        let processed_text = self.process_text(&text);
        document.page_count = self.estimate_page_count(&text);
        document.chunks = self.create_chunks(&processed_text, document.page_count);
        Ok(document)
    }

    fn process_text(&self, text: &str) -> String {
        if self.config.preserve_line_breaks {
            return text.trim().to_string();
        }

        text.lines()
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
            .split('\n')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
            .replace("\n\n\n", "\n\n")
            .trim()
            .to_string()
    }

    fn estimate_page_count(&self, text: &str) -> usize {
        if text.trim().is_empty() {
            0
        } else {
            text.matches('\u{c}').count() + 1
        }
    }

    fn create_chunks(&self, text: &str, page_count: usize) -> Vec<PdfChunk> {
        let pages = if text.contains('\u{c}') {
            text.split('\u{c}').collect::<Vec<_>>()
        } else {
            vec![text]
        };

        let mut chunks = Vec::new();
        let total_pages = page_count.max(pages.len());
        for (page_index, page_text) in pages.into_iter().enumerate() {
            let chunk_bodies = self.chunk_text(page_text);
            for (chunk_index, chunk_body) in chunk_bodies.into_iter().enumerate() {
                chunks.push(PdfChunk::new(
                    chunk_body,
                    (page_index + 1).min(total_pages.max(1)),
                    chunk_index,
                ));
            }
        }

        self.merge_small_chunks(chunks)
    }

    fn chunk_text(&self, text: &str) -> Vec<String> {
        let paragraphs = text
            .split("\n\n")
            .map(str::trim)
            .filter(|paragraph| !paragraph.is_empty())
            .collect::<Vec<_>>();
        if paragraphs.is_empty() {
            return vec![];
        }

        let mut chunks = Vec::new();
        let mut current = String::new();
        for paragraph in paragraphs {
            let separator = if current.is_empty() { "" } else { "\n\n" };
            if current.len() + separator.len() + paragraph.len() > self.config.max_chunk_size
                && !current.is_empty()
            {
                chunks.push(current.trim().to_string());
                current.clear();
            }

            if !current.is_empty() {
                current.push_str(separator);
            }
            current.push_str(paragraph);
        }

        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }

        chunks
    }

    fn merge_small_chunks(&self, chunks: Vec<PdfChunk>) -> Vec<PdfChunk> {
        let mut merged: Vec<PdfChunk> = Vec::new();
        for chunk in chunks {
            if let Some(previous) = merged.last_mut()
                && previous.page_number == chunk.page_number
                && previous.len() < self.config.min_chunk_size
            {
                previous.content.push_str("\n\n");
                previous.content.push_str(&chunk.content);
                continue;
            }
            merged.push(chunk);
        }
        merged
    }

    fn extract_metadata_from_bytes(&self, data: &[u8], document: &mut PdfDocument) {
        let text = String::from_utf8_lossy(data);
        for field in [
            "Title",
            "Author",
            "Creator",
            "Producer",
            "CreationDate",
            "ModDate",
        ] {
            if let Some(value) = extract_metadata_field(&text, field) {
                let key = field.to_ascii_lowercase();
                document.metadata.insert(key.clone(), value.clone());
                match field {
                    "Title" => document.title = Some(value),
                    "Author" => document.author = Some(value),
                    _ => {}
                }
            }
        }
    }
}

fn extract_metadata_field(content: &str, field: &str) -> Option<String> {
    let marker = format!("/{field} (");
    let start = content.find(&marker)? + marker.len();
    let remainder = &content[start..];
    let mut depth = 1usize;
    let mut value = String::new();

    for character in remainder.chars() {
        match character {
            '(' => {
                depth += 1;
                value.push(character);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                value.push(character);
            }
            _ => value.push(character),
        }
    }

    let cleaned = value.trim().replace("\\(", "(").replace("\\)", ")");
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_field_extraction_handles_simple_values() {
        let content = "/Title (Test Document) /Author (Ada)";
        assert_eq!(
            extract_metadata_field(content, "Title").as_deref(),
            Some("Test Document")
        );
        assert_eq!(
            extract_metadata_field(content, "Author").as_deref(),
            Some("Ada")
        );
    }

    #[test]
    fn page_count_uses_form_feed_when_present() {
        let ingester = PdfIngester::new();
        assert_eq!(ingester.estimate_page_count("one\u{c}two\u{c}three"), 3);
        assert_eq!(ingester.estimate_page_count("one page"), 1);
        assert_eq!(ingester.estimate_page_count(""), 0);
    }

    #[test]
    fn full_text_joins_chunks() {
        let mut document = PdfDocument::new(PathBuf::from("test.pdf"));
        document
            .chunks
            .push(PdfChunk::new("first".to_string(), 1, 0));
        document
            .chunks
            .push(PdfChunk::new("second".to_string(), 2, 0));
        assert_eq!(document.full_text(), "first\n\nsecond");
    }
}
