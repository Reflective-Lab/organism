// Copyright 2024-2025 Aprio One AB, Sweden
// Author: Kenneth Pernyer, kenneth@aprio.one
// SPDX-License-Identifier: LicenseRef-Proprietary
// All rights reserved.

//! # Organism Application
//!
//! Business-level applications built on the Organism organizational runtime.
//!
//! This crate contains application-level tools that use Organism's planning,
//! adversarial, and execution capabilities for specific end-user scenarios.
//!
//! ## Applications
//!
//! - [`formfiller`]: Form completion automation (TUI + WebDriver)
//! - [`course_planner`]: University course application planner (PDF-first)
//!
//! ## Application Agents
//!
//! Agent implementations that were previously in converge-application but
//! belong at the organism layer:
//!
//! - [`agents::strategic_insight`]: LLM-powered strategic synthesis
//! - [`agents::risk_assessment`]: Risk assessment agent

pub mod agents;
pub mod spike;
pub mod spike_capacity;
pub mod spike_market;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        // Proves the crate structure is valid
    }
}
