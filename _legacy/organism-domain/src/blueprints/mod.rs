// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: MIT

//! Workflow Blueprints - Pre-configured agent compositions for common business workflows.
//!
//! Blueprints provide ready-to-use configurations that combine agents from multiple packs
//! to implement end-to-end business processes. Each blueprint defines:
//!
//! - The agents involved and their execution order
//! - Cross-pack invariants that must hold
//! - Trigger conditions for pack transitions
//! - Expected inputs and outputs
//!
//! # Available Blueprints
//!
//! - [`lead_to_cash`]: Full revenue cycle from lead to payment collection
//! - [`hire_to_retire`]: Complete employee lifecycle
//! - [`procure_to_pay`]: Procurement through payment processing
//! - [`idea_to_launch`]: Product development from concept to release
//! - [`issue_to_resolution`]: Support ticket lifecycle with knowledge capture
//! - [`campaign_to_revenue`]: Marketing campaign to closed deal
//! - [`partner_to_value`]: Partnership lifecycle to revenue realization
//! - [`patent_research`]: Patent research with governed evidence and approvals
//!
//! # Usage
//!
//! ```rust,ignore
//! use converge_domain::blueprints::lead_to_cash::LeadToCashBlueprint;
//!
//! let blueprint = LeadToCashBlueprint::new();
//! let engine = blueprint.create_engine();
//! let result = engine.run(context)?;
//! ```

pub mod campaign_to_revenue;
pub mod hire_to_retire;
pub mod idea_to_launch;
pub mod issue_to_resolution;
pub mod lead_to_cash;
pub mod partner_to_value;
pub mod patent_research;
pub mod procure_to_pay;

// Re-export blueprints for convenience
pub use campaign_to_revenue::CampaignToRevenueBlueprint;
pub use hire_to_retire::HireToRetireBlueprint;
pub use idea_to_launch::IdeaToLaunchBlueprint;
pub use issue_to_resolution::IssueToResolutionBlueprint;
pub use lead_to_cash::LeadToCashBlueprint;
pub use partner_to_value::PartnerToValueBlueprint;
pub use patent_research::PatentResearchBlueprint;
pub use procure_to_pay::ProcureToPayBlueprint;

use converge_core::{Budget, Engine};

/// Trait for all workflow blueprints
pub trait Blueprint {
    /// Returns the name of this blueprint
    fn name(&self) -> &str;

    /// Returns a description of the workflow
    fn description(&self) -> &str;

    /// Returns the packs involved in this workflow
    fn packs(&self) -> &[&str];

    /// Creates an engine configured with all agents and invariants for this workflow
    fn create_engine(&self) -> Engine;

    /// Creates an engine with a custom budget
    fn create_engine_with_budget(&self, budget: Budget) -> Engine;
}

/// Default budget for blueprints
pub fn default_blueprint_budget() -> Budget {
    Budget {
        max_cycles: 100,
        max_facts: 1000,
    }
}
