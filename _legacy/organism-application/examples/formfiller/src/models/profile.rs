use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A person's profile containing all data needed for form filling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
    pub personal: PersonalInfo,
    pub contact: ContactInfo,
    pub address: Address,
    pub identification: Identification,
    pub intents: Vec<Intent>,
    pub custom_fields: Vec<CustomField>,
}

impl Profile {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            personal: PersonalInfo::default(),
            contact: ContactInfo::default(),
            address: Address::default(),
            identification: Identification::default(),
            intents: Vec::new(),
            custom_fields: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonalInfo {
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: Option<String>, // ISO 8601 format
    pub gender: Option<String>,
    pub nationality: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactInfo {
    pub email: String,
    pub phone: String,
    pub phone_alt: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub street2: Option<String>,
    pub city: String,
    pub postal_code: String,
    pub region: Option<String>, // State/province/county
    pub country: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Identification {
    pub personal_number: Option<String>, // Swedish personnummer, SSN, etc.
    pub passport_number: Option<String>,
    pub id_card_number: Option<String>,
    pub drivers_license: Option<String>,
}

/// Pre-written intent/motivation text for applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub name: String,        // e.g., "Housing application motivation"
    pub content: String,     // The actual text
    pub tags: Vec<String>,   // For easy filtering
}

/// For form-specific fields not covered by standard fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

/// Credentials for forms requiring login (future feature)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    // BankID would be handled separately via system integration
}
