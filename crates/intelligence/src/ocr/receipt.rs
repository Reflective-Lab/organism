// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Receipt-specific OCR for expense processing.
//!
//! These providers implement the canonical [`OcrProvider`] contract while
//! keeping receipt-oriented local execution paths together:
//! - `TesseractCliOcrProvider` — classic local OCR via the `tesseract` binary
//! - `OllamaReceiptOcrProvider` — local multimodal OCR via Ollama

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::Value;

use super::cloud::{
    OcrError, OcrOutputFormat, OcrPreprocessing, OcrProvenance, OcrProvider, OcrRequest, OcrResult,
};

/// Configuration for the Tesseract CLI backend.
#[derive(Debug, Clone)]
pub struct TesseractCliConfig {
    pub binary_path: String,
    pub tessdata_path: Option<String>,
    pub languages: Vec<String>,
    pub dpi: u32,
    pub psm: u32,
    pub oem: u32,
}

impl Default for TesseractCliConfig {
    fn default() -> Self {
        Self {
            binary_path: std::env::var("EXPENSES_OCR_TESSERACT_BIN")
                .unwrap_or_else(|_| "tesseract".to_string()),
            tessdata_path: std::env::var("EXPENSES_OCR_TESSDATA_PATH").ok(),
            languages: std::env::var("EXPENSES_OCR_TESSERACT_LANG")
                .ok()
                .map(|value| {
                    value
                        .split('+')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .filter(|languages| !languages.is_empty())
                .unwrap_or_else(|| vec!["eng".to_string(), "fra".to_string(), "swe".to_string()]),
            dpi: 300,
            psm: 3,
            oem: 3,
        }
    }
}

/// Tesseract-backed receipt OCR provider.
#[derive(Debug, Clone)]
pub struct TesseractCliOcrProvider {
    config: TesseractCliConfig,
}

impl TesseractCliOcrProvider {
    #[must_use]
    pub fn with_config(config: TesseractCliConfig) -> Self {
        Self { config }
    }
}

impl OcrProvider for TesseractCliOcrProvider {
    fn name(&self) -> &'static str {
        "tesseract"
    }

    fn model(&self) -> &str {
        "tesseract-cli"
    }

    fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError> {
        let started = Instant::now();
        let prepared = materialize_visual_input(request)?;
        let output = Command::new(&self.config.binary_path)
            .arg(&prepared.ocr_input_path)
            .arg("stdout")
            .arg("-l")
            .arg(self.config.languages.join("+"))
            .arg("--psm")
            .arg(self.config.psm.to_string())
            .arg("--oem")
            .arg(self.config.oem.to_string())
            .args(tessdata_args(self.config.tessdata_path.as_deref()))
            .output()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    OcrError::Api(format!(
                        "{} is not installed or not in PATH",
                        self.config.binary_path
                    ))
                } else {
                    OcrError::Api(error.to_string())
                }
            })?;

        if !output.status.success() {
            return Err(OcrError::Api(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }

        Ok(OcrResult {
            text: String::from_utf8_lossy(&output.stdout).to_string(),
            pages: 1,
            spans: vec![],
            tables: vec![],
            images: vec![],
            confidence: None,
            processing_time_ms: Some(started.elapsed().as_millis() as u64),
            provenance: OcrProvenance {
                provider: self.name().to_string(),
                version: tesseract_version(&self.config.binary_path),
                languages: self.config.languages.clone(),
                preprocessing: OcrPreprocessing {
                    dpi: Some(self.config.dpi),
                    psm: Some(self.config.psm),
                    oem: Some(self.config.oem),
                    ..OcrPreprocessing::default()
                },
                input_hash: None,
                output_hash: None,
                metadata: HashMap::from([
                    (
                        "input_path".to_string(),
                        prepared.ocr_input_path.display().to_string(),
                    ),
                    ("source_kind".to_string(), prepared.source_kind.to_string()),
                ]),
            },
        })
    }
}

/// Configuration for Ollama-backed receipt OCR.
#[derive(Debug, Clone)]
pub struct OllamaReceiptConfig {
    pub base_url: String,
    pub model: String,
}

impl Default for OllamaReceiptConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("EXPENSES_OCR_OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
            model: std::env::var("EXPENSES_OCR_OLLAMA_MODEL")
                .unwrap_or_else(|_| "glm-ocr".to_string()),
        }
    }
}

/// Ollama-backed receipt OCR provider.
#[derive(Debug, Clone)]
pub struct OllamaReceiptOcrProvider {
    config: OllamaReceiptConfig,
}

impl OllamaReceiptOcrProvider {
    #[must_use]
    pub fn with_config(config: OllamaReceiptConfig) -> Self {
        Self { config }
    }
}

impl OcrProvider for OllamaReceiptOcrProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn extract(&self, request: &OcrRequest) -> Result<OcrResult, OcrError> {
        let started = Instant::now();
        let prepared = materialize_visual_input(request)?;
        let image_bytes = fs::read(&prepared.ocr_input_path).map_err(|error| {
            OcrError::Api(format!(
                "failed to read {}: {error}",
                prepared.ocr_input_path.display()
            ))
        })?;
        let encoded = base64::engine::general_purpose::STANDARD.encode(image_bytes);
        let prompt = ollama_prompt_for_output(request.output_format);
        let request_body = match request.output_format {
            OcrOutputFormat::Json => serde_json::json!({
                "model": self.config.model,
                "prompt": prompt,
                "images": [encoded],
                "stream": false,
                "format": "json",
            }),
            _ => serde_json::json!({
                "model": self.config.model,
                "prompt": prompt,
                "images": [encoded],
                "stream": false,
            }),
        };

        let client = reqwest::blocking::Client::new();
        let response = client
            .post(format!(
                "{}/api/generate",
                self.config.base_url.trim_end_matches('/')
            ))
            .json(&request_body)
            .send()
            .map_err(|error| OcrError::Network(error.to_string()))?;
        let status = response.status();
        let payload: Value = response
            .json()
            .map_err(|error| OcrError::Parse(error.to_string()))?;
        if !status.is_success() {
            return Err(OcrError::Api(payload.to_string()));
        }

        let text = payload
            .get("response")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        Ok(OcrResult {
            text,
            pages: 1,
            spans: vec![],
            tables: vec![],
            images: vec![],
            confidence: None,
            processing_time_ms: Some(started.elapsed().as_millis() as u64),
            provenance: OcrProvenance {
                provider: self.name().to_string(),
                version: self.config.model.clone(),
                languages: request.languages.clone(),
                preprocessing: OcrPreprocessing::default(),
                input_hash: None,
                output_hash: None,
                metadata: HashMap::from([
                    ("base_url".to_string(), self.config.base_url.clone()),
                    ("source_kind".to_string(), prepared.source_kind.to_string()),
                ]),
            },
        })
    }
}

fn ollama_prompt_for_output(format: OcrOutputFormat) -> &'static str {
    match format {
        OcrOutputFormat::Json => {
            "Extract expense document fields and return JSON only. Use empty strings for unknown values. Return exactly these keys: merchant, issue_date, service_date, service_period_start, service_period_end, due_date, currency, total, subtotal, tax, tax_rate, invoice_number, receipt_number, order_id, account_reference, country. Normalize dates to YYYY-MM-DD. Normalize amounts to decimal strings using a dot."
        }
        OcrOutputFormat::Text => {
            "Extract all text from this document in reading order. Keep it plain text."
        }
        OcrOutputFormat::Markdown => {
            "Extract all text from this document, preserving layout and reading order. Use markdown for simple structure."
        }
        OcrOutputFormat::Html => {
            "Extract all text from this document, preserving layout and reading order. Use simple HTML for structure."
        }
    }
}

fn tessdata_args<'a>(path: Option<&'a str>) -> Vec<&'a str> {
    match path {
        Some(path) => vec!["--tessdata-dir", path],
        None => vec![],
    }
}

fn tesseract_version(binary_path: &str) -> String {
    let output = Command::new(binary_path).arg("--version").output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("tesseract")
            .trim()
            .to_string(),
        _ => "tesseract".to_string(),
    }
}

struct PreparedVisualInput {
    ocr_input_path: PathBuf,
    source_kind: &'static str,
}

fn materialize_visual_input(request: &OcrRequest) -> Result<PreparedVisualInput, OcrError> {
    match &request.input {
        super::cloud::OcrInput::PdfBytes(bytes) => {
            let pdf_path = unique_temp_path("receipt-ocr-input", "pdf");
            fs::write(&pdf_path, bytes).map_err(|error| {
                OcrError::Api(format!("failed to write {}: {error}", pdf_path.display()))
            })?;
            let png_path = rasterize_pdf_to_png(&pdf_path)?;
            Ok(PreparedVisualInput {
                ocr_input_path: png_path,
                source_kind: "pdf",
            })
        }
        super::cloud::OcrInput::ImageBytes(bytes) => {
            let image_path = unique_temp_path("receipt-ocr-input", "png");
            fs::write(&image_path, bytes).map_err(|error| {
                OcrError::Api(format!("failed to write {}: {error}", image_path.display()))
            })?;
            Ok(PreparedVisualInput {
                ocr_input_path: image_path,
                source_kind: "image",
            })
        }
        super::cloud::OcrInput::Base64(data) => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|error| OcrError::InvalidInput(error.to_string()))?;
            let image_path = unique_temp_path("receipt-ocr-input", "png");
            fs::write(&image_path, bytes).map_err(|error| {
                OcrError::Api(format!("failed to write {}: {error}", image_path.display()))
            })?;
            Ok(PreparedVisualInput {
                ocr_input_path: image_path,
                source_kind: "base64-image",
            })
        }
        super::cloud::OcrInput::Url(url) => Err(OcrError::InvalidInput(format!(
            "URL inputs are not supported yet: {url}"
        ))),
    }
}

fn rasterize_pdf_to_png(path: &Path) -> Result<PathBuf, OcrError> {
    let output = unique_temp_path("receipt-ocr-page", "png");
    let result = Command::new("sips")
        .arg("-s")
        .arg("format")
        .arg("png")
        .arg(path)
        .arg("--out")
        .arg(&output)
        .output();

    match result {
        Ok(output_result) if output_result.status.success() => Ok(output),
        Ok(output_result) => Err(OcrError::Api(
            String::from_utf8_lossy(&output_result.stderr)
                .trim()
                .to_string(),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(OcrError::Api(
            "sips is required on macOS to rasterize PDF pages".to_string(),
        )),
        Err(error) => Err(OcrError::Api(error.to_string())),
    }
}

fn unique_temp_path(prefix: &str, extension: &str) -> PathBuf {
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{epoch}.{extension}"))
}

/// Build an OCR request from a local document path.
pub fn ocr_request_for_path(
    path: &Path,
    output_format: OcrOutputFormat,
    languages: Vec<String>,
) -> Result<OcrRequest, OcrError> {
    let bytes = fs::read(path)
        .map_err(|error| OcrError::Api(format!("failed to read {}: {error}", path.display())))?;
    let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    let request = match extension {
        "pdf" => OcrRequest::from_pdf_bytes(bytes),
        "png" | "jpg" | "jpeg" | "webp" => OcrRequest::from_image_bytes(bytes),
        _ => {
            return Err(OcrError::InvalidInput(format!(
                "unsupported document extension for OCR path: {}",
                path.display()
            )));
        }
    };
    Ok(request
        .with_output_format(output_format)
        .with_languages(languages))
}
