// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Seven deterministic agents for the Budget Approval Decision spike.
//!
//! Each agent follows the converge contract:
//! - Declare dependencies (which context keys it cares about)
//! - Pure `accepts()` check
//! - Immutable `execute()` returning `AgentEffect`

#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};

// ---------------------------------------------------------------------------
// [1] IntentDecompositionAgent
// ---------------------------------------------------------------------------

/// Decomposes the OrganismIntent into structured seed facts.
///
/// Truth #5: Intent is explicit and bounded — seeds carry amount, department,
/// purpose, reversibility, and expiry.
pub struct IntentDecompositionAgent {
    pub amount: u64,
    pub department: String,
    pub purpose: String,
    pub channel: String,
    pub reversibility: String,
}

impl Agent for IntentDecompositionAgent {
    fn name(&self) -> &str {
        "IntentDecompositionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        !ctx.has(ContextKey::Seeds)
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        AgentEffect::with_facts(vec![
            Fact::new(ContextKey::Seeds, "budget:amount", self.amount.to_string()),
            Fact::new(
                ContextKey::Seeds,
                "budget:department",
                self.department.clone(),
            ),
            Fact::new(ContextKey::Seeds, "budget:purpose", self.purpose.clone()),
            Fact::new(ContextKey::Seeds, "budget:channel", self.channel.clone()),
            Fact::new(
                ContextKey::Seeds,
                "budget:reversibility",
                self.reversibility.clone(),
            ),
        ])
    }
}

// ---------------------------------------------------------------------------
// [2] AuthorityVerificationAgent
// ---------------------------------------------------------------------------

/// Verifies department delegation authority against the requested amount.
///
/// Truth #2: Authority re-derived at commit — emits authority status and
/// remaining budget as signals.
pub struct AuthorityVerificationAgent {
    pub delegation_limit: u64,
}

impl Agent for AuthorityVerificationAgent {
    fn name(&self) -> &str {
        "AuthorityVerificationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|s| s.id.starts_with("authority:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);

        let amount: u64 = seeds
            .iter()
            .find(|s| s.id == "budget:amount")
            .and_then(|s| s.content.parse().ok())
            .unwrap_or(0);

        let within_authority = amount <= self.delegation_limit;
        let remaining = self.delegation_limit.saturating_sub(amount);

        AgentEffect::with_facts(vec![
            Fact::new(
                ContextKey::Signals,
                "authority:status",
                if within_authority {
                    "delegated"
                } else {
                    "exceeds_delegation"
                },
            ),
            Fact::new(
                ContextKey::Signals,
                "authority:remaining",
                remaining.to_string(),
            ),
        ])
    }
}

// ---------------------------------------------------------------------------
// [3] PlanningAgent
// ---------------------------------------------------------------------------

/// Generates 3 candidate strategies: approve, phase, reject.
///
/// Truth #4: Argue better, not faster — 3 candidates debate;
/// revision responds to challenge evidence.
pub struct PlanningAgent;

impl Agent for PlanningAgent {
    fn name(&self) -> &str {
        "PlanningAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals, ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id == "authority:status")
            && !ctx.has(ContextKey::Strategies)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let amount: u64 = seeds
            .iter()
            .find(|s| s.id == "budget:amount")
            .and_then(|s| s.content.parse().ok())
            .unwrap_or(0);

        let channel = seeds
            .iter()
            .find(|s| s.id == "budget:channel")
            .map_or("unknown", |s| &s.content);

        let half = amount / 2;

        AgentEffect::with_facts(vec![
            Fact::new(
                ContextKey::Strategies,
                "strategy:approve",
                format!(
                    r#"{{"action":"approve","total_cost":{amount},"description":"Full approval: ${amount} on {channel} in Q2","expected_roi":"3.2x based on industry benchmarks"}}"#
                ),
            ),
            Fact::new(
                ContextKey::Strategies,
                "strategy:phase",
                format!(
                    r#"{{"action":"phase","total_cost":{half},"description":"Phased: ${half} in Q2, rest contingent on Q2 ROI > 2x","expected_roi":"2.8x conservative estimate"}}"#
                ),
            ),
            Fact::new(
                ContextKey::Strategies,
                "strategy:reject",
                r#"{"action":"reject","total_cost":0,"description":"Reject: defer to Q3 pending market analysis","expected_roi":"N/A"}"#,
            ),
        ])
    }
}

// ---------------------------------------------------------------------------
// [4] AdversarialAgent
// ---------------------------------------------------------------------------

/// Structurally blocks convergence until ROI challenge is addressed.
///
/// Truth #3: Adversarial challenge is institutional — this agent
/// emits a blocking challenge as a constraint fact.
pub struct BudgetAdversarialAgent;

impl Agent for BudgetAdversarialAgent {
    fn name(&self) -> &str {
        "BudgetAdversarialAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Fire when strategies exist but no challenge yet
        ctx.has(ContextKey::Strategies)
            && !ctx
                .get(ContextKey::Constraints)
                .iter()
                .any(|c| c.id.starts_with("challenge:"))
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);

        // Check if any strategy claims ROI > 3x — challenge it
        let has_optimistic_roi = strategies.iter().any(|s| s.content.contains("3.2x"));

        if has_optimistic_roi {
            let challenge = organism_core::adversarial::Challenge {
                skepticism: organism_core::adversarial::SkepticismKind::EconomicSkepticism,
                target: "strategy:approve ROI projection".into(),
                description: "ROI assumption of 3.2x is based on industry benchmarks that include \
                             B2C campaigns. LinkedIn B2B typically yields 1.5-2.2x ROI."
                    .into(),
                severity: organism_core::adversarial::ChallengeSeverity::Blocking,
                evidence: vec![
                    "LinkedIn B2B benchmark: median 1.8x ROI (2024 data)".into(),
                    "Company historical LinkedIn ROAS: 1.6x (last 4 quarters)".into(),
                ],
                suggestion: Some(
                    "Use conservative 1.8x ROI estimate; phase spending to validate".into(),
                ),
            };

            let challenge_json = serde_json::to_string(&challenge).expect("challenge serializes");

            AgentEffect::with_fact(Fact::new(
                ContextKey::Constraints,
                "challenge:roi-assumption",
                challenge_json,
            ))
        } else {
            AgentEffect::empty()
        }
    }
}

// ---------------------------------------------------------------------------
// [5] PlanRevisionAgent
// ---------------------------------------------------------------------------

/// Revises strategies in response to adversarial challenges.
///
/// Truth #4: Argue better — the revised plan addresses the specific
/// evidence raised by the adversarial agent.
pub struct PlanRevisionAgent;

impl Agent for PlanRevisionAgent {
    fn name(&self) -> &str {
        "PlanRevisionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Constraints, ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run when a challenge exists but no revised strategy yet
        let has_challenge = ctx
            .get(ContextKey::Constraints)
            .iter()
            .any(|c| c.id.starts_with("challenge:"));
        let has_revision = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|s| s.id.starts_with("revised:"));

        has_challenge && !has_revision
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let amount: u64 = seeds
            .iter()
            .find(|s| s.id == "budget:amount")
            .and_then(|s| s.content.parse().ok())
            .unwrap_or(0);

        // Build a revised strategy that addresses the ROI challenge
        // Use conservative ROI (1.8x) and phase the spending
        let phase1 = amount / 2;
        let phase2 = amount - phase1;

        AgentEffect::with_fact(Fact::new(
            ContextKey::Strategies,
            "revised:phased-conservative",
            format!(
                r#"{{"action":"approve_phased","total_cost":{amount},"description":"Revised: ${phase1} Q2 phase 1, ${phase2} Q2 phase 2 contingent on 1.5x ROI gate","expected_roi":"1.8x (conservative, addresses challenge)","addresses_challenge":"challenge:roi-assumption","phase1_amount":{phase1},"phase2_gate":"ROI >= 1.5x after 30 days"}}"#
            ),
        ))
    }
}

// ---------------------------------------------------------------------------
// [6] SimulationAgent
// ---------------------------------------------------------------------------

/// Evaluates strategies along cost, outcome, and policy dimensions.
///
/// Truth #8: Not a simulation of human firm — parallel evaluation
/// at machine speed, no approval chain.
pub struct SimulationAgent;

impl Agent for SimulationAgent {
    fn name(&self) -> &str {
        "SimulationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run when revised strategy exists but no simulation evaluations yet
        let has_revision = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|s| s.id.starts_with("revised:"));
        let has_simulation = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with("sim:"));

        has_revision && !has_simulation
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);

        let mut facts = Vec::new();

        for strategy in strategies {
            let (cost_score, outcome_score, policy_score) = evaluate_strategy(strategy);
            let total = (cost_score + outcome_score + policy_score) / 3;

            facts.push(Fact::new(
                ContextKey::Evaluations,
                format!("sim:{}", strategy.id),
                format!(
                    r#"{{"strategy":"{}","cost_score":{},"outcome_score":{},"policy_score":{},"total_score":{},"confidence":0.85}}"#,
                    strategy.id, cost_score, outcome_score, policy_score, total
                ),
            ));
        }

        AgentEffect::with_facts(facts)
    }
}

/// Deterministic strategy evaluation.
fn evaluate_strategy(strategy: &Fact) -> (u32, u32, u32) {
    let content = &strategy.content;

    if strategy.id.starts_with("revised:") {
        // Revised strategies score highest — they address challenges
        (85, 80, 95)
    } else if content.contains("\"action\":\"approve\"") {
        // Full approval: good outcome but challenged ROI
        (70, 75, 80)
    } else if content.contains("\"action\":\"phase\"") {
        // Original phased: decent but doesn't address challenge
        (80, 70, 85)
    } else if content.contains("\"action\":\"reject\"") {
        // Reject: safe but no outcome
        (95, 30, 90)
    } else {
        (50, 50, 50)
    }
}

// ---------------------------------------------------------------------------
// [7] DecisionAgent
// ---------------------------------------------------------------------------

/// Produces the final decision recommendation.
///
/// Truth #1: Hierarchy was a workaround — no manager approves;
/// agents converge on a decision based on simulation scores.
/// Truth #7: Trustworthy execution of intent — the decision traces
/// back to seeds, challenges, and evaluations.
pub struct DecisionAgent;

impl Agent for DecisionAgent {
    fn name(&self) -> &str {
        "DecisionAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run when simulation evaluations exist but no decision yet
        let has_simulations = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with("sim:"));
        let has_decision = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with("decision:"));

        has_simulations && !has_decision
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);

        // Find the highest-scoring strategy
        let best = evaluations
            .iter()
            .filter(|e| e.id.starts_with("sim:"))
            .max_by_key(|e| {
                // Extract total_score from JSON content
                e.content
                    .split("\"total_score\":")
                    .nth(1)
                    .and_then(|s| s.split([',', '}']).next())
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(0)
            });

        let (recommendation, strategy_id, rationale) = match best {
            Some(eval) => {
                let strategy_ref = eval.id.strip_prefix("sim:").unwrap_or(&eval.id);
                if strategy_ref.starts_with("revised:") {
                    (
                        "approve_phased",
                        strategy_ref,
                        "Revised phased strategy addresses ROI challenge with conservative estimates and gate mechanism",
                    )
                } else if strategy_ref.starts_with("strategy:approve") {
                    (
                        "approve",
                        strategy_ref,
                        "Full approval based on positive evaluation",
                    )
                } else {
                    ("defer", strategy_ref, "Further analysis recommended")
                }
            }
            None => ("defer", "none", "No strategies evaluated"),
        };

        AgentEffect::with_fact(Fact::new(
            ContextKey::Evaluations,
            "decision:recommendation",
            format!(
                r#"{{"recommendation":"{recommendation}","strategy":"{strategy_id}","rationale":"{rationale}","confidence":0.87}}"#
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_decomposition_emits_seeds() {
        let agent = IntentDecompositionAgent {
            amount: 50_000,
            department: "marketing".into(),
            purpose: "Q2 campaign".into(),
            channel: "LinkedIn".into(),
            reversibility: "Partial".into(),
        };

        let ctx = Context::new();
        assert!(agent.accepts(&ctx));

        let effect = agent.execute(&ctx);
        assert_eq!(effect.facts.len(), 5);
        assert!(effect.facts.iter().all(|f| f.key == ContextKey::Seeds));
    }

    #[test]
    fn authority_verification_checks_delegation() {
        let agent = AuthorityVerificationAgent {
            delegation_limit: 100_000,
        };

        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(ContextKey::Seeds, "budget:amount", "50000"))
            .unwrap();

        assert!(agent.accepts(&ctx));
        let effect = agent.execute(&ctx);
        assert_eq!(effect.facts.len(), 2);

        let status = effect
            .facts
            .iter()
            .find(|f| f.id == "authority:status")
            .unwrap();
        assert_eq!(status.content, "delegated");
    }

    #[test]
    fn adversarial_agent_challenges_optimistic_roi() {
        let agent = BudgetAdversarialAgent;

        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strategy:approve",
            r#"{"expected_roi":"3.2x"}"#,
        ))
        .unwrap();

        assert!(agent.accepts(&ctx));
        let effect = agent.execute(&ctx);
        assert_eq!(effect.facts.len(), 1);
        assert!(effect.facts[0].id.starts_with("challenge:"));
    }
}
