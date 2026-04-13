// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Billing — Stripe ACP integration for SaaS products.
//!
//! Moved from converge-runtime. This is product business logic, not
//! kernel infrastructure. No convergence loop needs Stripe webhooks.
//!
//! - `types` — Stripe ACP types (checkout sessions, line items, webhooks)
//! - `client` — HTTP client for Stripe API
//! - `ledger` — Payment event ledger
//! - `webhook` — Stripe webhook handler
//!
//! Migration source: `converge-runtime/src/billing/` (2,010 lines).
//! Files are copied from source. They need import refactoring to
//! remove `crate::config::BillingConfig` and `utoipa::ToSchema` deps.

pub mod types;
// client, ledger, webhook need import refactoring before they compile:
// - Remove crate::config::BillingConfig (define locally or inject)
// - Remove utoipa::ToSchema (not needed outside the HTTP layer)
// pub mod client;
// pub mod ledger;
// pub mod webhook;
