use anyhow::{Context, Result};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

use crate::models::{DataSource, FieldMapping, FormConfig, Profile, SelectorType};

/// The form filler engine - drives browser automation
pub struct Filler {
    driver: WebDriver,
}

impl Filler {
    /// Create a new filler with a WebDriver connection
    /// Requires chromedriver/geckodriver running on the specified port
    pub async fn new(webdriver_url: &str) -> Result<Self> {
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::new(webdriver_url, caps)
            .await
            .context("Failed to connect to WebDriver")?;

        Ok(Self { driver })
    }

    /// Fill a form using the given profile and config
    pub async fn fill_form(&self, profile: &Profile, config: &FormConfig) -> Result<FillResult> {
        let mut result = FillResult::default();

        // Navigate to the form
        self.driver
            .goto(&config.url)
            .await
            .context("Failed to navigate to form URL")?;

        // Pre-fill delay
        sleep(Duration::from_millis(config.pre_fill_delay_ms)).await;

        // Fill each field
        for field in &config.fields {
            match self.fill_field(profile, field).await {
                Ok(()) => result.filled += 1,
                Err(e) => {
                    result.errors.push(format!(
                        "Field '{}': {}",
                        field.selector,
                        e
                    ));
                    if field.required {
                        result.success = false;
                    }
                }
            }

            // Delay between fields
            sleep(Duration::from_millis(config.field_delay_ms)).await;
        }

        result.success = result.errors.is_empty();
        Ok(result)
    }

    async fn fill_field(&self, profile: &Profile, field: &FieldMapping) -> Result<()> {
        // Find the element
        let element = match field.selector_type {
            SelectorType::Css => self.driver.find(By::Css(&field.selector)).await?,
            SelectorType::XPath => self.driver.find(By::XPath(&field.selector)).await?,
            SelectorType::Id => self.driver.find(By::Id(&field.selector)).await?,
            SelectorType::Name => self.driver.find(By::Name(&field.selector)).await?,
        };

        // Get the value from profile
        let value = self.resolve_data_source(profile, &field.source);

        // Fill based on field type
        element.send_keys(&value).await?;

        Ok(())
    }

    fn resolve_data_source(&self, profile: &Profile, source: &DataSource) -> String {
        match source {
            // Personal
            DataSource::FirstName => profile.personal.first_name.clone(),
            DataSource::LastName => profile.personal.last_name.clone(),
            DataSource::FullName => {
                format!("{} {}", profile.personal.first_name, profile.personal.last_name)
            }
            DataSource::DateOfBirth => {
                profile.personal.date_of_birth.clone().unwrap_or_default()
            }
            DataSource::Gender => profile.personal.gender.clone().unwrap_or_default(),
            DataSource::Nationality => profile.personal.nationality.clone().unwrap_or_default(),

            // Contact
            DataSource::Email => profile.contact.email.clone(),
            DataSource::Phone => profile.contact.phone.clone(),
            DataSource::PhoneAlt => profile.contact.phone_alt.clone().unwrap_or_default(),

            // Address
            DataSource::Street => profile.address.street.clone(),
            DataSource::Street2 => profile.address.street2.clone().unwrap_or_default(),
            DataSource::City => profile.address.city.clone(),
            DataSource::PostalCode => profile.address.postal_code.clone(),
            DataSource::Region => profile.address.region.clone().unwrap_or_default(),
            DataSource::Country => profile.address.country.clone(),
            DataSource::FullAddress => {
                format!(
                    "{}, {} {}",
                    profile.address.street, profile.address.postal_code, profile.address.city
                )
            }

            // Identification
            DataSource::PersonalNumber => {
                profile.identification.personal_number.clone().unwrap_or_default()
            }
            DataSource::PassportNumber => {
                profile.identification.passport_number.clone().unwrap_or_default()
            }
            DataSource::IdCardNumber => {
                profile.identification.id_card_number.clone().unwrap_or_default()
            }
            DataSource::DriversLicense => {
                profile.identification.drivers_license.clone().unwrap_or_default()
            }

            // Intent by name
            DataSource::Intent(name) => profile
                .intents
                .iter()
                .find(|i| i.name == *name)
                .map(|i| i.content.clone())
                .unwrap_or_default(),

            // Custom field by key
            DataSource::Custom(key) => profile
                .custom_fields
                .iter()
                .find(|f| f.key == *key)
                .map(|f| f.value.clone())
                .unwrap_or_default(),

            // Static value
            DataSource::Static(value) => value.clone(),
        }
    }

    /// Submit the form (if submit selector is configured)
    pub async fn submit(&self, config: &FormConfig) -> Result<()> {
        if let Some(selector) = &config.submit_selector {
            let button = self.driver.find(By::Css(selector)).await?;
            button.click().await?;
        }
        Ok(())
    }

    /// Close the browser
    pub async fn quit(self) -> Result<()> {
        self.driver.quit().await?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FillResult {
    pub success: bool,
    pub filled: usize,
    pub errors: Vec<String>,
}
