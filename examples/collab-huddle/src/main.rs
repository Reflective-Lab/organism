// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Huddle — strict, round-robin planning with synthesis checkpoints.
//!
//! A huddle is the tightest collaboration shape: enforced discipline,
//! explicit turns, mandatory dissent mapping, and a done-gate vote
//! before the team can declare convergence.
//!
//! This example assembles a due-diligence research team and shows:
//! - Curated team formation with explicit roles
//! - Round-robin turn cadence
//! - Majority consensus for the done gate
//! - Validation rejects teams that violate the charter

use organism_pack::{
    CollaborationCharter, CollaborationMember, CollaborationRole, ConsensusRule, TeamFormation,
    TeamFormationMode,
};
use organism_runtime::{CollaborationParticipant, CollaborationRunner};

#[derive(Debug, Clone)]
struct Agent {
    id: String,
    name: String,
    role: CollaborationRole,
    model: String,
}

impl CollaborationParticipant for Agent {
    fn id(&self) -> &str {
        &self.id
    }
    fn display_name(&self) -> &str {
        &self.name
    }
    fn role(&self) -> CollaborationRole {
        self.role
    }
}

fn main() {
    println!("=== Huddle: Due Diligence Research Team ===\n");

    // The charter defines the rules before any team is assembled.
    let charter = CollaborationCharter::huddle();

    println!("Charter:");
    println!("  Topology:       {:?}", charter.topology);
    println!("  Discipline:     {:?}", charter.discipline);
    println!("  Turn cadence:   {:?}", charter.turn_cadence);
    println!("  Consensus:      {:?}", charter.consensus_rule);
    println!("  Min members:    {}", charter.minimum_members);
    println!("  Explicit turns: {}", charter.require_explicit_turns);
    println!("  Round synth:    {}", charter.require_round_synthesis);
    println!("  Dissent map:    {}", charter.require_dissent_map);
    println!("  Done gate:      {}", charter.require_done_gate);
    println!("  Report owner:   {}", charter.require_report_owner);
    println!();

    // Assemble the team — capability-matched, as the huddle charter expects.
    let team = TeamFormation::new(
        TeamFormationMode::CapabilityMatched,
        vec![
            CollaborationMember::new("researcher-lead", "Research Lead", CollaborationRole::Lead)
                .with_persona("Senior analyst who keeps the team focused on evidence quality"),
            CollaborationMember::new(
                "market-analyst",
                "Market Analyst",
                CollaborationRole::Domain,
            )
            .with_persona("Deep expertise in SaaS market sizing and competitive dynamics"),
            CollaborationMember::new("red-team", "Red Team", CollaborationRole::Critic)
                .with_persona("Aggressive skeptic — challenges every assumption and data source"),
            CollaborationMember::new("synthesizer", "Synthesizer", CollaborationRole::Synthesizer)
                .with_persona("Produces structured summaries after each round of debate"),
        ],
    );

    // Runtime participants — in a real system these would be LLM-backed agents.
    let agents = vec![
        Agent {
            id: "researcher-lead".into(),
            name: "Research Lead".into(),
            role: CollaborationRole::Lead,
            model: "claude-sonnet-4-6".into(),
        },
        Agent {
            id: "market-analyst".into(),
            name: "Market Analyst".into(),
            role: CollaborationRole::Domain,
            model: "claude-sonnet-4-6".into(),
        },
        Agent {
            id: "red-team".into(),
            name: "Red Team".into(),
            role: CollaborationRole::Critic,
            model: "claude-opus-4-6".into(),
        },
        Agent {
            id: "synthesizer".into(),
            name: "Synthesizer".into(),
            role: CollaborationRole::Synthesizer,
            model: "claude-sonnet-4-6".into(),
        },
    ];

    // Build the runner — validates charter + team + participants.
    let runner = CollaborationRunner::new(team, charter, agents).expect("team should be valid");

    println!("Team assembled:");
    println!("  Contributors (speak in rounds):");
    for c in runner.contributors() {
        println!("    - {} ({:?}, {})", c.name, c.role, c.model);
    }
    println!("  Voters (done-gate):");
    for v in runner.voters() {
        println!("    - {} ({:?})", v.name, v.role);
    }
    if let Some(owner) = runner.report_owner() {
        println!("  Report owner: {} ({:?})", owner.name, owner.role);
    }
    println!();

    // Simulate a done-gate vote.
    println!("--- Done Gate Vote ---");
    let votes = [
        ("Research Lead", true),
        ("Market Analyst", true),
        ("Red Team", false),
    ];
    let yes = votes.iter().filter(|(_, v)| *v).count();
    let total = votes.len();
    for (name, vote) in &votes {
        println!("  {name}: {}", if *vote { "YES" } else { "NO" });
    }
    let passed = runner.consensus_rule().passes(yes, total);
    println!(
        "  Result: {} ({yes}/{total}, rule: {:?})",
        if passed { "PASSED" } else { "BLOCKED" },
        runner.consensus_rule()
    );
    println!();

    // Show what happens when the charter is violated.
    println!("--- Validation Failures ---");

    let too_small = TeamFormation::new(
        TeamFormationMode::CapabilityMatched,
        vec![CollaborationMember::new(
            "solo",
            "Solo",
            CollaborationRole::Lead,
        )],
    );
    match CollaborationCharter::huddle().validate(&too_small) {
        Err(e) => println!("  Too few members: {e}"),
        Ok(()) => println!("  (unexpected: accepted)"),
    }

    let missing_critic = TeamFormation::new(
        TeamFormationMode::CapabilityMatched,
        vec![
            CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
            CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
            CollaborationMember::new("synth", "Synth", CollaborationRole::Synthesizer),
        ],
    );
    match CollaborationCharter::huddle().validate(&missing_critic) {
        Err(e) => println!("  Missing critic: {e}"),
        Ok(()) => println!("  (unexpected: accepted)"),
    }

    // Override consensus to unanimous — same votes now fail.
    println!();
    println!("--- Unanimous Override ---");
    let strict = ConsensusRule::Unanimous;
    let passed_strict = strict.passes(yes, total);
    println!(
        "  Same votes ({yes}/{total}) with unanimous: {}",
        if passed_strict { "PASSED" } else { "BLOCKED" }
    );
}
