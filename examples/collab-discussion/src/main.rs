// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Discussion Group — moderated, advisory collaboration.
//!
//! A discussion group sits between the strict huddle and the loose swarm:
//! moderated discipline, a moderator who speaks first, and advisory-only
//! consensus (the output is a recommendation, not a binding decision).
//!
//! This example shows:
//! - Moderator-then-round-robin turn cadence
//! - Moderated discipline (flexible formation modes accepted)
//! - Advisory consensus (non-binding — no done-gate)
//! - Comparing all four collaboration topologies side by side

use organism_pack::{
    CollaborationCharter, CollaborationDiscipline, CollaborationMember, CollaborationRole,
    ConsensusRule, TeamFormation, TeamFormationMode, TurnCadence,
};
use organism_runtime::{CollaborationParticipant, CollaborationRunner};

#[derive(Debug, Clone)]
struct Participant {
    id: String,
    name: String,
    role: CollaborationRole,
}

impl CollaborationParticipant for Participant {
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
    println!("=== Discussion Group: Strategy Brainstorm ===\n");

    let charter = CollaborationCharter::discussion_group();

    println!("Charter:");
    println!("  Topology:       {:?}", charter.topology);
    println!("  Discipline:     {:?}", charter.discipline);
    println!("  Turn cadence:   {:?}", charter.turn_cadence);
    println!("  Consensus:      {:?}", charter.consensus_rule);
    println!("  Done gate:      {} (advisory — no binding vote)", charter.require_done_gate);
    println!("  Dissent map:    {} (softer than a huddle)", charter.require_dissent_map);
    println!();

    // Moderated discipline accepts any formation mode.
    println!("--- Flexible Formation ---");
    let team = TeamFormation::new(
        TeamFormationMode::SelfSelected,
        vec![
            CollaborationMember::new("facilitator", "Facilitator", CollaborationRole::Moderator)
                .with_persona("Keeps the discussion productive and on-topic"),
            CollaborationMember::new("product", "Product Lead", CollaborationRole::Domain)
                .with_persona("Owns the product roadmap and customer feedback"),
            CollaborationMember::new("eng", "Engineering Lead", CollaborationRole::Generalist)
                .with_persona("Evaluates technical feasibility and trade-offs"),
            CollaborationMember::new("design", "Design Lead", CollaborationRole::Generalist)
                .with_persona("Advocates for user experience and accessibility"),
        ],
    );
    println!(
        "  Formation mode: {:?} (charter expects {:?}, but discipline is {:?} — accepted)",
        team.mode, charter.formation_mode, charter.discipline
    );
    println!();

    let participants = vec![
        Participant { id: "facilitator".into(), name: "Facilitator".into(), role: CollaborationRole::Moderator },
        Participant { id: "product".into(), name: "Product Lead".into(), role: CollaborationRole::Domain },
        Participant { id: "eng".into(), name: "Engineering Lead".into(), role: CollaborationRole::Generalist },
        Participant { id: "design".into(), name: "Design Lead".into(), role: CollaborationRole::Generalist },
    ];

    let runner = CollaborationRunner::new(team, charter, participants)
        .expect("discussion group should be valid");

    // Moderator speaks first, then round-robin.
    println!("--- Turn Order (Moderator Then Round-Robin) ---");
    println!("  Turn cadence: {:?}", runner.turn_cadence());
    println!("  1. Facilitator (Moderator) frames the topic");
    println!("  2. Product Lead (Domain) shares customer insights");
    println!("  3. Engineering Lead (Generalist) evaluates feasibility");
    println!("  4. Design Lead (Generalist) considers UX impact");
    println!("  5. → Round synthesis produced");
    println!();

    // The moderator is special: speaks first but doesn't contribute or vote.
    println!("--- Moderator Role ---");
    let mod_role = CollaborationRole::Moderator;
    println!("  Contributes in rounds: {}", mod_role.contributes_in_rounds());
    println!("  Votes on done gate:    {}", mod_role.votes_on_done_gate());
    println!("  Can write report:      {}", mod_role.can_write_report());
    println!("  (Moderator guides but doesn't dominate)");
    println!();

    // Advisory consensus — the output is informational, not authoritative.
    println!("--- Advisory Consensus ---");
    println!("  No done gate: the discussion ends when the moderator");
    println!("  determines sufficient ground has been covered.");
    println!("  Output is a recommendation — authority stays with Converge.");
    println!();

    // Compare all four topologies side by side.
    println!("=== Topology Comparison ===\n");
    let charters = [
        ("Huddle", CollaborationCharter::huddle()),
        ("Discussion", CollaborationCharter::discussion_group()),
        ("Panel", CollaborationCharter::panel()),
        ("Self-Org", CollaborationCharter::self_organizing()),
    ];

    println!(
        "  {:12} {:14} {:10} {:22} {:14} {:>4} {:>6} {:>5} {:>5}",
        "Topology", "Discipline", "Formation", "Turns", "Consensus", "Min", "Turns", "Diss", "Gate"
    );
    println!("  {}", "-".repeat(105));
    for (label, c) in &charters {
        println!(
            "  {:12} {:14} {:10} {:22} {:14} {:>4} {:>6} {:>5} {:>5}",
            label,
            format!("{:?}", c.discipline),
            format!("{:?}", c.formation_mode).chars().take(10).collect::<String>(),
            format!("{:?}", c.turn_cadence),
            format!("{:?}", c.consensus_rule),
            c.minimum_members,
            c.require_explicit_turns,
            c.require_dissent_map,
            c.require_done_gate,
        );
    }
    println!();

    // Show the discipline spectrum.
    println!("=== Discipline Spectrum ===\n");
    println!("  {:?} — charter is law, violations rejected", CollaborationDiscipline::Enforced);
    println!("  {:?} — soft guidance, formation mismatches tolerated", CollaborationDiscipline::Moderated);
    println!("  {:?} — minimal structure, maximum autonomy", CollaborationDiscipline::Loose);
    println!();

    // Show all turn cadences.
    println!("=== Turn Cadences ===\n");
    for cadence in [
        TurnCadence::RoundRobin,
        TurnCadence::LeadThenRoundRobin,
        TurnCadence::ModeratorThenRoundRobin,
        TurnCadence::SynthesisOnly,
        TurnCadence::FigureItOut,
    ] {
        println!("  {:?} — {}", cadence, match cadence {
            TurnCadence::RoundRobin => "equal turns, strict rotation",
            TurnCadence::LeadThenRoundRobin => "lead frames, then rotation",
            TurnCadence::ModeratorThenRoundRobin => "moderator frames, then rotation",
            TurnCadence::SynthesisOnly => "no individual turns, only synthesis rounds",
            TurnCadence::FigureItOut => "agents self-coordinate",
        });
    }
    println!();

    // Show all consensus rules with example votes.
    println!("=== Consensus Rules (5 voters) ===\n");
    let rules = [
        ConsensusRule::Majority,
        ConsensusRule::Supermajority,
        ConsensusRule::Unanimous,
        ConsensusRule::LeadDecides,
        ConsensusRule::AdvisoryOnly,
    ];
    println!("  {:14} 0/5  1/5  2/5  3/5  4/5  5/5", "Rule");
    println!("  {}", "-".repeat(56));
    for rule in &rules {
        println!(
            "  {:14} {:<5}{:<5}{:<5}{:<5}{:<5}{}",
            format!("{rule:?}"),
            if rule.passes(0, 5) { "PASS" } else { "FAIL" },
            if rule.passes(1, 5) { "PASS" } else { "FAIL" },
            if rule.passes(2, 5) { "PASS" } else { "FAIL" },
            if rule.passes(3, 5) { "PASS" } else { "FAIL" },
            if rule.passes(4, 5) { "PASS" } else { "FAIL" },
            if rule.passes(5, 5) { "PASS" } else { "FAIL" },
        );
    }
}
