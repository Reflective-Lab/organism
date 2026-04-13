// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: LicenseRef-Proprietary
// All rights reserved.

//! # Organism Core
//!
//! The organizational intelligence runtime, built on top of Converge.
//!
//! Converge defines the laws of physics — deterministic execution, authority,
//! append-only truth, convergence. Organism defines the living system running
//! on top — intent decoding, planning huddles, adversarial agents, simulation
//! swarms, adaptive strategies, organizational evolution.
//!
//! ## Relationship to Converge
//!
//! organism-core **extends** converge-core. It does not duplicate it.
//!
//! - [`intent`] wraps [`converge_core::RootIntent`] with organism-specific
//!   envelope fields (reversibility, expiry, forbidden actions)
//! - [`planning`] produces [`converge_core::types::Proposal`] candidates
//!   that flow through converge's PromotionGate
//! - [`adversarial`] implements the [`converge_core::Agent`] trait as
//!   specialized skeptic agents
//! - [`simulation`] stress-tests plans before they reach the commit boundary
//! - [`commit`] implements organism-specific [`converge_core::Invariant`]
//!   checks for non-monotonic authority re-verification
//! - [`learning`] captures adversarial firings as labeled training signals
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │ Organism (this crate)                           │
//! │ Intent decoding, planning, adversarial,         │
//! │ simulation, organizational learning             │
//! ├─────────────────────────────────────────────────┤
//! │ Converge (converge-core)                        │
//! │ Engine, Context, Agent, Invariant, Proposal,    │
//! │ Fact, PromotionGate, Authority, Budget          │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! Converge is stable, deterministic, mathematical.
//! Organism is emergent, experimental, adaptive.

pub mod intent;
pub mod planning;
pub mod adversarial;
pub mod simulation;
pub mod commit;
pub mod learning;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
