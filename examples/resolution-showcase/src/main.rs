// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Resolution Showcase — demonstrates all matching dimensions.
//!
//! Runs 8 intents through the structural resolver, each designed to
//! light up a specific dimension. Shows exactly how organism decides
//! which packs, capabilities, and invariants an intent needs.

use organism_pack::*;
use organism_runtime::{
    BudgetProbe, CredentialProbe, GapSeverity, PackProbe, ReadinessReport, Registry,
    StructuralResolver, check_readiness,
};

fn main() {
    println!("=== Organism Resolution Showcase ===");
    println!("    8 intents × 8 matching dimensions\n");

    let registry = build_full_registry();
    let resolver = StructuralResolver::new(&registry);

    // ── 1. Fact prefix matching ────────────────────────────────────
    run_scenario(
        &resolver,
        "1. Fact Prefix",
        IntentPacket::new("process incoming leads", one_hour()).with_context(serde_json::json!({
            "pending": ["lead:acme-001", "lead:beta-002"],
            "related": "contract:draft-7"
        })),
        "Context contains 'lead:' and 'contract:' → should match customers + legal",
    );

    // ── 2. Constraint → invariant matching ─────────────────────────
    run_scenario(
        &resolver,
        "2. Constraint → Invariant",
        {
            let mut i = IntentPacket::new("finalize vendor agreement", one_hour());
            i.constraints = vec!["signature_required".into(), "vendor_has_assessment".into()];
            i
        },
        "Constraints reference invariants → should match legal + partnerships",
    );

    // ── 3. Context key flow ────────────────────────────────────────
    run_scenario(
        &resolver,
        "3. Context Key Flow",
        IntentPacket::new("aggregate vendor scores into strategy", one_hour()).with_context(
            serde_json::json!({
                "evaluations": ["price:vendor-a", "compliance:vendor-a"],
                "strategies": "final recommendation needed"
            }),
        ),
        "Context keys 'evaluations' + 'strategies' → match packs writing those ContextKeys",
    );

    // ── 4. Domain entity matching ──────────────────────────────────
    run_scenario(
        &resolver,
        "4. Domain Entity",
        IntentPacket::new(
            "onboard new employee and provision access to all systems",
            one_hour(),
        ),
        "'employee' + 'access' entities → should match people pack",
    );

    // ── 5. Keyword matching ────────────────────────────────────────
    run_scenario(
        &resolver,
        "5. Keyword",
        IntentPacket::new(
            "plan Q3 marketing campaign with attribution tracking",
            one_hour(),
        ),
        "'campaign' + 'attribution' keywords → should match growth_marketing",
    );

    // ── 6. Reversibility ───────────────────────────────────────────
    run_scenario(
        &resolver,
        "6. Reversibility",
        IntentPacket::new("terminate contractor and revoke all access", one_hour())
            .with_reversibility(Reversibility::Irreversible),
        "Irreversible intent → should add governance packs with Acceptance invariants",
    );

    // ── 7. Forbidden action filtering ──────────────────────────────
    run_scenario(
        &resolver,
        "7. Forbidden Filtering",
        {
            let mut i = IntentPacket::new("research potential vendor", one_hour())
                .with_context(serde_json::json!({ "ref": "lead:prospect-1" }));
            i.forbidden = vec![
                ForbiddenAction {
                    action: "linkedin".into(),
                    reason: "no external social media contact authorized".into(),
                },
                ForbiddenAction {
                    action: "outreach".into(),
                    reason: "research only, no contact".into(),
                },
            ];
            i
        },
        "Forbidden 'linkedin' + 'outreach' → linkedin_research should be filtered OUT",
    );

    // ── 8. Capability affinity + readiness ─────────────────────────
    run_scenario_with_readiness(
        &resolver,
        &registry,
        "8. Capability Affinity + Readiness",
        {
            let binding = DeclarativeBinding::new()
                .pack("linkedin_research", "build dossier on target company")
                .build();
            let i = IntentPacket::new("build research dossier", one_hour());
            (i, binding)
        },
        "Declarative linkedin_research → auto-adds linkedin/web/social capabilities → readiness checks credentials",
    );

    println!("=== Done ===");
}

// ── Helpers ────────────────────────────────────────────────────────

fn build_full_registry() -> Registry {
    let mut r = Registry::with_standard_packs();

    // Register available capabilities
    r.register_capability("web", "URL capture and metadata extraction");
    r.register_capability("ocr", "Document understanding");
    r.register_capability("social", "Social profile extraction");
    // Note: linkedin capability NOT registered — this is intentional for readiness demo
    r
}

fn one_hour() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now() + chrono::Duration::hours(1)
}

fn run_scenario(
    resolver: &StructuralResolver<'_>,
    title: &str,
    intent: IntentPacket,
    description: &str,
) {
    println!("── {title} ──");
    println!("   {description}");
    println!("   Intent: \"{}\"", intent.outcome);
    if intent.reversibility != Reversibility::Reversible {
        println!("   Reversibility: {:?}", intent.reversibility);
    }
    if !intent.forbidden.is_empty() {
        for f in &intent.forbidden {
            println!("   Forbidden: {} ({})", f.action, f.reason);
        }
    }
    if !intent.constraints.is_empty() {
        println!("   Constraints: {}", intent.constraints.join(", "));
    }

    let binding = resolver.resolve(&intent, &IntentBinding::default());
    print_binding(&binding);
    println!();
}

fn run_scenario_with_readiness(
    resolver: &StructuralResolver<'_>,
    registry: &Registry,
    title: &str,
    (intent, baseline): (IntentPacket, IntentBinding),
    description: &str,
) {
    println!("── {title} ──");
    println!("   {description}");
    println!("   Intent: \"{}\"", intent.outcome);
    if !baseline.packs.is_empty() {
        println!(
            "   Declarative packs: {}",
            baseline
                .packs
                .iter()
                .map(|p| p.pack_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    let binding = resolver.resolve(&intent, &baseline);
    print_binding(&binding);

    // Run readiness check
    let cred_probe = CredentialProbe::new()
        .require("linkedin", "LINKEDIN_API_KEY")
        .require("web", "ORGANISM_WEB_KEY")
        .require("social", "ANTHROPIC_API_KEY")
        .require("ocr", "MISTRAL_API_KEY")
        .require("vision", "ANTHROPIC_API_KEY");
    let pack_probe = PackProbe::new(registry);
    let budget_probe = BudgetProbe::new()
        .with_token_budget(50_000)
        .with_spend_budget(2.50);

    let report = check_readiness(&binding, &[&cred_probe, &pack_probe, &budget_probe]);
    print_readiness(&report);
    println!();
}

fn print_binding(binding: &IntentBinding) {
    if binding.packs.is_empty() {
        println!("   Packs: (none matched)");
    } else {
        println!("   Packs resolved:");
        for p in &binding.packs {
            println!(
                "     {:14} conf={:.0}%  [{:?}]  {}",
                p.pack_name,
                p.confidence * 100.0,
                p.source,
                p.reason
            );
        }
    }

    if !binding.capabilities.is_empty() {
        println!("   Capabilities:");
        for c in &binding.capabilities {
            println!(
                "     {:14} conf={:.0}%  [{:?}]  {}",
                c.capability,
                c.confidence * 100.0,
                c.source,
                c.reason
            );
        }
    }

    if !binding.invariants.is_empty() {
        println!("   Invariants: {}", binding.invariants.join(", "));
    }
}

fn print_readiness(report: &ReadinessReport) {
    let status = if report.ready { "READY" } else { "NOT READY" };
    println!("   Readiness: {status}");
    for c in &report.confirmed {
        println!("     [ok] {} — {}", c.resource, c.detail);
    }
    for g in &report.gaps {
        let icon = match g.severity {
            GapSeverity::Blocking => "BLOCK",
            GapSeverity::Degraded => "WARN ",
            GapSeverity::Advisory => "INFO ",
        };
        println!("     [{icon}] {} — {}", g.resource, g.reason);
        if let Some(suggestion) = &g.suggestion {
            println!("            fix: {suggestion}");
        }
    }
}
