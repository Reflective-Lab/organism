// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Stripe HTTP client for the Agentic Commerce Protocol.
//!
//! Wraps `reqwest` to talk to Stripe's REST API with typed requests/responses,
//! idempotency key support, and proper error mapping.

use reqwest::Client;
use tracing::{debug, warn};

use super::types::{
    BillingError, CheckoutSession, CompleteCheckoutRequest, CreateCheckoutRequest,
    CreatePaymentLinkRequest, Customer, MeterEventResponse, PaymentLink, UpdateCheckoutRequest,
};
use crate::config::BillingConfig;

const DEFAULT_STRIPE_BASE_URL: &str = "https://api.stripe.com/v1";

/// Stripe API client for Agentic Commerce Protocol operations.
#[derive(Debug, Clone)]
pub struct StripeClient {
    http: Client,
    api_key: String,
    base_url: String,
}

impl StripeClient {
    /// Create a new `StripeClient` from a `BillingConfig`.
    pub fn new(config: &BillingConfig) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            http,
            api_key: config.stripe_api_key.clone(),
            base_url: config
                .stripe_base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_STRIPE_BASE_URL.to_string()),
        }
    }

    /// Create a new `StripeClient` from environment variables.
    ///
    /// Reads `STRIPE_API_KEY` and optionally `STRIPE_BASE_URL`.
    pub fn from_env() -> Result<Self, BillingError> {
        let api_key = std::env::var("STRIPE_API_KEY").map_err(|_| {
            BillingError::InvalidRequest("STRIPE_API_KEY environment variable not set".to_string())
        })?;

        let base_url = std::env::var("STRIPE_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_STRIPE_BASE_URL.to_string());

        let config = BillingConfig {
            stripe_api_key: api_key,
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            stripe_base_url: Some(base_url),
            meter_event_name: std::env::var("STRIPE_METER_EVENT_NAME")
                .unwrap_or_else(|_| "convergence_cycles".to_string()),
            credits_per_cycle: std::env::var("STRIPE_CREDITS_PER_CYCLE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
        };

        Ok(Self::new(&config))
    }

    /// Build a request with standard Stripe auth headers.
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.base_url))
            .bearer_auth(&self.api_key)
            .header("Stripe-Version", "2024-12-18.acacia")
    }

    /// Build a request with an idempotency key.
    fn request_with_idempotency(
        &self,
        method: reqwest::Method,
        path: &str,
        idempotency_key: Option<&str>,
    ) -> reqwest::RequestBuilder {
        let mut builder = self.request(method, path);
        if let Some(key) = idempotency_key {
            builder = builder.header("Idempotency-Key", key);
        }
        builder
    }

    /// Parse a Stripe API response, mapping errors to `BillingError`.
    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, BillingError> {
        let status = response.status();
        if status.is_success() {
            let body = response.text().await?;
            debug!(body_len = body.len(), "Stripe API response");
            serde_json::from_str(&body).map_err(BillingError::Deserialization)
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!(status = status_code, body = %body, "Stripe API error");

            // Try to extract Stripe error message
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("error")?.get("message")?.as_str().map(String::from))
                .unwrap_or(body);

            Err(BillingError::StripeApi {
                status: status_code,
                message,
            })
        }
    }

    // =========================================================================
    // Checkout Session endpoints (ACP)
    // =========================================================================

    /// Create a new checkout session.
    pub async fn create_checkout(
        &self,
        req: &CreateCheckoutRequest,
    ) -> Result<CheckoutSession, BillingError> {
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let response = self
            .request_with_idempotency(
                reqwest::Method::POST,
                "/checkout/sessions",
                Some(&idempotency_key),
            )
            .json(req)
            .send()
            .await?;

        self.parse_response(response).await
    }

    /// Get a checkout session by ID.
    pub async fn get_checkout(&self, id: &str) -> Result<CheckoutSession, BillingError> {
        let response = self
            .request(reqwest::Method::GET, &format!("/checkout/sessions/{id}"))
            .send()
            .await?;

        self.parse_response(response).await
    }

    /// Update a checkout session.
    pub async fn update_checkout(
        &self,
        id: &str,
        req: &UpdateCheckoutRequest,
    ) -> Result<CheckoutSession, BillingError> {
        let response = self
            .request(reqwest::Method::POST, &format!("/checkout/sessions/{id}"))
            .json(req)
            .send()
            .await?;

        self.parse_response(response).await
    }

    /// Complete a checkout session with a shared payment token.
    pub async fn complete_checkout(
        &self,
        id: &str,
        req: &CompleteCheckoutRequest,
    ) -> Result<CheckoutSession, BillingError> {
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let response = self
            .request_with_idempotency(
                reqwest::Method::POST,
                &format!("/checkout/sessions/{id}/complete"),
                Some(&idempotency_key),
            )
            .json(req)
            .send()
            .await?;

        self.parse_response(response).await
    }

    /// Cancel a checkout session.
    pub async fn cancel_checkout(&self, id: &str) -> Result<CheckoutSession, BillingError> {
        let response = self
            .request(
                reqwest::Method::POST,
                &format!("/checkout/sessions/{id}/cancel"),
            )
            .send()
            .await?;

        self.parse_response(response).await
    }

    // =========================================================================
    // Payment Link endpoints (SaaS billing)
    // =========================================================================

    /// Create a payment link for SaaS billing.
    pub async fn create_payment_link(
        &self,
        req: &CreatePaymentLinkRequest,
    ) -> Result<PaymentLink, BillingError> {
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let response = self
            .request_with_idempotency(
                reqwest::Method::POST,
                "/payment_links",
                Some(&idempotency_key),
            )
            .json(req)
            .send()
            .await?;

        self.parse_response(response).await
    }

    // =========================================================================
    // Customer endpoints
    // =========================================================================

    /// Create a new Stripe customer.
    pub async fn create_customer(
        &self,
        email: &str,
        name: Option<&str>,
    ) -> Result<Customer, BillingError> {
        let idempotency_key = uuid::Uuid::new_v4().to_string();
        let mut params = vec![("email", email.to_string())];
        if let Some(n) = name {
            params.push(("name", n.to_string()));
        }

        let response = self
            .request_with_idempotency(reqwest::Method::POST, "/customers", Some(&idempotency_key))
            .form(&params)
            .send()
            .await?;

        self.parse_response(response).await
    }

    // =========================================================================
    // Meter Event endpoints (usage-based billing)
    // =========================================================================

    /// Report usage to Stripe via meter events.
    ///
    /// This is a fire-and-forget style call — callers should log but not fail
    /// if this returns an error.
    pub async fn report_usage(
        &self,
        event_name: &str,
        stripe_customer_id: &str,
        value: u32,
        idempotency_key: Option<&str>,
    ) -> Result<MeterEventResponse, BillingError> {
        let params = vec![
            ("event_name", event_name.to_string()),
            (
                "payload[stripe_customer_id]",
                stripe_customer_id.to_string(),
            ),
            ("payload[value]", value.to_string()),
        ];

        debug!(
            event_name,
            customer = stripe_customer_id,
            value,
            "Reporting usage to Stripe"
        );

        let response = self
            .request_with_idempotency(
                reqwest::Method::POST,
                "/billing/meter_events",
                idempotency_key,
            )
            .form(&params)
            .send()
            .await?;

        self.parse_response(response).await
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BillingConfig {
        BillingConfig {
            stripe_api_key: "sk_test_fake_key".to_string(),
            stripe_webhook_secret: Some("whsec_test_secret".to_string()),
            stripe_base_url: Some("https://api.stripe.com/v1".to_string()),
            meter_event_name: "convergence_cycles".to_string(),
            credits_per_cycle: 1,
        }
    }

    #[test]
    fn test_stripe_client_new() {
        let config = test_config();
        let client = StripeClient::new(&config);
        assert_eq!(client.api_key, "sk_test_fake_key");
        assert_eq!(client.base_url, "https://api.stripe.com/v1");
    }

    #[test]
    fn test_stripe_client_default_base_url() {
        let config = BillingConfig {
            stripe_api_key: "sk_test_key".to_string(),
            stripe_webhook_secret: None,
            stripe_base_url: None,
            meter_event_name: "convergence_cycles".to_string(),
            credits_per_cycle: 1,
        };
        let client = StripeClient::new(&config);
        assert_eq!(client.base_url, DEFAULT_STRIPE_BASE_URL);
    }

    #[test]
    fn test_stripe_client_clone() {
        let config = test_config();
        let client = StripeClient::new(&config);
        let cloned = client.clone();
        assert_eq!(client.api_key, cloned.api_key);
        assert_eq!(client.base_url, cloned.base_url);
    }

    #[test]
    fn test_stripe_client_debug() {
        let config = test_config();
        let client = StripeClient::new(&config);
        let debug_str = format!("{client:?}");
        assert!(debug_str.contains("StripeClient"));
    }
}
