//! Learner module - fetches suggestions from logging server and updates configs
//!
//! Workflow:
//! 1. User visits form manually or bot attempts fill
//! 2. Logger captures all field interactions
//! 3. Learner fetches suggestions from /api/suggestions
//! 4. Merges suggestions into existing FormConfig
//! 5. Saves updated config for next attempt

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::{DataSource, FieldMapping, FieldType, FormConfig, FormStep, SelectorType};
use crate::storage;

const DEFAULT_SERVER: &str = "http://localhost:3001";

/// Suggestion from the logging server
#[derive(Debug, Clone, Deserialize)]
pub struct FieldSuggestion {
    pub selector: String,
    pub selector_type: String,
    pub field_type: String,
    pub source: Option<String>,
    pub confidence: Option<f32>,
    pub label: Option<String>,
    pub required: bool,
}

/// Fetch suggestions from the logging server
pub async fn fetch_suggestions(server_url: Option<&str>) -> Result<HashMap<String, Vec<FieldSuggestion>>> {
    let url = format!("{}/api/suggestions", server_url.unwrap_or(DEFAULT_SERVER));

    let response = reqwest::get(&url)
        .await
        .context("Failed to connect to logging server")?;

    let suggestions: HashMap<String, Vec<FieldSuggestion>> = response
        .json()
        .await
        .context("Failed to parse suggestions")?;

    Ok(suggestions)
}

/// Fetch analysis data from the logging server
pub async fn fetch_analysis(server_url: Option<&str>) -> Result<serde_json::Value> {
    let url = format!("{}/api/analysis", server_url.unwrap_or(DEFAULT_SERVER));

    let response = reqwest::get(&url)
        .await
        .context("Failed to connect to logging server")?;

    let analysis: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse analysis")?;

    Ok(analysis)
}

/// Convert a suggestion to a FieldMapping
fn suggestion_to_field_mapping(suggestion: &FieldSuggestion) -> Option<FieldMapping> {
    let source = match suggestion.source.as_deref() {
        Some("Email") => DataSource::Email,
        Some("Phone") => DataSource::Phone,
        Some("PersonalNumber") => DataSource::PersonalNumber,
        Some("FirstName") => DataSource::FirstName,
        Some("LastName") => DataSource::LastName,
        Some("Street") => DataSource::Street,
        Some("PostalCode") => DataSource::PostalCode,
        Some("City") => DataSource::City,
        Some("Country") => DataSource::Country,
        Some("DateOfBirth") => DataSource::DateOfBirth,
        Some(other) => DataSource::Custom(other.to_string()),
        None => return None, // Skip fields we can't map
    };

    let selector_type = match suggestion.selector_type.as_str() {
        "Css" => SelectorType::Css,
        "Name" => SelectorType::Name,
        "Id" => SelectorType::Id,
        "XPath" => SelectorType::XPath,
        _ => SelectorType::Css,
    };

    let field_type = match suggestion.field_type.as_str() {
        "Email" => FieldType::Email,
        "Phone" => FieldType::Phone,
        "Date" => FieldType::Date,
        "Select" => FieldType::Select,
        "TextArea" => FieldType::TextArea,
        "Checkbox" => FieldType::Checkbox,
        "Radio" => FieldType::Radio,
        _ => FieldType::Text,
    };

    Some(FieldMapping {
        selector: suggestion.selector.clone(),
        selector_type,
        field_type,
        source,
        required: suggestion.required,
    })
}

/// Create a new FormConfig from suggestions for a URL
pub fn create_config_from_suggestions(
    pathname: &str,
    base_url: &str,
    suggestions: &[FieldSuggestion],
) -> FormConfig {
    let name = pathname
        .trim_start_matches('/')
        .replace(".html", "")
        .replace('-', " ")
        .to_string();

    let url = format!("{}{}", base_url.trim_end_matches('/'), pathname);

    let fields: Vec<FieldMapping> = suggestions
        .iter()
        .filter_map(suggestion_to_field_mapping)
        .collect();

    let mut config = FormConfig::new(name, url);
    config.fields = fields;
    config.submit_selector = Some("button[type='submit']".to_string());

    config
}

/// Update an existing FormConfig with new suggestions
pub fn merge_suggestions_into_config(
    config: &mut FormConfig,
    suggestions: &[FieldSuggestion],
) -> usize {
    let mut added = 0;

    for suggestion in suggestions {
        // Skip if we already have a mapping for this selector
        let exists = config.fields.iter().any(|f| f.selector == suggestion.selector)
            || config.steps.iter().any(|s| {
                s.fields.iter().any(|f| f.selector == suggestion.selector)
            });

        if exists {
            continue;
        }

        if let Some(mapping) = suggestion_to_field_mapping(suggestion) {
            config.fields.push(mapping);
            added += 1;
        }
    }

    added
}

/// Learn from the logging server and update form configs
pub async fn learn_and_update(server_url: Option<&str>, base_url: &str) -> Result<LearnResult> {
    let suggestions = fetch_suggestions(server_url).await?;

    let mut existing_configs = storage::load_form_configs().unwrap_or_default();
    let mut result = LearnResult::default();

    for (pathname, field_suggestions) in suggestions {
        // Find existing config for this URL
        let full_url = format!("{}{}", base_url.trim_end_matches('/'), pathname);

        let existing = existing_configs
            .iter_mut()
            .find(|c| c.url == full_url || c.url.ends_with(&pathname));

        if let Some(config) = existing {
            // Update existing config
            let added = merge_suggestions_into_config(config, &field_suggestions);
            if added > 0 {
                result.updated_configs += 1;
                result.new_fields += added;
            }
        } else {
            // Create new config
            let new_config = create_config_from_suggestions(&pathname, base_url, &field_suggestions);
            if !new_config.fields.is_empty() {
                result.new_configs += 1;
                result.new_fields += new_config.fields.len();
                existing_configs.push(new_config);
            }
        }
    }

    // Save updated configs
    if result.new_configs > 0 || result.updated_configs > 0 {
        storage::save_form_configs(&existing_configs)?;
    }

    result.total_configs = existing_configs.len();

    Ok(result)
}

/// Result of a learning operation
#[derive(Debug, Default)]
pub struct LearnResult {
    pub new_configs: usize,
    pub updated_configs: usize,
    pub new_fields: usize,
    pub total_configs: usize,
}

impl std::fmt::Display for LearnResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Learning complete:\n  \
             New configs: {}\n  \
             Updated configs: {}\n  \
             New fields added: {}\n  \
             Total configs: {}",
            self.new_configs, self.updated_configs, self.new_fields, self.total_configs
        )
    }
}

/// Analyze validation errors and suggest fixes
pub async fn analyze_errors(server_url: Option<&str>) -> Result<Vec<ErrorAnalysis>> {
    let analysis = fetch_analysis(server_url).await?;

    let mut errors = Vec::new();

    if let Some(validation_errors) = analysis.get("validationErrors").and_then(|v| v.as_array()) {
        for err in validation_errors {
            errors.push(ErrorAnalysis {
                pathname: err.get("pathname").and_then(|v| v.as_str()).map(String::from),
                field: err.get("field").and_then(|v| v.as_str()).map(String::from),
                message: err.get("message").and_then(|v| v.as_str()).map(String::from),
                suggestion: suggest_fix(
                    err.get("message").and_then(|v| v.as_str()),
                ),
            });
        }
    }

    Ok(errors)
}

#[derive(Debug)]
pub struct ErrorAnalysis {
    pub pathname: Option<String>,
    pub field: Option<String>,
    pub message: Option<String>,
    pub suggestion: Option<String>,
}

fn suggest_fix(message: Option<&str>) -> Option<String> {
    let msg = message?.to_lowercase();

    if msg.contains("required") || msg.contains("obligatorisk") {
        return Some("Field is required but wasn't filled. Check selector.".to_string());
    }

    if msg.contains("email") || msg.contains("e-post") {
        return Some("Invalid email format. Check DataSource::Email mapping.".to_string());
    }

    if msg.contains("pattern") || msg.contains("format") {
        return Some("Value doesn't match expected pattern. Check field format.".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_to_mapping() {
        let suggestion = FieldSuggestion {
            selector: "#email".to_string(),
            selector_type: "Css".to_string(),
            field_type: "Email".to_string(),
            source: Some("Email".to_string()),
            confidence: Some(0.95),
            label: Some("E-post".to_string()),
            required: true,
        };

        let mapping = suggestion_to_field_mapping(&suggestion).unwrap();
        assert_eq!(mapping.selector, "#email");
        assert!(matches!(mapping.source, DataSource::Email));
        assert!(matches!(mapping.field_type, FieldType::Email));
    }
}
