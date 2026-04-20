// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Self-Organizing — loose swarm that figures out its own turn order.
//!
//! The loosest collaboration shape: open-call formation, loose discipline,
//! and a "figure it out" turn cadence. The team self-selects, roles are
//! fluid, and the only hard requirement is round synthesis.
//!
//! This example shows:
//! - OpenCall formation (anyone can join)
//! - Loose discipline (no formation mode enforcement)
//! - FigureItOut turn cadence (agents decide order themselves)
//! - Minimum 1 member (can start solo and grow)
//! - Generalist role (can contribute, vote, AND write reports)
//! - Advisory consensus (the done gate always passes)
//! - Dynamic team scaling

use organism_pack::{
    CollaborationCharter, CollaborationMember, CollaborationRole, ConsensusRule, TeamFormation,
    TeamFormationMode, TurnCadence,
};
use organism_runtime::{CollaborationParticipant, CollaborationRunner};

#[derive(Debug, Clone)]
struct SwarmAgent {
    id: String,
    name: String,
    role: CollaborationRole,
}

impl CollaborationParticipant for SwarmAgent {
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
    println!("=== Self-Organizing: Research Swarm ===\n");

    let charter = CollaborationCharter::self_organizing();

    println!("Charter:");
    println!("  Topology:       {:?}", charter.topology);
    println!("  Formation mode: {:?}", charter.formation_mode);
    println!("  Discipline:     {:?}", charter.discipline);
    println!("  Turn cadence:   {:?}", charter.turn_cadence);
    println!("  Consensus:      {:?}", charter.consensus_rule);
    println!("  Min members:    {}", charter.minimum_members);
    println!("  Explicit turns: {}", charter.require_explicit_turns);
    println!("  Done gate:      {}", charter.require_done_gate);
    println!();

    // Start with a single generalist — the minimum viable team.
    println!("--- Phase 1: Solo Start ---");
    let solo_team = TeamFormation::new(
        TeamFormationMode::OpenCall,
        vec![CollaborationMember::new(
            "scout",
            "Scout Agent",
            CollaborationRole::Generalist,
        )],
    );
    let solo_agents = vec![SwarmAgent {
        id: "scout".into(),
        name: "Scout Agent".into(),
        role: CollaborationRole::Generalist,
    }];

    let runner = CollaborationRunner::new(solo_team, charter.clone(), solo_agents)
        .expect("solo team should be valid");

    println!("  Team size: {}", runner.team().member_count());
    println!("  Contributors: {}", runner.contributors().len());
    println!("  Voters: {}", runner.voters().len());
    println!(
        "  Report owner: {}",
        runner.report_owner().map_or("none", |p| p.display_name())
    );

    // Generalist can do everything — that's the point.
    let generalist = CollaborationRole::Generalist;
    println!("  Generalist capabilities:");
    println!("    contributes: {}", generalist.contributes_in_rounds());
    println!("    votes:       {}", generalist.votes_on_done_gate());
    println!("    writes:      {}", generalist.can_write_report());
    println!();

    // Grow the team — open call, anyone can join.
    println!("--- Phase 2: Swarm Grows ---");
    let grown_team = TeamFormation::new(
        TeamFormationMode::OpenCall,
        vec![
            CollaborationMember::new("scout", "Scout Agent", CollaborationRole::Generalist),
            CollaborationMember::new("deep-1", "Deep Diver 1", CollaborationRole::Generalist),
            CollaborationMember::new("deep-2", "Deep Diver 2", CollaborationRole::Generalist),
            CollaborationMember::new("watcher", "Observer", CollaborationRole::Observer),
        ],
    );
    let grown_agents = vec![
        SwarmAgent {
            id: "scout".into(),
            name: "Scout Agent".into(),
            role: CollaborationRole::Generalist,
        },
        SwarmAgent {
            id: "deep-1".into(),
            name: "Deep Diver 1".into(),
            role: CollaborationRole::Generalist,
        },
        SwarmAgent {
            id: "deep-2".into(),
            name: "Deep Diver 2".into(),
            role: CollaborationRole::Generalist,
        },
        SwarmAgent {
            id: "watcher".into(),
            name: "Observer".into(),
            role: CollaborationRole::Observer,
        },
    ];

    let runner = CollaborationRunner::new(grown_team, charter.clone(), grown_agents)
        .expect("grown team should be valid");

    println!("  Team size: {}", runner.team().member_count());
    println!("  Contributors: {}", runner.contributors().len());
    println!("  Voters: {}", runner.voters().len());
    println!(
        "  Observer (watcher) contributes: {}",
        CollaborationRole::Observer.contributes_in_rounds()
    );
    println!(
        "  Observer (watcher) votes: {}",
        CollaborationRole::Observer.votes_on_done_gate()
    );
    println!();

    // Advisory consensus — the done gate always passes, even with 0 yes votes.
    println!("--- Advisory Consensus ---");
    println!(
        "  Advisory with 0/3 yes: {}",
        ConsensusRule::AdvisoryOnly.passes(0, 3)
    );
    println!(
        "  Advisory with 0/0:     {}",
        ConsensusRule::AdvisoryOnly.passes(0, 0)
    );
    println!("  (The done gate is a formality — the team decides when it's done)");
    println!();

    // FigureItOut cadence means no prescribed turn order.
    println!("--- Turn Cadence ---");
    println!("  Cadence: {:?}", runner.turn_cadence());
    println!("  No explicit turns: {}", !charter.require_explicit_turns);
    println!("  Agents self-coordinate — whoever has something to say, speaks");
    println!();

    // Show that you can override individual charter fields.
    println!("--- Customization ---");
    let strict_swarm = CollaborationCharter::self_organizing()
        .with_consensus_rule(ConsensusRule::Majority)
        .with_turn_cadence(TurnCadence::RoundRobin);
    println!(
        "  Swarm with majority consensus: {:?}",
        strict_swarm.consensus_rule
    );
    println!(
        "  Swarm with round-robin turns:  {:?}",
        strict_swarm.turn_cadence
    );
}
