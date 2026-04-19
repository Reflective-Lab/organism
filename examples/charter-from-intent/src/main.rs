// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Charter from Intent — dynamic collaboration shape derivation.
//!
//! Shows how the same derivation engine reads different intent profiles
//! and produces entirely different collaboration charters, with a
//! transparent rationale for every choice.

use chrono::{Duration, Utc};
use organism_intent::{ExpiryAction, ForbiddenAction, IntentPacket, Reversibility};
use organism_pack::derive_charter;

fn main() {
    let now = Utc::now();

    println!("=== Charter Derivation from Intent ===\n");

    // ── Intent 1: Low-stakes exploration ──────────────────────────
    let exploration = IntentPacket::new("Research market trends in Nordic SaaS", now + Duration::days(14));

    let derived = derive_charter(&exploration, now);
    print_derived("1. Low-Stakes Exploration", &derived);

    // ── Intent 2: High-stakes irreversible acquisition ────────────
    let mut acquisition = IntentPacket::new("Acquire Outpost24 for €200M", now + Duration::days(30));
    acquisition.reversibility = Reversibility::Irreversible;
    acquisition.authority = vec!["board".into(), "cfo".into(), "legal".into(), "ceo".into()];
    acquisition.constraints = vec![
        "regulatory_approval".into(),
        "antitrust_review".into(),
        "due_diligence_complete".into(),
        "financing_secured".into(),
    ];
    acquisition.forbidden = vec![
        ForbiddenAction { action: "public_disclosure".into(), reason: "NDA in effect".into() },
        ForbiddenAction { action: "direct_contact".into(), reason: "Intermediary required".into() },
        ForbiddenAction { action: "binding_offer".into(), reason: "Board approval needed first".into() },
    ];
    acquisition.expiry_action = ExpiryAction::Escalate;

    let derived = derive_charter(&acquisition, now);
    print_derived("2. High-Stakes Irreversible Acquisition", &derived);

    // ── Intent 3: Urgent time-pressured decision ──────────────────
    let urgent = IntentPacket::new("Respond to competitor's hostile bid", now + Duration::hours(4));

    let derived = derive_charter(&urgent, now);
    print_derived("3. Urgent Time-Pressured Decision", &derived);

    // ── Intent 4: Moderate complexity ─────────────────────────────
    let mut moderate = IntentPacket::new("Evaluate new vendor for data pipeline", now + Duration::days(7));
    moderate.reversibility = Reversibility::Partial;
    moderate.constraints = vec!["budget_cap".into(), "timeline".into(), "soc2_required".into()];
    moderate.forbidden = vec![
        ForbiddenAction { action: "multi_year_commitment".into(), reason: "Trial first".into() },
    ];

    let derived = derive_charter(&moderate, now);
    print_derived("4. Moderate Complexity Vendor Evaluation", &derived);

    // ── Show how a single field changes the entire shape ──────────
    println!("=== Sensitivity: Changing Reversibility Alone ===\n");

    for rev in [Reversibility::Reversible, Reversibility::Partial, Reversibility::Irreversible] {
        let mut intent = IntentPacket::new("Same outcome, different reversibility", now + Duration::days(7));
        intent.reversibility = rev;
        intent.constraints = vec!["budget".into(), "timeline".into()];

        let derived = derive_charter(&intent, now);
        println!(
            "  {:12} → {:?} / {:?} / {:?} (confidence: {:.2})",
            format!("{rev:?}"),
            derived.charter.topology,
            derived.charter.discipline,
            derived.charter.consensus_rule,
            derived.confidence,
        );
    }
}

fn print_derived(title: &str, derived: &organism_pack::DerivedCharter) {
    println!("--- {title} ---\n");

    println!("  Complexity signals:");
    let c = &derived.intent_complexity;
    println!("    constraint_pressure:  {:.2}", c.constraint_pressure);
    println!("    authority_breadth:    {:.2}", c.authority_breadth);
    println!("    forbidden_density:    {:.2}", c.forbidden_density);
    println!("    time_pressure:        {:.2}", c.time_pressure);
    println!("    reversibility_weight: {:.2}", c.reversibility_weight);
    println!("    escalation_required:  {}", c.escalation_required);
    println!();

    let ch = &derived.charter;
    println!("  Derived charter:");
    println!("    Topology:       {:?}", ch.topology);
    println!("    Discipline:     {:?}", ch.discipline);
    println!("    Consensus:      {:?}", ch.consensus_rule);
    println!("    Turn cadence:   {:?}", ch.turn_cadence);
    println!("    Formation:      {:?}", ch.formation_mode);
    println!("    Min members:    {}", ch.minimum_members);
    println!("    Explicit turns: {}", ch.require_explicit_turns);
    println!("    Dissent map:    {}", ch.require_dissent_map);
    println!("    Done gate:      {}", ch.require_done_gate);
    println!("    Roles:          {:?}", ch.expected_roles.iter().map(|r| r.label()).collect::<Vec<_>>());
    println!("    Confidence:     {:.2}", derived.confidence);
    println!();

    let r = &derived.rationale;
    println!("  Rationale:");
    println!("    Topology:   {}", r.topology_reason);
    println!("    Discipline: {}", r.discipline_reason);
    println!("    Consensus:  {}", r.consensus_reason);
    println!("    Cadence:    {}", r.cadence_reason);
    println!("    Formation:  {}", r.formation_reason);
    println!();
}
