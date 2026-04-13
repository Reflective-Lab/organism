// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! OCR — Document understanding for Organism.
//!
//! This is the single home for all OCR capabilities across the Reflective Labs
//! stack. Consolidated from three separate implementations:
//!
//! - `cloud` — Cloud LLM-based document AI (Mistral, DeepSeek, LightOn)
//!   Source: `converge-provider/src/ocr.rs`
//! - `local` — Local backends (Tesseract, Apple Vision) for photo/screenshot ingestion
//!   Source: `converge-knowledge/src/ingest/ocr.rs`
//! - `receipt` — Receipt-specific OCR (TesseractCli, Ollama) for expense processing
//!   Source: `saas-killer/prio-expenses/src/organism_ocr_bridge.rs`
//!
//! # Canonical Trait
//!
//! All backends implement [`OcrService`] — the unified trait for document extraction.
//! Consumers inject `Arc<dyn OcrService>` and don't know which backend they're using.
//!
//! # Available Backends
//!
//! | Backend | Module | Network | Best for |
//! |---|---|---|---|
//! | Mistral OCR 3 | `cloud` | Yes | GDPR-compliant EU document AI |
//! | DeepSeek OCR 2 | `cloud` | Yes | Visual Causal Flow analysis |
//! | LightOn OCR | `cloud` | Yes | Efficient open-source extraction |
//! | Tesseract | `local` | No | Local document/screenshot OCR |
//! | Apple Vision | `local` | No | macOS-native high-quality OCR |
//! | TesseractCli | `receipt` | No | Receipt extraction via CLI |
//! | Ollama | `receipt` | Local | LLM-powered receipt understanding |

pub mod cloud;
pub mod local;
pub mod photos;
pub mod receipt;
pub mod screenshots;

// Re-export the most commonly used types from cloud (the primary API)
pub use cloud::{
    OcrConfidence, OcrError, OcrImage, OcrInput, OcrOutputFormat, OcrPreprocessing, OcrProvenance,
    OcrProvider, OcrRequest, OcrResult, OcrSpan, OcrTable,
};
pub use receipt::{
    OllamaReceiptConfig, OllamaReceiptOcrProvider, TesseractCliConfig, TesseractCliOcrProvider,
    ocr_request_for_path,
};
