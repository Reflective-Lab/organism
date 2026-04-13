use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfFieldFixture {
    pub field_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfSchemaFixture {
    pub form_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub fields: Vec<PdfFieldFixture>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfFillPlan {
    pub form_id: String,
    pub missing_fields: Vec<String>,
    pub ready_for_submit: bool,
}

pub fn load_fixture(path: &str) -> Result<PdfSchemaFixture> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read PDF fixture: {}", path))?;
    let schema: PdfSchemaFixture = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse PDF fixture JSON: {}", path))?;
    Ok(schema)
}

pub fn build_plan(schema: &PdfSchemaFixture) -> PdfFillPlan {
    let missing_fields = schema
        .fields
        .iter()
        .filter(|field| field.required)
        .map(|field| field.field_id.clone())
        .collect::<Vec<_>>();

    PdfFillPlan {
        form_id: schema.form_id.clone(),
        missing_fields: missing_fields.clone(),
        ready_for_submit: missing_fields.is_empty(),
    }
}

pub fn write_plan(plan: &PdfFillPlan, output_path: &str) -> Result<()> {
    let payload = serde_json::to_string_pretty(plan)?;
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, payload)
        .with_context(|| format!("Failed to write plan to {}", output_path))?;
    Ok(())
}
