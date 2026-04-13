// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: LicenseRef-Proprietary
// All rights reserved.

//! # Organism Domain
//!
//! Business domain packs, blueprints, and use cases for the Organism
//! organizational intelligence runtime.
//!
//! This crate contains the business-specific logic that sits above the
//! Converge kernel. Where `converge-domain` holds kernel packs (money, trust,
//! delivery, knowledge), this crate holds organizational packs that are
//! business-specific, configurable, and evolve with company strategy.
//!
//! # Packs (Organizational Business Domains)
//!
//! - [`packs::autonomous_org`]: Governance automation (Policy, Approval, Budget, Exception, Delegation)
//! - [`packs::customers`]: Revenue operations (Lead → Qualify → Offer → Close → Handoff)
//! - [`packs::people`]: People lifecycle (Hire → Onboard → Pay → Review → Offboard)
//! - [`packs::performance`]: Performance management (Plan → Feedback → Review → Calibrate)
//! - [`packs::growth_marketing`]: Customer lifecycle (Acquire → Activate → Retain → Refer)
//! - [`packs::legal`]: Contracts, equity, IP governance (Draft → Sign → Execute)
//! - [`packs::product_engineering`]: Product development (Plan → Build → Release → Observe)
//! - [`packs::ops_support`]: Universal intake (Intake → Triage → Resolve → Prevent)
//! - [`packs::procurement_assets`]: Purchase management (Request → Approve → Buy → Track)
//! - [`packs::partnerships_vendors`]: External relationships (Source → Evaluate → Contract → Operate)
//! - [`packs::virtual_teams`]: Team composition (Form → Operate → Publish → Audit)
//! - [`packs::linkedin_research`]: Research signals (Signals → Evidence → Dossier → Approval)
//! - [`packs::reskilling`]: Skills and learning (Assess → Plan → Learn → Certify)
//!
//! # Use Cases (Applied Domain Agents)
//!
//! - [`use_cases::growth_strategy`]: Growth strategy pipeline for market analysis
//! - [`use_cases::sdr_sales`]: SDR sales qualification and outreach
//! - [`use_cases::patent_research`]: Patent landscape analysis and claim strategy
//! - [`use_cases::release_readiness`]: Release quality gates
//! - [`use_cases::compliance_monitoring`]: Continuous compliance monitoring
//! - [`use_cases::crm_account_health`]: CRM account health and growth strategy
//! - [`use_cases::hr_policy_alignment`]: HR policy alignment
//! - [`use_cases::catalog_enrichment`]: Catalog update and enrichment
//! - [`use_cases::inventory_rebalancing`]: Multi-region inventory rebalancing
//! - [`use_cases::strategic_sourcing`]: Vendor selection and sourcing
//! - [`use_cases::supply_chain`]: Supply chain re-planning
//! - [`use_cases::travel`]: Travel booking and management
//!
//! # Blueprints (Multi-Pack Workflows)
//!
//! - [`blueprints::lead_to_cash`]: Full revenue cycle (4-pack orchestration)
//! - [`blueprints::hire_to_retire`]: Employee lifecycle (3-pack orchestration)
//! - [`blueprints::procure_to_pay`]: Procurement cycle
//! - [`blueprints::issue_to_resolution`]: Support resolution
//! - [`blueprints::idea_to_launch`]: Product launch
//! - [`blueprints::campaign_to_revenue`]: Marketing to revenue
//! - [`blueprints::partner_to_value`]: Partnership lifecycle
//! - [`blueprints::patent_research`]: IP research pipeline

pub mod evals;
pub mod packs;
pub mod blueprints;
pub mod use_cases;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        // Proves the crate structure is valid
    }
}
