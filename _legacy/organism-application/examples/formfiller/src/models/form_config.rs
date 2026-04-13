use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration for a specific form/website
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub description: Option<String>,

    /// For single-page forms: list of all fields
    #[serde(default)]
    pub fields: Vec<FieldMapping>,

    /// For multi-step forms: list of steps (each with fields and next button)
    #[serde(default)]
    pub steps: Vec<FormStep>,

    pub submit_selector: Option<String>,
    pub auth: AuthRequirement,
    pub pre_fill_delay_ms: u64,  // Delay before starting to fill
    pub field_delay_ms: u64,     // Delay between fields (some sites need this)
    #[serde(default = "default_step_delay")]
    pub step_delay_ms: u64,      // Delay after clicking next (for page transitions)
}

fn default_step_delay() -> u64 {
    500
}

impl FormConfig {
    pub fn new(name: String, url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            url,
            description: None,
            fields: Vec::new(),
            steps: Vec::new(),
            submit_selector: None,
            auth: AuthRequirement::None,
            pre_fill_delay_ms: 500,
            field_delay_ms: 50,
            step_delay_ms: 500,
        }
    }

    /// Check if this is a multi-step form
    pub fn is_multi_step(&self) -> bool {
        !self.steps.is_empty()
    }
}

/// A single step in a multi-step form wizard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormStep {
    /// Human-readable name for this step
    pub name: Option<String>,

    /// Fields to fill on this step
    pub fields: Vec<FieldMapping>,

    /// Selector for the "Next" button (None for last step - uses submit_selector)
    pub next_selector: Option<String>,

    /// Optional: wait for this element to appear before filling (for dynamic pages)
    pub wait_for_selector: Option<String>,
}

/// Maps a form field (by selector) to profile data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub selector: String,           // CSS selector or XPath
    pub selector_type: SelectorType,
    pub field_type: FieldType,
    pub source: DataSource,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectorType {
    Css,
    XPath,
    Id,
    Name,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    Text,
    Email,
    Phone,
    Date,
    Select,     // Dropdown
    Radio,
    Checkbox,
    TextArea,
}

/// Where to get the data for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    // Personal info
    FirstName,
    LastName,
    FullName,
    DateOfBirth,
    Gender,
    Nationality,

    // Contact
    Email,
    Phone,
    PhoneAlt,

    // Address
    Street,
    Street2,
    City,
    PostalCode,
    Region,
    Country,
    FullAddress,

    // Identification
    PersonalNumber,
    PassportNumber,
    IdCardNumber,
    DriversLicense,

    // Intent - by name
    Intent(String),

    // Custom field - by key
    Custom(String),

    // Static value (always the same)
    Static(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthRequirement {
    None,
    UsernamePassword,
    BankId,
    // Future: OAuth, etc.
}

// --- CAPTCHA Notes ---
//
// If target forms add CAPTCHA, options include:
//
// 1. **Manual intervention**: Pause automation, alert user, let them solve it
//    - Best for occasional CAPTCHAs
//    - Add a "waiting for CAPTCHA" state in TUI
//
// 2. **Audio CAPTCHA**: Some can be solved via speech-to-text APIs
//    - Works for reCAPTCHA v2 audio option
//
// 3. **CAPTCHA solving services**:
//    - 2captcha, Anti-Captcha, CapMonster
//    - Cost money, add latency (10-30 seconds)
//    - May violate ToS
//
// 4. **Browser fingerprinting**:
//    - Use real browser profiles
//    - Avoid detection patterns (consistent timing, mouse movements)
//
// Recommended approach: Start with manual intervention, add service
// integration as optional feature if needed.
