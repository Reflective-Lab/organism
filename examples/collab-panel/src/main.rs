// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Panel — curated expert panel with lead-then-round-robin cadence.
//!
//! A panel is a formal collaboration: curated membership, enforced
//! discipline, and a structured turn order where the lead speaks first
//! to frame the discussion, then the rest follow in round-robin.
//!
//! This example shows:
//! - Curated team formation (enforced — wrong mode is rejected)
//! - Lead-then-round-robin turn cadence
//! - Judges who vote but don't contribute content
//! - Report writers who write but don't vote
//! - Formation mode enforcement (strict matching)

use organism_pack::{
    CollaborationCharter, CollaborationMember, CollaborationRole, TeamFormation,
    TeamFormationMode,
};
use organism_runtime::{CollaborationParticipant, CollaborationRunner};

#[derive(Debug, Clone)]
struct Panelist {
    id: String,
    name: String,
    role: CollaborationRole,
    expertise: String,
}

impl CollaborationParticipant for Panelist {
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
    println!("=== Panel: Investment Committee Review ===\n");

    let charter = CollaborationCharter::panel();

    println!("Charter:");
    println!("  Topology:       {:?}", charter.topology);
    println!("  Formation mode: {:?}", charter.formation_mode);
    println!("  Discipline:     {:?}", charter.discipline);
    println!("  Turn cadence:   {:?}", charter.turn_cadence);
    println!("  Consensus:      {:?}", charter.consensus_rule);
    println!();

    // Panel requires Curated formation. Build a proper team.
    let team = TeamFormation::curated(vec![
        CollaborationMember::new("chair", "Committee Chair", CollaborationRole::Lead)
            .with_persona("Frames the investment thesis and drives toward a decision"),
        CollaborationMember::new("sector-expert", "Sector Expert", CollaborationRole::Domain)
            .with_persona("Deep knowledge of the target's industry vertical"),
        CollaborationMember::new("devil", "Devil's Advocate", CollaborationRole::Critic)
            .with_persona("Systematically attacks the bull case"),
        CollaborationMember::new("judge-1", "Senior Partner", CollaborationRole::Judge),
        CollaborationMember::new("judge-2", "Risk Officer", CollaborationRole::Judge),
        CollaborationMember::new("analyst", "Report Analyst", CollaborationRole::ReportWriter),
    ]);

    let panelists = vec![
        Panelist { id: "chair".into(), name: "Committee Chair".into(), role: CollaborationRole::Lead, expertise: "M&A strategy".into() },
        Panelist { id: "sector-expert".into(), name: "Sector Expert".into(), role: CollaborationRole::Domain, expertise: "Enterprise SaaS".into() },
        Panelist { id: "devil".into(), name: "Devil's Advocate".into(), role: CollaborationRole::Critic, expertise: "Risk analysis".into() },
        Panelist { id: "judge-1".into(), name: "Senior Partner".into(), role: CollaborationRole::Judge, expertise: "Portfolio management".into() },
        Panelist { id: "judge-2".into(), name: "Risk Officer".into(), role: CollaborationRole::Judge, expertise: "Operational risk".into() },
        Panelist { id: "analyst".into(), name: "Report Analyst".into(), role: CollaborationRole::ReportWriter, expertise: "Investment memos".into() },
    ];

    let runner = CollaborationRunner::new(team, charter, panelists).expect("panel should be valid");

    // Show the role-based capability split.
    println!("Role capabilities:");
    println!("  Contributors (speak in rounds): {}", runner.contributors().len());
    for c in runner.contributors() {
        println!("    {} — {:?} ({})", c.name, c.role, c.expertise);
    }
    println!("  Voters (done-gate): {}", runner.voters().len());
    for v in runner.voters() {
        println!("    {} — {:?}", v.name, v.role);
    }
    println!("  Report owner: {}", runner.report_owner().map_or("none", |p| p.display_name()));
    println!();

    // Key insight: Judges vote but don't contribute content.
    // Report writers write but don't vote.
    println!("--- Role Matrix ---");
    for role in [
        CollaborationRole::Lead,
        CollaborationRole::Domain,
        CollaborationRole::Critic,
        CollaborationRole::Judge,
        CollaborationRole::ReportWriter,
    ] {
        println!(
            "  {:15} contributes={:<5} votes={:<5} writes={}",
            role.label(),
            role.contributes_in_rounds(),
            role.votes_on_done_gate(),
            role.can_write_report(),
        );
    }
    println!();

    // Demonstrate formation mode enforcement.
    println!("--- Formation Mode Enforcement ---");
    let wrong_mode_team = TeamFormation::new(
        TeamFormationMode::OpenCall,
        vec![
            CollaborationMember::new("lead", "Lead", CollaborationRole::Lead),
            CollaborationMember::new("domain", "Domain", CollaborationRole::Domain),
            CollaborationMember::new("critic", "Critic", CollaborationRole::Critic),
        ],
    );
    match CollaborationCharter::panel().validate(&wrong_mode_team) {
        Err(e) => println!("  OpenCall team rejected by enforced Panel: {e}"),
        Ok(()) => println!("  (unexpected: accepted)"),
    }

    // Same team works with a moderated charter (DiscussionGroup).
    let moderated_charter = CollaborationCharter::discussion_group();
    println!(
        "  Moderated charter ignores mode mismatch: discipline={:?}",
        moderated_charter.discipline
    );
    println!();

    // Simulate the lead-then-round-robin cadence.
    println!("--- Turn Order (Lead Then Round-Robin) ---");
    println!("  Turn cadence: {:?}", runner.turn_cadence());
    println!("  1. Committee Chair (Lead) frames the discussion");
    println!("  2. Sector Expert (Domain) presents sector analysis");
    println!("  3. Devil's Advocate (Critic) challenges the thesis");
    println!("  4. → Synthesizer produces round summary");
    println!("  5. → Done gate: 5 voters decide (Judges + contributors)");
}
