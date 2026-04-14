//! Organizational domain packs for Organism.
//!
//! These packs encode reusable organizational workflow patterns.
//! Each pack defines agents (suggestors), invariants, and fact prefixes
//! for a specific organizational domain.
//!
//! When wired to a Converge engine, each agent implements the `Suggestor`
//! trait from `converge-pack`. The patterns here define the organizational
//! logic; Converge enforces the axioms.
//!
//! # Packs
//!
//! ## Knowledge lifecycle (from converge-domain)
//! - [`packs::knowledge`] — Signal → Hypothesis → Experiment → Decision → Canonical
//!
//! ## Organizational workflows (from _legacy/organism-domain)
//! - [`packs::customers`] — Revenue operations: Lead → Close → Handoff
//! - [`packs::people`] — People lifecycle: Hire → Onboard → Pay → Offboard
//! - [`packs::legal`] — Contracts, equity, IP governance
//! - [`packs::performance`] — Reviews, goals, improvement plans
//! - [`packs::autonomous_org`] — Governance, policies, budgets, delegations
//! - [`packs::growth_marketing`] — Campaigns, channels, attribution
//! - [`packs::product_engineering`] — Roadmaps, features, releases, incidents
//! - [`packs::ops_support`] — Ticket intake, triage, escalation, SLA
//! - [`packs::procurement`] — Purchase requests, assets, subscriptions
//! - [`packs::partnerships`] — Vendor sourcing, evaluation, contracting
//! - [`packs::virtual_teams`] — Team formation, personas, content publishing
//! - [`packs::linkedin_research`] — Signal extraction, dossier building
//! - [`packs::reskilling`] — Skills assessment, learning plans, credentials
//! - [`packs::due_diligence`] — Convergent research, fact extraction, gap detection, synthesis
//!
//! # Blueprints
//!
//! Multi-pack orchestrations composing packs into end-to-end workflows:
//! - [`blueprints::lead_to_cash`] — Customers → Delivery → Legal → Money
//! - [`blueprints::hire_to_retire`] — Legal → People → Trust → Money
//! - [`blueprints::procure_to_pay`] — Procurement → Legal → Money
//! - [`blueprints::issue_to_resolution`] — Ops Support → Knowledge
//! - [`blueprints::idea_to_launch`] — Product Engineering → Delivery
//! - [`blueprints::campaign_to_revenue`] — Growth Marketing → Customers → Money
//! - [`blueprints::partner_to_value`] — Partnerships → Legal → Delivery
//! - [`blueprints::patent_research`] — Knowledge → Legal → IP pipeline
//! - [`blueprints::diligence_to_decision`] — DueDiligence → Legal → Knowledge

pub mod blueprints;
pub mod pack;
pub mod packs;
