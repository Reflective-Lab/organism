// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Stripe Agentic Commerce Protocol types.
//!
//! Types modeled after the ACP specification for AI agent-driven commerce
//! with Shared Payment Tokens, scoped authorization, and human-in-the-loop consent.

use serde::{Deserialize, Serialize};

/// Checkout session status per Stripe ACP specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutStatus {
    /// Session created but not yet ready for payment.
    NotReadyForPayment,
    /// Line items finalized, awaiting payment token.
    ReadyForPayment,
    /// Payment is being processed.
    InProgress,
    /// Payment completed successfully.
    Completed,
    /// Session was canceled.
    Canceled,
}

/// A line item in a checkout session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    /// SKU or product identifier.
    pub sku: Option<String>,
    /// Display name.
    pub name: String,
    /// Quantity.
    pub quantity: u32,
    /// Unit amount in smallest currency unit (e.g., cents).
    pub unit_amount: i64,
    /// ISO 4217 currency code.
    pub currency: String,
}

/// Checkout session representing a Stripe ACP checkout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSession {
    /// Stripe checkout session ID.
    pub id: String,
    /// Current status.
    pub status: CheckoutStatus,
    /// Line items in this session.
    pub line_items: Vec<LineItem>,
    /// Total amount in smallest currency unit.
    pub total: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Fulfillment details (e.g., subscription activation info).
    pub fulfillment: Option<serde_json::Value>,
    /// Client secret for frontend confirmation (if applicable).
    pub client_secret: Option<String>,
    /// URL for hosted checkout page.
    pub url: Option<String>,
}

/// Request to create a new checkout session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckoutRequest {
    /// Line items for this checkout.
    pub line_items: Vec<LineItem>,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Customer ID (if existing Stripe customer).
    pub customer_id: Option<String>,
    /// Customer email (for guest checkout).
    pub customer_email: Option<String>,
    /// Success redirect URL.
    pub success_url: Option<String>,
    /// Cancel redirect URL.
    pub cancel_url: Option<String>,
    /// Arbitrary metadata.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// Request to update an existing checkout session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckoutRequest {
    /// Updated line items (replaces existing).
    pub line_items: Option<Vec<LineItem>>,
    /// Updated metadata (merged with existing).
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Request to complete a checkout session with a shared payment token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteCheckoutRequest {
    /// Shared Payment Token from the user's consent flow.
    pub shared_payment_token: String,
}

/// Summary of a Stripe PaymentIntent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntentSummary {
    /// PaymentIntent ID.
    pub id: String,
    /// Amount in smallest currency unit.
    pub amount: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// PaymentIntent status.
    pub status: String,
}

/// A Stripe payment link for SaaS billing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentLink {
    /// Payment link ID.
    pub id: String,
    /// The URL customers visit to pay.
    pub url: String,
    /// Whether the link is active.
    pub active: bool,
}

/// Request to create a payment link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePaymentLinkRequest {
    /// Line items for this payment link.
    pub line_items: Vec<LineItem>,
    /// Metadata to attach.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// A Stripe customer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    /// Customer ID.
    pub id: String,
    /// Customer email.
    pub email: Option<String>,
    /// Customer name.
    pub name: Option<String>,
}

/// Incoming Stripe webhook event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Event ID.
    pub id: String,
    /// Event type (e.g., "checkout.session.completed").
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event data payload.
    pub data: serde_json::Value,
    /// Unix timestamp when event was created.
    pub created: i64,
}

// =============================================================================
// Meter Event types (billing feature only)
// =============================================================================

/// A Stripe meter event for usage-based billing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterEvent {
    /// The meter event name (as configured in Stripe Dashboard).
    pub event_name: String,
    /// Stripe customer ID.
    pub stripe_customer_id: String,
    /// Usage value (number of units consumed).
    pub value: u32,
    /// Unix timestamp of the event.
    pub timestamp: i64,
}

/// Response from creating a Stripe meter event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterEventResponse {
    /// Event identifier returned by Stripe.
    pub identifier: Option<String>,
}

// =============================================================================
// Credit Ledger types (billing + gcp features)
// =============================================================================

/// Kind of credit transaction.
#[cfg(all(feature = "billing", feature = "gcp"))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreditTransactionKind {
    /// Credits added via payment or subscription.
    TopUp,
    /// Credits deducted for usage.
    Deduction,
    /// Manual adjustment (admin).
    Adjustment,
    /// Credits expired.
    Expiry,
}

/// A credit transaction record (append-only audit log).
#[cfg(all(feature = "billing", feature = "gcp"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransaction {
    /// Transaction ID.
    pub id: String,
    /// User ID.
    pub user_id: String,
    /// Transaction kind.
    pub kind: CreditTransactionKind,
    /// Amount (positive for top-up, negative for deduction).
    pub amount: i64,
    /// Balance after this transaction.
    pub balance_after: i64,
    /// Human-readable description.
    pub description: String,
    /// Reference ID (e.g., job ID, invoice ID).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
    /// Idempotency key to prevent double-processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Created timestamp (ISO 8601).
    pub created_at: String,
}

/// User's current credit balance.
#[cfg(all(feature = "billing", feature = "gcp"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBalance {
    /// User ID.
    pub user_id: String,
    /// Current credit balance.
    pub balance: i64,
    /// Last updated timestamp (ISO 8601).
    pub updated_at: String,
}

/// Billing-specific error type.
#[derive(Debug, thiserror::Error)]
pub enum BillingError {
    /// Stripe API returned an error.
    #[error("Stripe API error ({status}): {message}")]
    StripeApi { status: u16, message: String },

    /// HTTP client error (network, timeout, etc.).
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    /// Webhook signature verification failed.
    #[error("webhook signature verification failed: {0}")]
    WebhookSignature(String),

    /// Invalid request data.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Deserialization error from Stripe response.
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Insufficient credits for the requested operation.
    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[error("insufficient credits: required {required}, available {available}")]
    InsufficientCredits { required: i64, available: i64 },

    /// Ledger persistence error (Firestore).
    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[error("ledger persistence error: {0}")]
    LedgerPersistence(String),
}

// RuntimeError conversion removed — that was a converge-runtime coupling.

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_status_serialization() {
        let status = CheckoutStatus::ReadyForPayment;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"ready_for_payment\"");
    }

    #[test]
    fn test_checkout_status_deserialization() {
        let status: CheckoutStatus = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(status, CheckoutStatus::Completed);
    }

    #[test]
    fn test_line_item_roundtrip() {
        let item = LineItem {
            sku: Some("sku_test".to_string()),
            name: "Test Item".to_string(),
            quantity: 2,
            unit_amount: 1000,
            currency: "usd".to_string(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let restored: LineItem = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "Test Item");
        assert_eq!(restored.quantity, 2);
        assert_eq!(restored.unit_amount, 1000);
    }

    #[test]
    fn test_checkout_session_roundtrip() {
        let session = CheckoutSession {
            id: "cs_test_123".to_string(),
            status: CheckoutStatus::NotReadyForPayment,
            line_items: vec![],
            total: 0,
            currency: "usd".to_string(),
            fulfillment: None,
            client_secret: None,
            url: None,
        };
        let json = serde_json::to_string(&session).unwrap();
        let restored: CheckoutSession = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "cs_test_123");
        assert_eq!(restored.status, CheckoutStatus::NotReadyForPayment);
    }

    #[test]
    fn test_create_checkout_request_with_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("agent_id".to_string(), "agent-001".to_string());
        let req = CreateCheckoutRequest {
            line_items: vec![],
            currency: "sek".to_string(),
            customer_id: None,
            customer_email: Some("test@example.com".to_string()),
            success_url: None,
            cancel_url: None,
            metadata,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("agent_id"));
        assert!(json.contains("sek"));
    }

    #[test]
    fn test_webhook_event_deserialization() {
        let json = r#"{
            "id": "evt_123",
            "type": "checkout.session.completed",
            "data": {"object": {}},
            "created": 1700000000
        }"#;
        let event: WebhookEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, "evt_123");
        assert_eq!(event.event_type, "checkout.session.completed");
        assert_eq!(event.created, 1700000000);
    }

    #[test]
    fn test_billing_error_display() {
        let err = BillingError::StripeApi {
            status: 402,
            message: "Payment required".to_string(),
        };
        assert_eq!(err.to_string(), "Stripe API error (402): Payment required");
    }

    #[test]
    fn test_billing_error_webhook_display() {
        let err = BillingError::WebhookSignature("invalid signature".to_string());
        assert!(err.to_string().contains("invalid signature"));
    }

    #[test]
    fn test_complete_checkout_request() {
        let req = CompleteCheckoutRequest {
            shared_payment_token: "spt_test_token".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("spt_test_token"));
    }

    #[test]
    fn test_payment_link_roundtrip() {
        let link = PaymentLink {
            id: "plink_123".to_string(),
            url: "https://buy.stripe.com/test".to_string(),
            active: true,
        };
        let json = serde_json::to_string(&link).unwrap();
        let restored: PaymentLink = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "plink_123");
        assert!(restored.active);
    }

    #[test]
    fn test_customer_roundtrip() {
        let customer = Customer {
            id: "cus_123".to_string(),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
        };
        let json = serde_json::to_string(&customer).unwrap();
        let restored: Customer = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "cus_123");
        assert_eq!(restored.email.unwrap(), "test@example.com");
    }

    #[test]
    fn test_meter_event_roundtrip() {
        let event = MeterEvent {
            event_name: "convergence_cycles".to_string(),
            stripe_customer_id: "cus_abc123".to_string(),
            value: 5,
            timestamp: 1700000000,
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: MeterEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.event_name, "convergence_cycles");
        assert_eq!(restored.value, 5);
    }

    #[test]
    fn test_meter_event_response_with_identifier() {
        let response = MeterEventResponse {
            identifier: Some("mtr_evt_123".to_string()),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("mtr_evt_123"));
    }

    #[test]
    fn test_meter_event_response_without_identifier() {
        let response = MeterEventResponse { identifier: None };
        let json = serde_json::to_string(&response).unwrap();
        let restored: MeterEventResponse = serde_json::from_str(&json).unwrap();
        assert!(restored.identifier.is_none());
    }

    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[test]
    fn test_credit_transaction_kind_serialization() {
        let kind = CreditTransactionKind::TopUp;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"top_up\"");

        let kind: CreditTransactionKind = serde_json::from_str("\"deduction\"").unwrap();
        assert_eq!(kind, CreditTransactionKind::Deduction);
    }

    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[test]
    fn test_credit_balance_roundtrip() {
        let balance = CreditBalance {
            user_id: "user-123".to_string(),
            balance: 500,
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&balance).unwrap();
        let restored: CreditBalance = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.user_id, "user-123");
        assert_eq!(restored.balance, 500);
    }

    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[test]
    fn test_credit_transaction_roundtrip() {
        let tx = CreditTransaction {
            id: "tx-001".to_string(),
            user_id: "user-123".to_string(),
            kind: CreditTransactionKind::Deduction,
            amount: -10,
            balance_after: 490,
            description: "Job execution".to_string(),
            reference_id: Some("job-456".to_string()),
            idempotency_key: Some("job_456_deduct".to_string()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&tx).unwrap();
        let restored: CreditTransaction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, "tx-001");
        assert_eq!(restored.amount, -10);
        assert_eq!(restored.kind, CreditTransactionKind::Deduction);
    }

    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[test]
    fn test_insufficient_credits_error() {
        let err = BillingError::InsufficientCredits {
            required: 100,
            available: 50,
        };
        assert!(err.to_string().contains("required 100"));
        assert!(err.to_string().contains("available 50"));
    }

    #[cfg(all(feature = "billing", feature = "gcp"))]
    #[test]
    fn test_ledger_persistence_error() {
        let err = BillingError::LedgerPersistence("Firestore timeout".to_string());
        assert!(err.to_string().contains("Firestore timeout"));
    }
}
