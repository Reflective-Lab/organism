// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Adversarial agents — converge Agents specialized for plan challenge.
//!
//! Adversarial agents implement the [`converge_core::Agent`] trait. They
//! participate in the standard convergence loop but their purpose is to
//! emit challenges (as Facts) that force plan revision.
//!
//! ## The Debate Loop
//!
//! 1. Planning agents propose candidate plans (as Proposals)
//! 2. Adversarial agents run in the same engine cycle
//! 3. They emit challenge Facts that block convergence
//! 4. Planning agents revise in response
//! 5. Cycle repeats until challenges are resolved or budget exhausted
//!
//! This is standard converge semantics — no special mechanism needed.
//! The engine's fixed-point detection handles the debate loop naturally.
//!
//! ## Second-Order Effect
//!
//! When an adversarial agent fires, the before/after pair becomes a
//! labeled training signal: "this assumption type fails in this context."
//! Over time the planning priors get calibrated. This is compounding value.

use serde::{Deserialize, Serialize};

/// A challenge emitted by an adversarial agent.
///
/// Challenges are stored as structured content inside converge Facts
/// (under ContextKey::Constraints). The engine treats them like any
/// other fact — they block convergence until resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// What kind of skepticism produced this challenge.
    pub skepticism: SkepticismKind,
    /// What aspect of the plan is being challenged.
    pub target: String,
    /// Description of the challenge.
    pub description: String,
    /// Severity of the challenge.
    pub severity: ChallengeSeverity,
    /// Evidence or reasoning supporting the challenge.
    pub evidence: Vec<String>,
    /// Suggested remediation.
    pub suggestion: Option<String>,
}

/// Kinds of skepticism that adversarial agents apply.
///
/// These are not agent types — they are reasoning modes. A single
/// adversarial agent implementation may apply multiple skepticism kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkepticismKind {
    /// Surfaces hidden premises and unstated assumptions.
    AssumptionBreaking,
    /// Finds policy, authority, and constraint violations.
    ConstraintChecking,
    /// Challenges correlations — cohort bias, seasonality, confounders.
    CausalSkepticism,
    /// Attacks ROI assumptions, cost projections, economic reasoning.
    EconomicSkepticism,
    /// Checks execution feasibility — team capacity, system readiness.
    OperationalSkepticism,
}

/// How severe a challenge is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeSeverity {
    /// Worth noting but not blocking convergence.
    Advisory,
    /// Should be addressed — emitted as a constraint fact.
    Warning,
    /// Must be resolved before convergence — blocks fixed-point.
    Blocking,
}

/// A labeled training signal from adversarial firings.
///
/// When an adversarial agent forces a plan revision, the challenge
/// becomes a learning signal that calibrates future planning priors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialSignal {
    /// What kind of skepticism fired.
    pub skepticism: SkepticismKind,
    /// The challenge that caused revision.
    pub challenge: Challenge,
    /// The assumption type that failed.
    pub assumption_type: String,
    /// Context in which it failed.
    pub failure_context: String,
    /// How the plan was revised in response.
    pub revision_summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_roundtrips() {
        let challenge = Challenge {
            skepticism: SkepticismKind::EconomicSkepticism,
            target: "Plan A, step 3: ROI projection".into(),
            description: "Revenue projection assumes 100% conversion".into(),
            severity: ChallengeSeverity::Blocking,
            evidence: vec!["Historical conversion is 12%".into()],
            suggestion: Some("Use conservative 10% conversion estimate".into()),
        };

        let json = serde_json::to_string(&challenge).unwrap();
        let _: Challenge = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn adversarial_signal_roundtrips() {
        let signal = AdversarialSignal {
            skepticism: SkepticismKind::CausalSkepticism,
            challenge: Challenge {
                skepticism: SkepticismKind::CausalSkepticism,
                target: "correlation claim".into(),
                description: "Seasonal confound not accounted for".into(),
                severity: ChallengeSeverity::Warning,
                evidence: vec![],
                suggestion: None,
            },
            assumption_type: "causal_attribution".into(),
            failure_context: "q4_seasonal_revenue".into(),
            revision_summary: "Added seasonal adjustment to forecast model".into(),
        };

        let json = serde_json::to_string(&signal).unwrap();
        let _: AdversarialSignal = serde_json::from_str(&json).unwrap();
    }
}
