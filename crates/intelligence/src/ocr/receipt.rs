// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Receipt-specific OCR for expense processing.
//!
//! Migration source: `saas-killer/prio-expenses/src/organism_ocr_bridge.rs` (613 lines).
//!
//! Backends:
//! - `TesseractCliOcrProvider` — CLI-based Tesseract for receipts
//! - `OllamaReceiptOcrProvider` — LLM-powered receipt understanding via Ollama

use serde::{Deserialize, Serialize};

/// Configuration for Tesseract CLI backend.
#[derive(Debug, Clone)]
pub struct TesseractCliConfig {
    pub binary_path: String,
    pub tessdata_path: Option<String>,
    pub languages: Vec<String>,
    pub dpi: u32,
    pub psm: u8,
    pub oem: u8,
}

impl Default for TesseractCliConfig {
    fn default() -> Self {
        Self {
            binary_path: "tesseract".to_string(),
            tessdata_path: None,
            languages: vec!["eng".to_string()],
            dpi: 300,
            psm: 3,
            oem: 3,
        }
    }
}

impl TesseractCliConfig {
    pub fn from_env() -> Self {
        Self {
            binary_path: std::env::var("EXPENSES_OCR_TESSERACT_BIN")
                .unwrap_or_else(|_| "tesseract".to_string()),
            tessdata_path: std::env::var("EXPENSES_OCR_TESSDATA_PATH").ok(),
            languages: std::env::var("EXPENSES_OCR_TESSERACT_LANG")
                .map(|v| v.split(',').map(str::trim).map(String::from).collect())
                .unwrap_or_else(|_| vec!["eng".to_string()]),
            ..Self::default()
        }
    }
}

/// Configuration for Ollama receipt OCR backend.
#[derive(Debug, Clone)]
pub struct OllamaReceiptConfig {
    pub base_url: String,
    pub model: String,
}

impl Default for OllamaReceiptConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "glm-ocr".to_string(),
        }
    }
}

impl OllamaReceiptConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("EXPENSES_OCR_OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            model: std::env::var("EXPENSES_OCR_OLLAMA_MODEL")
                .unwrap_or_else(|_| "glm-ocr".to_string()),
        }
    }
}

/// Extracted receipt fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptExtraction {
    pub vendor: Option<String>,
    pub total: Option<f64>,
    pub currency: Option<String>,
    pub date: Option<String>,
    pub items: Vec<ReceiptLineItem>,
    pub raw_text: String,
}

/// A line item from a receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptLineItem {
    pub description: String,
    pub quantity: Option<f64>,
    pub unit_price: Option<f64>,
    pub total: Option<f64>,
}

/// Receipt OCR provider trait.
///
/// Full implementations to be ported from:
/// - `saas-killer/prio-expenses/src/organism_ocr_bridge.rs` (TesseractCli, Ollama)
pub trait ReceiptOcrProvider: Send + Sync {
    fn extract_receipt(&self, image_bytes: &[u8]) -> Result<ReceiptExtraction, String>;
}
