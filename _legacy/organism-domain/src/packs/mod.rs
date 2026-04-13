// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Organism domain packs — business-specific domain logic.
//!
//! These packs sit above the Converge kernel packs (money, trust, delivery,
//! knowledge) and contain organizational logic that is business-specific,
//! configurable, and evolves with company strategy.
//!
//! # Kernel vs Organism Packs
//!
//! | Kernel (converge-domain) | Organism (this crate) |
//! |--------------------------|----------------------|
//! | Money: immutable transactions | Customers: lead scoring, ICP-specific |
//! | Trust: audit, provenance | Legal: contract templates, org-specific |
//! | Delivery: promise fulfillment | Product Engineering: feature/release mgmt |
//! | Knowledge: signal→decision | Growth Marketing: campaigns, channels |
//! | Data Metrics: instrumentation | Performance: review cycles, calibration |
//!
//! # Design Principle
//!
//! Organism packs contain logic that:
//! - Changes with business strategy (lead scoring, budget thresholds)
//! - Varies by company, vertical, or GTM model
//! - Requires domain expertise to configure correctly
//! - Is often customized per deployment
//!
//! # Fact ID Prefixes (Organism-specific)
//!
//! - Autonomous Org: `policy:`, `approval:`, `budget_envelope:`, `exception:`, `delegation:`
//! - Customers: `lead:`, `opportunity:`, `proposal:`, `deal:`, `handoff:`
//! - People: `employee:`, `identity:`, `access:`, `payroll:`, `expense:`
//! - Performance: `review_cycle:`, `goal:`, `improvement_plan:`, `feedback:`
//! - Growth Marketing: `campaign:`, `channel:`, `content:`, `audience:`
//! - Legal: `contract:`, `equity:`, `ip_assignment:`, `board_approval:`
//! - Product Engineering: `initiative:`, `feature:`, `release:`, `incident:`
//! - Ops Support: `ticket:`, `escalation:`, `sla:`, `root_cause:`
//! - Procurement: `request:`, `approval:`, `order:`, `asset:`
//! - Partnerships: `partner:`, `supplier:`, `p_agreement:`, `vendor_assessment:`
//! - Virtual Teams: `team:`, `persona:`, `content_draft:`, `publish:`
//! - LinkedIn Research: `linkedin_signal:`, `linkedin_evidence:`, `linkedin_dossier:`
//! - Reskilling: `skill:`, `learning_plan:`, `credential:`

// TODO: These modules will be populated by moving the corresponding code
// from converge-domain/src/packs/ into this crate.

pub mod autonomous_org;
pub mod customers;
pub mod people;
pub mod performance;
pub mod growth_marketing;
pub mod legal;
pub mod product_engineering;
pub mod ops_support;
pub mod procurement_assets;
pub mod partnerships_vendors;
pub mod virtual_teams;
pub mod linkedin_research;
pub mod reskilling;
