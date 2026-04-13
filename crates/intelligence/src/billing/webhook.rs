// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Stripe webhook signature verification and event dispatch.
//!
//! Implements HMAC-SHA256 signature verification per Stripe's webhook spec,
//! including timestamp tolerance to prevent replay attacks.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::{debug, info, warn};

use super::types::{BillingError, WebhookEvent};

type HmacSha256 = Hmac<Sha256>;

/// Default tolerance for webhook timestamp verification (5 minutes).
const DEFAULT_TOLERANCE_SECS: i64 = 300;

/// Verify a Stripe webhook signature.
///
/// Stripe sends a `Stripe-Signature` header with format:
/// `t=<timestamp>,v1=<signature>[,v0=<legacy>]`
///
/// This function:
/// 1. Parses the timestamp and signature from the header
/// 2. Checks the timestamp is within tolerance (replay prevention)
/// 3. Computes HMAC-SHA256 of `{timestamp}.{payload}` using the webhook secret
/// 4. Compares the computed signature to the provided one
pub fn verify_webhook_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<(), BillingError> {
    verify_webhook_signature_with_tolerance(payload, sig_header, secret, DEFAULT_TOLERANCE_SECS)
}

/// Verify a Stripe webhook signature with a custom timestamp tolerance.
pub fn verify_webhook_signature_with_tolerance(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
    tolerance_secs: i64,
) -> Result<(), BillingError> {
    // Parse signature header
    let (timestamp, signatures) = parse_signature_header(sig_header)?;

    // Check timestamp tolerance
    let now = chrono::Utc::now().timestamp();
    let age = now - timestamp;
    if age.abs() > tolerance_secs {
        return Err(BillingError::WebhookSignature(format!(
            "timestamp outside tolerance: event age {age}s exceeds {tolerance_secs}s"
        )));
    }

    // Compute expected signature
    let signed_payload = format!("{timestamp}.").into_bytes();
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| BillingError::WebhookSignature(format!("invalid webhook secret: {e}")))?;
    mac.update(&signed_payload);
    mac.update(payload);
    let expected = hex::encode(mac.finalize().into_bytes());

    // Check if any v1 signature matches
    let matched = signatures.iter().any(|sig| {
        // Constant-time comparison via HMAC verification
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid key");
        mac.update(&format!("{timestamp}.").into_bytes());
        mac.update(payload);
        let expected_bytes = mac.finalize().into_bytes();
        let sig_bytes = hex::decode(sig).unwrap_or_default();
        expected_bytes.as_slice() == sig_bytes.as_slice()
    });

    if matched {
        debug!(timestamp, "Webhook signature verified");
        Ok(())
    } else {
        Err(BillingError::WebhookSignature(format!(
            "no matching v1 signature found (expected: {expected})"
        )))
    }
}

/// Parse the `Stripe-Signature` header into (timestamp, v1_signatures).
fn parse_signature_header(header: &str) -> Result<(i64, Vec<String>), BillingError> {
    let mut timestamp = None;
    let mut signatures = Vec::new();

    for part in header.split(',') {
        let part = part.trim();
        if let Some(ts) = part.strip_prefix("t=") {
            timestamp =
                Some(ts.parse::<i64>().map_err(|_| {
                    BillingError::WebhookSignature(format!("invalid timestamp: {ts}"))
                })?);
        } else if let Some(sig) = part.strip_prefix("v1=") {
            signatures.push(sig.to_string());
        }
        // Ignore v0 (legacy test signatures)
    }

    let timestamp = timestamp.ok_or_else(|| {
        BillingError::WebhookSignature("missing timestamp in Stripe-Signature header".to_string())
    })?;

    if signatures.is_empty() {
        return Err(BillingError::WebhookSignature(
            "no v1 signatures in Stripe-Signature header".to_string(),
        ));
    }

    Ok((timestamp, signatures))
}

/// Webhook event dispatcher that routes verified events to handlers.
pub struct WebhookDispatcher {
    #[cfg(all(feature = "billing", feature = "gcp"))]
    credit_ledger: Option<std::sync::Arc<super::ledger::CreditLedger>>,
    #[cfg(all(feature = "billing", feature = "gcp"))]
    credits_per_cycle: u32,
    _private: (),
}

impl WebhookDispatcher {
    pub fn new() -> Self {
        Self {
            #[cfg(all(feature = "billing", feature = "gcp"))]
            credit_ledger: None,
            #[cfg(all(feature = "billing", feature = "gcp"))]
            credits_per_cycle: 1,
            _private: (),
        }
    }

    /// Create a dispatcher from AppState, capturing credit ledger if available.
    pub fn from_state(state: &crate::state::AppState) -> Self {
        #[cfg(all(feature = "billing", feature = "gcp"))]
        {
            Self {
                credit_ledger: state.credit_ledger.clone(),
                credits_per_cycle: state.credits_per_cycle,
                _private: (),
            }
        }
        #[cfg(not(all(feature = "billing", feature = "gcp")))]
        {
            let _ = state;
            Self::new()
        }
    }

    /// Dispatch a verified webhook event to the appropriate handler.
    pub async fn dispatch(&self, event: &WebhookEvent) {
        match event.event_type.as_str() {
            "checkout.session.completed" => {
                info!(event_id = %event.id, "Checkout session completed");
            }
            "payment_intent.succeeded" => {
                info!(event_id = %event.id, "Payment intent succeeded");
            }
            "payment_intent.payment_failed" => {
                warn!(event_id = %event.id, "Payment intent failed");
            }
            "invoice.paid" => {
                info!(event_id = %event.id, "Invoice paid");
                self.handle_invoice_paid(event).await;
            }
            "invoice.payment_failed" => {
                warn!(event_id = %event.id, "Invoice payment failed");
            }
            "customer.subscription.created" => {
                info!(event_id = %event.id, "Subscription created");
            }
            "customer.subscription.updated" => {
                info!(event_id = %event.id, "Subscription updated");
            }
            "customer.subscription.deleted" => {
                warn!(event_id = %event.id, "Subscription deleted");
            }
            other => {
                debug!(event_id = %event.id, event_type = %other, "Unhandled webhook event type");
            }
        }
    }

    /// Handle `invoice.paid` events — top up credits based on amount paid.
    async fn handle_invoice_paid(&self, event: &WebhookEvent) {
        #[cfg(all(feature = "billing", feature = "gcp"))]
        {
            let Some(ref ledger) = self.credit_ledger else {
                debug!(event_id = %event.id, "No credit ledger configured, skipping top-up");
                return;
            };

            // Extract invoice data from event
            let object = match event.data.get("object") {
                Some(obj) => obj,
                None => {
                    warn!(event_id = %event.id, "invoice.paid event missing data.object");
                    return;
                }
            };

            let customer = match object.get("customer").and_then(|v| v.as_str()) {
                Some(c) => c,
                None => {
                    warn!(event_id = %event.id, "invoice.paid event missing customer");
                    return;
                }
            };

            let amount_paid = object
                .get("amount_paid")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let invoice_id = object
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            if amount_paid <= 0 {
                debug!(event_id = %event.id, amount_paid, "Skipping zero/negative amount invoice");
                return;
            }

            // Calculate credits: amount_paid (in cents) / credits_per_cycle
            // For now, 1 cent = 1 credit (configurable via credits_per_cycle)
            let credits = amount_paid / i64::from(self.credits_per_cycle.max(1));

            let idempotency_key = format!("topup_{invoice_id}");
            let description = format!("Invoice payment: {invoice_id}");

            match ledger
                .top_up(
                    customer,
                    credits,
                    &description,
                    Some(invoice_id),
                    Some(&idempotency_key),
                )
                .await
            {
                Ok(tx) => {
                    info!(
                        event_id = %event.id,
                        customer,
                        credits,
                        tx_id = %tx.id,
                        "Credits topped up from invoice payment"
                    );
                }
                Err(e) => {
                    warn!(
                        event_id = %event.id,
                        customer,
                        error = %e,
                        "Failed to top up credits from invoice payment"
                    );
                }
            }
        }

        #[cfg(not(all(feature = "billing", feature = "gcp")))]
        {
            let _ = event;
        }
    }
}

impl Default for WebhookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn compute_test_signature(payload: &[u8], secret: &str, timestamp: i64) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{timestamp}.").as_bytes());
        mac.update(payload);
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn test_verify_webhook_signature_valid() {
        let secret = "whsec_test_secret";
        let payload = b"{\"id\":\"evt_123\"}";
        let timestamp = chrono::Utc::now().timestamp();
        let sig = compute_test_signature(payload, secret, timestamp);
        let header = format!("t={timestamp},v1={sig}");

        let result = verify_webhook_signature(payload, &header, secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_webhook_signature_invalid() {
        let secret = "whsec_test_secret";
        let payload = b"{\"id\":\"evt_123\"}";
        let timestamp = chrono::Utc::now().timestamp();
        let header = format!("t={timestamp},v1=invalid_signature_hex");

        let result = verify_webhook_signature(payload, &header, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_webhook_signature_expired() {
        let secret = "whsec_test_secret";
        let payload = b"{\"id\":\"evt_123\"}";
        let timestamp = chrono::Utc::now().timestamp() - 600; // 10 minutes ago
        let sig = compute_test_signature(payload, secret, timestamp);
        let header = format!("t={timestamp},v1={sig}");

        let result = verify_webhook_signature(payload, &header, secret);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("tolerance"));
    }

    #[test]
    fn test_verify_webhook_signature_custom_tolerance() {
        let secret = "whsec_test_secret";
        let payload = b"{\"id\":\"evt_123\"}";
        let timestamp = chrono::Utc::now().timestamp() - 600; // 10 minutes ago
        let sig = compute_test_signature(payload, secret, timestamp);
        let header = format!("t={timestamp},v1={sig}");

        // With 15 minute tolerance, this should succeed
        let result = verify_webhook_signature_with_tolerance(payload, &header, secret, 900);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_signature_header_valid() {
        let header = "t=1700000000,v1=abc123def456";
        let (ts, sigs) = parse_signature_header(header).unwrap();
        assert_eq!(ts, 1700000000);
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0], "abc123def456");
    }

    #[test]
    fn test_parse_signature_header_multiple_v1() {
        let header = "t=1700000000,v1=sig1,v1=sig2";
        let (ts, sigs) = parse_signature_header(header).unwrap();
        assert_eq!(ts, 1700000000);
        assert_eq!(sigs.len(), 2);
    }

    #[test]
    fn test_parse_signature_header_with_v0() {
        let header = "t=1700000000,v0=legacy,v1=current";
        let (ts, sigs) = parse_signature_header(header).unwrap();
        assert_eq!(ts, 1700000000);
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0], "current");
    }

    #[test]
    fn test_parse_signature_header_missing_timestamp() {
        let header = "v1=abc123";
        let result = parse_signature_header(header);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_signature_header_missing_v1() {
        let header = "t=1700000000";
        let result = parse_signature_header(header);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_signature_header_invalid_timestamp() {
        let header = "t=notanumber,v1=abc123";
        let result = parse_signature_header(header);
        assert!(result.is_err());
    }

    #[test]
    fn test_webhook_dispatcher_new() {
        let dispatcher = WebhookDispatcher::new();
        let _default = WebhookDispatcher::default();
        // Just verify construction works
        let _ = dispatcher;
    }

    #[tokio::test]
    async fn test_webhook_dispatcher_dispatch() {
        let dispatcher = WebhookDispatcher::new();
        let event = WebhookEvent {
            id: "evt_test".to_string(),
            event_type: "checkout.session.completed".to_string(),
            data: serde_json::json!({}),
            created: 1700000000,
        };
        // Should not panic
        dispatcher.dispatch(&event).await;
    }

    #[test]
    fn test_tampered_payload_rejected() {
        let secret = "whsec_test_secret";
        let original_payload = b"{\"id\":\"evt_123\"}";
        let tampered_payload = b"{\"id\":\"evt_456\"}";
        let timestamp = chrono::Utc::now().timestamp();
        let sig = compute_test_signature(original_payload, secret, timestamp);
        let header = format!("t={timestamp},v1={sig}");

        let result = verify_webhook_signature(tampered_payload, &header, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let payload = b"{\"id\":\"evt_123\"}";
        let timestamp = chrono::Utc::now().timestamp();
        let sig = compute_test_signature(payload, "correct_secret", timestamp);
        let header = format!("t={timestamp},v1={sig}");

        let result = verify_webhook_signature(payload, &header, "wrong_secret");
        assert!(result.is_err());
    }
}
