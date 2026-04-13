//! Form analyzer - discovers form fields from a URL
//!
//! Takes a URL and attempts to:
//! 1. Find all form elements
//! 2. Extract field metadata (name, type, label, placeholder)
//! 3. Guess what profile data should fill each field
//! 4. Generate a FormConfig automatically

use anyhow::Result;
use serde::{Deserialize, Serialize};
use thirtyfour::prelude::*;

use crate::models::{DataSource, FieldMapping, FieldType, FormConfig, FormStep, SelectorType};

/// Discovered form field with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredField {
    /// Best selector to use (id preferred, then name, then css path)
    pub selector: String,
    pub selector_type: SelectorType,

    /// HTML input type
    pub input_type: String,

    /// Field name attribute
    pub name: Option<String>,

    /// Field id attribute
    pub id: Option<String>,

    /// Associated label text
    pub label: Option<String>,

    /// Placeholder text
    pub placeholder: Option<String>,

    /// aria-label if present
    pub aria_label: Option<String>,

    /// Whether field has required attribute
    pub required: bool,

    /// Guessed data source based on field metadata
    pub guessed_source: Option<DataSource>,

    /// Confidence score for the guess (0.0 - 1.0)
    pub confidence: f32,
}

/// Result of analyzing a form
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormAnalysis {
    pub url: String,
    pub title: Option<String>,
    pub fields: Vec<DiscoveredField>,
    pub submit_selector: Option<String>,
    pub is_multi_step: bool,
    pub next_button_selector: Option<String>,
    pub step_indicators: Vec<String>,
}

impl FormAnalysis {
    /// Convert analysis to a FormConfig
    pub fn to_form_config(&self, name: String) -> FormConfig {
        let fields: Vec<FieldMapping> = self
            .fields
            .iter()
            .filter_map(|f| {
                f.guessed_source.as_ref().map(|source| FieldMapping {
                    selector: f.selector.clone(),
                    selector_type: f.selector_type.clone(),
                    field_type: guess_field_type(&f.input_type),
                    source: source.clone(),
                    required: f.required,
                })
            })
            .collect();

        let mut config = FormConfig::new(name, self.url.clone());
        config.fields = fields;
        config.submit_selector = self.submit_selector.clone();
        config
    }
}

/// Analyze a form at the given URL
pub async fn analyze_form(driver: &WebDriver, url: &str) -> Result<FormAnalysis> {
    driver.goto(url).await?;

    // Wait for page to load
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let title = driver.title().await.ok();

    // Find all input fields
    let mut fields = Vec::new();

    // Find inputs
    let inputs = driver.find_all(By::Css("input:not([type='hidden']):not([type='submit']):not([type='button'])")).await?;
    for input in inputs {
        if let Ok(field) = analyze_input_element(&driver, &input).await {
            fields.push(field);
        }
    }

    // Find selects
    let selects = driver.find_all(By::Css("select")).await?;
    for select in selects {
        if let Ok(field) = analyze_select_element(&driver, &select).await {
            fields.push(field);
        }
    }

    // Find textareas
    let textareas = driver.find_all(By::Css("textarea")).await?;
    for textarea in textareas {
        if let Ok(field) = analyze_textarea_element(&driver, &textarea).await {
            fields.push(field);
        }
    }

    // Detect submit button
    let submit_selector = find_submit_button(&driver).await;

    // Detect if multi-step
    let (is_multi_step, next_button_selector) = detect_multi_step(&driver).await;

    // Find step indicators
    let step_indicators = find_step_indicators(&driver).await;

    Ok(FormAnalysis {
        url: url.to_string(),
        title,
        fields,
        submit_selector,
        is_multi_step,
        next_button_selector,
        step_indicators,
    })
}

async fn analyze_input_element(
    driver: &WebDriver,
    element: &WebElement,
) -> Result<DiscoveredField> {
    let input_type = element.attr("type").await?.unwrap_or_else(|| "text".to_string());
    let name = element.attr("name").await?;
    let id = element.attr("id").await?;
    let placeholder = element.attr("placeholder").await?;
    let aria_label = element.attr("aria-label").await?;
    let required = element.attr("required").await?.is_some();

    // Try to find associated label
    let label = find_label_for_element(driver, &id, &name).await;

    // Determine best selector
    let (selector, selector_type) = determine_selector(&id, &name);

    // Guess what data source this field needs
    let (guessed_source, confidence) = guess_data_source(
        &input_type,
        name.as_deref(),
        id.as_deref(),
        label.as_deref(),
        placeholder.as_deref(),
        aria_label.as_deref(),
    );

    Ok(DiscoveredField {
        selector,
        selector_type,
        input_type,
        name,
        id,
        label,
        placeholder,
        aria_label,
        required,
        guessed_source,
        confidence,
    })
}

async fn analyze_select_element(
    driver: &WebDriver,
    element: &WebElement,
) -> Result<DiscoveredField> {
    let name = element.attr("name").await?;
    let id = element.attr("id").await?;
    let aria_label = element.attr("aria-label").await?;
    let required = element.attr("required").await?.is_some();

    let label = find_label_for_element(driver, &id, &name).await;
    let (selector, selector_type) = determine_selector(&id, &name);

    // For selects, we can't easily guess the data source without knowing options
    let (guessed_source, confidence) = guess_data_source(
        "select",
        name.as_deref(),
        id.as_deref(),
        label.as_deref(),
        None,
        aria_label.as_deref(),
    );

    Ok(DiscoveredField {
        selector,
        selector_type,
        input_type: "select".to_string(),
        name,
        id,
        label,
        placeholder: None,
        aria_label,
        required,
        guessed_source,
        confidence,
    })
}

async fn analyze_textarea_element(
    driver: &WebDriver,
    element: &WebElement,
) -> Result<DiscoveredField> {
    let name = element.attr("name").await?;
    let id = element.attr("id").await?;
    let placeholder = element.attr("placeholder").await?;
    let aria_label = element.attr("aria-label").await?;
    let required = element.attr("required").await?.is_some();

    let label = find_label_for_element(driver, &id, &name).await;
    let (selector, selector_type) = determine_selector(&id, &name);

    let (guessed_source, confidence) = guess_data_source(
        "textarea",
        name.as_deref(),
        id.as_deref(),
        label.as_deref(),
        placeholder.as_deref(),
        aria_label.as_deref(),
    );

    Ok(DiscoveredField {
        selector,
        selector_type,
        input_type: "textarea".to_string(),
        name,
        id,
        label,
        placeholder,
        aria_label,
        required,
        guessed_source,
        confidence,
    })
}

async fn find_label_for_element(
    driver: &WebDriver,
    id: &Option<String>,
    name: &Option<String>,
) -> Option<String> {
    // Try label[for=id]
    if let Some(id) = id {
        if let Ok(label) = driver.find(By::Css(&format!("label[for='{}']", id))).await {
            if let Ok(text) = label.text().await {
                if !text.is_empty() {
                    return Some(text.trim().to_string());
                }
            }
        }
    }

    // Try label[for=name]
    if let Some(name) = name {
        if let Ok(label) = driver.find(By::Css(&format!("label[for='{}']", name))).await {
            if let Ok(text) = label.text().await {
                if !text.is_empty() {
                    return Some(text.trim().to_string());
                }
            }
        }
    }

    None
}

fn determine_selector(id: &Option<String>, name: &Option<String>) -> (String, SelectorType) {
    if let Some(id) = id {
        if !id.is_empty() {
            return (format!("#{}", id), SelectorType::Css);
        }
    }

    if let Some(name) = name {
        if !name.is_empty() {
            return (name.clone(), SelectorType::Name);
        }
    }

    // Fallback - this shouldn't happen often
    ("input".to_string(), SelectorType::Css)
}

/// Guess what profile data source should fill this field
fn guess_data_source(
    input_type: &str,
    name: Option<&str>,
    id: Option<&str>,
    label: Option<&str>,
    placeholder: Option<&str>,
    aria_label: Option<&str>,
) -> (Option<DataSource>, f32) {
    // Combine all text hints
    let hints: Vec<&str> = [name, id, label, placeholder, aria_label]
        .into_iter()
        .flatten()
        .collect();

    let combined = hints.join(" ").to_lowercase();

    // Email field
    if input_type == "email" || combined.contains("e-post") || combined.contains("email") || combined.contains("epost") {
        return (Some(DataSource::Email), 0.95);
    }

    // Phone field
    if input_type == "tel" || combined.contains("telefon") || combined.contains("phone") || combined.contains("mobil") {
        return (Some(DataSource::Phone), 0.95);
    }

    // Swedish personnummer
    if combined.contains("personnummer") || combined.contains("personal number") || combined.contains("ssn") {
        return (Some(DataSource::PersonalNumber), 0.95);
    }

    // First name
    if combined.contains("förnamn") || combined.contains("fornamn") || combined.contains("firstname") || combined.contains("first name") {
        return (Some(DataSource::FirstName), 0.9);
    }

    // Last name
    if combined.contains("efternamn") || combined.contains("lastname") || combined.contains("last name") || combined.contains("surname") {
        return (Some(DataSource::LastName), 0.9);
    }

    // Street address
    if combined.contains("gatuadress") || combined.contains("street") || combined.contains("adress") && !combined.contains("e-post") {
        return (Some(DataSource::Street), 0.85);
    }

    // Postal code
    if combined.contains("postnummer") || combined.contains("postal") || combined.contains("zip") || combined.contains("postcode") {
        return (Some(DataSource::PostalCode), 0.9);
    }

    // City
    if combined.contains("ort") || combined.contains("stad") || combined.contains("city") || combined.contains("town") {
        return (Some(DataSource::City), 0.85);
    }

    // Country
    if combined.contains("land") || combined.contains("country") {
        return (Some(DataSource::Country), 0.85);
    }

    // Date of birth
    if combined.contains("födelse") || combined.contains("birth") || combined.contains("dob") {
        return (Some(DataSource::DateOfBirth), 0.8);
    }

    // No confident guess
    (None, 0.0)
}

fn guess_field_type(input_type: &str) -> FieldType {
    match input_type {
        "email" => FieldType::Email,
        "tel" => FieldType::Phone,
        "date" => FieldType::Date,
        "select" => FieldType::Select,
        "textarea" => FieldType::TextArea,
        "checkbox" => FieldType::Checkbox,
        "radio" => FieldType::Radio,
        _ => FieldType::Text,
    }
}

async fn find_submit_button(driver: &WebDriver) -> Option<String> {
    // Try various submit button selectors
    let selectors = [
        "button[type='submit']",
        "input[type='submit']",
        "button:contains('Skicka')",
        "button:contains('Submit')",
        "button:contains('Anmäl')",
        ".submit-button",
        "#submit",
    ];

    for selector in selectors {
        if driver.find(By::Css(selector)).await.is_ok() {
            return Some(selector.to_string());
        }
    }

    None
}

async fn detect_multi_step(driver: &WebDriver) -> (bool, Option<String>) {
    // Look for next/forward buttons
    let next_selectors = [
        "button:contains('Nästa')",
        "button:contains('Next')",
        "#btnNext",
        ".btn-next",
        "button[type='button']:contains('→')",
        "input[value='Nästa']",
    ];

    for selector in next_selectors {
        if driver.find(By::Css(selector)).await.is_ok() {
            return (true, Some(selector.to_string()));
        }
    }

    // Look for step indicators
    let step_indicators = [
        ".progress-bar",
        ".wizard-steps",
        ".step-indicator",
        "[class*='progress']",
        "[class*='step']",
    ];

    for selector in step_indicators {
        if driver.find(By::Css(selector)).await.is_ok() {
            return (true, None);
        }
    }

    (false, None)
}

async fn find_step_indicators(driver: &WebDriver) -> Vec<String> {
    let mut indicators = Vec::new();

    let selectors = [".progress-bar", ".progress-fill", ".step", ".wizard-step"];

    for selector in selectors {
        if driver.find(By::Css(selector)).await.is_ok() {
            indicators.push(selector.to_string());
        }
    }

    indicators
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_email() {
        let (source, conf) = guess_data_source("email", Some("email"), None, None, None, None);
        assert!(matches!(source, Some(DataSource::Email)));
        assert!(conf > 0.9);
    }

    #[test]
    fn test_guess_personnummer() {
        let (source, conf) = guess_data_source("text", Some("personnummer"), None, Some("Personnummer"), None, None);
        assert!(matches!(source, Some(DataSource::PersonalNumber)));
        assert!(conf > 0.9);
    }

    #[test]
    fn test_guess_swedish_fields() {
        let (source, _) = guess_data_source("text", Some("fornamn"), None, Some("Förnamn"), None, None);
        assert!(matches!(source, Some(DataSource::FirstName)));

        let (source, _) = guess_data_source("text", Some("efternamn"), None, None, None, None);
        assert!(matches!(source, Some(DataSource::LastName)));

        let (source, _) = guess_data_source("text", None, Some("postnummer"), None, None, None);
        assert!(matches!(source, Some(DataSource::PostalCode)));
    }
}
