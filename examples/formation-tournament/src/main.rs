// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Formation Tournament — the same diligence intent through competing shapes.
//!
//! Organism assembles two formations for the same DD brief:
//! - a loose self-organizing swarm that starts with partial coverage
//! - a curated panel that seeds broader coverage up front
//!
//! Both run through `organism-runtime::Formation`, then
//! `FormationTournament` scores the governed results using only runtime data.

use std::sync::Arc;

use chrono::{Duration, Utc};
use converge_kernel::{Budget, ContextKey};
use organism_pack::{
    BreadthResearchSuggestor, CollaborationCharter, CollaborationMember, CollaborationRole,
    CollaborationTopology, ContradictionFinderSuggestor, DdError, DdLlm, DdSearch,
    DepthResearchSuggestor, FactExtractorSuggestor, GapDetectorSuggestor, HuddleSeedSuggestor,
    IntentPacket, Plan, PlanStep, ReasoningSystem, SearchHit, SharedBudget, SynthesisSuggestor,
    TeamFormation, TeamFormationMode,
};
use organism_runtime::{
    CollaborationParticipant, CollaborationRunner, Formation, FormationTournament, TournamentResult,
};
use uuid::Uuid;

const SUBJECT: &str = "Northstar Cyber";

type TournamentRun = (TournamentResult, Vec<FormationPreview>);
type FormationBuild = (Formation, FormationPreview);

#[tokio::main]
async fn main() {
    println!("=== Formation Tournament ===\n");
    println!("Intent: Run due diligence on {SUBJECT}\n");

    match run_tournament().await {
        Ok((result, previews)) => {
            print_previews(&previews);
            print_result(&result);
        }
        Err(error) => eprintln!("Failed: {error}"),
    }
}

async fn run_tournament() -> Result<TournamentRun, Box<dyn std::error::Error>> {
    let intent = build_intent();
    let builds = vec![
        build_formation(FormationShape::SelfOrganizing, &intent)?,
        build_formation(FormationShape::Panel, &intent)?,
    ];

    let previews: Vec<FormationPreview> =
        builds.iter().map(|(_, preview)| preview.clone()).collect();
    let formations: Vec<Formation> = builds.into_iter().map(|(formation, _)| formation).collect();

    let tournament = FormationTournament::new(intent.id, Uuid::new_v4(), formations);
    let result = tournament.run().await?;
    Ok((result, previews))
}

fn build_formation(
    shape: FormationShape,
    intent: &IntentPacket,
) -> Result<FormationBuild, Box<dyn std::error::Error>> {
    let runner = build_runner(shape)?;
    let plans = seed_plans(shape, &runner, intent);
    let label = shape.label();

    let budget = Arc::new(
        SharedBudget::new()
            .with_limit("searches", 8)
            .with_limit("llm", 8),
    );
    let search: Arc<dyn DdSearch> = Arc::new(StubSearch);
    let llm: Arc<dyn DdLlm> = Arc::new(StubDdLlm);

    let formation = Formation::new(label)
        .with_budget(Budget {
            max_cycles: 12,
            max_facts: 256,
        })
        .seed(
            ContextKey::Seeds,
            format!("brief:{}", shape.label()),
            build_brief(),
            "investment-directive",
        )
        .agent(HuddleSeedSuggestor::from_plans(intent.clone(), plans))
        .agent(BreadthResearchSuggestor::new(
            SUBJECT,
            budget.clone(),
            search.clone(),
        ))
        .agent(DepthResearchSuggestor::new(SUBJECT, budget.clone(), search))
        .agent(FactExtractorSuggestor::new(
            SUBJECT,
            budget.clone(),
            llm.clone(),
        ))
        .agent(
            GapDetectorSuggestor::new(SUBJECT, budget.clone(), llm.clone()).with_max_generations(2),
        )
        .agent(ContradictionFinderSuggestor::new())
        .agent(
            SynthesisSuggestor::new(SUBJECT, budget.clone(), llm).with_required_stable_cycles(2),
        );

    let preview = FormationPreview {
        label: label.into(),
        topology: runner.charter().topology,
        mode: runner.team().mode,
        contributors: runner
            .contributors()
            .iter()
            .map(|participant| participant.display_name().to_owned())
            .collect(),
        seed_plan_count: runner.contributors().len(),
        seed_summary: seed_summary(shape),
    };

    Ok((formation, preview))
}

fn build_intent() -> IntentPacket {
    IntentPacket::new(
        format!("Run diligence tournament on {SUBJECT}"),
        Utc::now() + Duration::hours(12),
    )
}

fn build_brief() -> String {
    serde_json::json!({
        "target": SUBJECT,
        "thesis": "Focused attack surface management vendor serving regulated mid-market security teams.",
        "questions": [
            "Is the product technically differentiated?",
            "Do the economics justify deeper diligence?",
            "Where does the company win and lose competitively?"
        ]
    })
    .to_string()
}

fn build_runner(
    shape: FormationShape,
) -> Result<CollaborationRunner<ResearchParticipant>, Box<dyn std::error::Error>> {
    let (charter, team, participants) = match shape {
        FormationShape::SelfOrganizing => (
            CollaborationCharter::self_organizing(),
            TeamFormation::new(
                TeamFormationMode::OpenCall,
                vec![
                    CollaborationMember::new("scout", "Scout Agent", CollaborationRole::Generalist),
                    CollaborationMember::new(
                        "tech-diver",
                        "Tech Diver",
                        CollaborationRole::Generalist,
                    ),
                    CollaborationMember::new("watcher", "Watcher", CollaborationRole::Observer),
                ],
            ),
            vec![
                ResearchParticipant::new(
                    "scout",
                    "Scout Agent",
                    CollaborationRole::Generalist,
                    "commercial reconnaissance",
                    ReasoningSystem::DomainModel,
                ),
                ResearchParticipant::new(
                    "tech-diver",
                    "Tech Diver",
                    CollaborationRole::Generalist,
                    "technical depth",
                    ReasoningSystem::CausalAnalysis,
                ),
                ResearchParticipant::new(
                    "watcher",
                    "Watcher",
                    CollaborationRole::Observer,
                    "passive observation",
                    ReasoningSystem::MlPrediction,
                ),
            ],
        ),
        FormationShape::Panel => (
            CollaborationCharter::panel(),
            TeamFormation::curated(vec![
                CollaborationMember::new("chair", "Committee Chair", CollaborationRole::Lead),
                CollaborationMember::new("sector", "Sector Expert", CollaborationRole::Domain),
                CollaborationMember::new("critic", "Devil's Advocate", CollaborationRole::Critic),
                CollaborationMember::new(
                    "synth",
                    "Lead Synthesizer",
                    CollaborationRole::Synthesizer,
                ),
            ]),
            vec![
                ResearchParticipant::new(
                    "chair",
                    "Committee Chair",
                    CollaborationRole::Lead,
                    "commercial framing",
                    ReasoningSystem::DomainModel,
                ),
                ResearchParticipant::new(
                    "sector",
                    "Sector Expert",
                    CollaborationRole::Domain,
                    "technical and product depth",
                    ReasoningSystem::CausalAnalysis,
                ),
                ResearchParticipant::new(
                    "critic",
                    "Devil's Advocate",
                    CollaborationRole::Critic,
                    "risk and competitive pressure",
                    ReasoningSystem::ConstraintSolver,
                ),
                ResearchParticipant::new(
                    "synth",
                    "Lead Synthesizer",
                    CollaborationRole::Synthesizer,
                    "coverage closure",
                    ReasoningSystem::LlmReasoning,
                ),
            ],
        ),
    };

    Ok(CollaborationRunner::new(team, charter, participants)?)
}

fn seed_plans(
    shape: FormationShape,
    runner: &CollaborationRunner<ResearchParticipant>,
    intent: &IntentPacket,
) -> Vec<Plan> {
    runner
        .contributors()
        .iter()
        .filter_map(|participant| {
            let mut plan = Plan::new(
                intent,
                format!(
                    "{} seeds diligence from a {} angle",
                    participant.display_name(),
                    participant.focus
                ),
            );
            plan.contributor = participant.system;

            match (shape, participant.id()) {
                (FormationShape::SelfOrganizing, "scout") => {
                    plan.steps.push(step(
                        "breadth market map, customer proof, ideal customer profile",
                        "establish commercial baseline and buyer pull",
                    ));
                    Some(plan)
                }
                (FormationShape::SelfOrganizing, "tech-diver") => {
                    plan.steps.push(step(
                        "depth platform architecture, attack surface telemetry, integrations",
                        "validate technical moat and implementation credibility",
                    ));
                    Some(plan)
                }
                (FormationShape::Panel, "chair") => {
                    plan.steps.push(step(
                        "breadth market map, customer proof, ideal customer profile",
                        "cover market fit and customer validation",
                    ));
                    Some(plan)
                }
                (FormationShape::Panel, "sector") => {
                    plan.steps.push(step(
                        "depth platform architecture, attack surface telemetry, integrations",
                        "cover technical credibility in the first pass",
                    ));
                    Some(plan)
                }
                (FormationShape::Panel, "critic") => {
                    plan.steps.push(step(
                        "depth competitive positioning and win-loss against exposure management vendors",
                        "force the competitive pressure test early",
                    ));
                    Some(plan)
                }
                (FormationShape::Panel, "synth") => {
                    plan.steps.push(step(
                        "breadth ARR growth and ownership Northstar Cyber",
                        "cover economics and ownership before the loop has to reopen gaps",
                    ));
                    Some(plan)
                }
                _ => None,
            }
        })
        .collect()
}

fn step(action: &str, expected_effect: &str) -> PlanStep {
    PlanStep {
        action: action.into(),
        expected_effect: expected_effect.into(),
    }
}

fn print_previews(previews: &[FormationPreview]) {
    println!("Competing formations:");
    for preview in previews {
        println!("  {}:", preview.label);
        println!("    topology:    {:?}", preview.topology);
        println!("    mode:        {:?}", preview.mode);
        println!("    contributors: {}", preview.contributors.join(", "));
        println!("    seed plans:  {}", preview.seed_plan_count);
        println!("    seed style:  {}", preview.seed_summary);
    }
    println!();
}

fn print_result(result: &TournamentResult) {
    println!("Scoreboard:");
    for score in &result.all_scores {
        println!(
            "  - {:<24} score {:.3} | converged {} | cycles {} | criteria {}/{}",
            score.label,
            score.score,
            score.converged,
            score.cycles,
            score.criteria_met,
            score.criteria_total
        );
    }
    println!();

    println!(
        "Winner: {} (score {:.3}, {} cycles)\n",
        result.winner.label, result.winner.score, result.winner.cycles
    );

    if result.priors.is_empty() {
        println!("No priors emitted.");
        return;
    }

    println!("Calibrated priors:");
    for prior in &result.priors {
        println!(
            "  - {} / {}: {:.3} -> {:.3} (evidence {})",
            prior.assumption_type,
            prior.context,
            prior.prior_confidence,
            prior.posterior_confidence,
            prior.evidence_count
        );
    }
}

fn seed_summary(shape: FormationShape) -> &'static str {
    match shape {
        FormationShape::SelfOrganizing => "starts partial and lets the loop reopen gaps",
        FormationShape::Panel => "front-loads broader coverage before the loop starts",
    }
}

#[derive(Debug, Clone, Copy)]
enum FormationShape {
    SelfOrganizing,
    Panel,
}

impl FormationShape {
    const fn label(self) -> &'static str {
        match self {
            Self::SelfOrganizing => "dd-self-organizing",
            Self::Panel => "dd-panel",
        }
    }
}

#[derive(Debug, Clone)]
struct FormationPreview {
    label: String,
    topology: CollaborationTopology,
    mode: TeamFormationMode,
    contributors: Vec<String>,
    seed_plan_count: usize,
    seed_summary: &'static str,
}

#[derive(Debug, Clone)]
struct ResearchParticipant {
    id: String,
    name: String,
    role: CollaborationRole,
    focus: &'static str,
    system: ReasoningSystem,
}

impl ResearchParticipant {
    fn new(
        id: &str,
        name: &str,
        role: CollaborationRole,
        focus: &'static str,
        system: ReasoningSystem,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role,
            focus,
            system,
        }
    }
}

impl CollaborationParticipant for ResearchParticipant {
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

struct StubSearch;

#[async_trait::async_trait]
impl DdSearch for StubSearch {
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, DdError> {
        let query = query.to_ascii_lowercase();

        if query.contains("market map") || query.contains("customer proof") {
            return Ok(vec![
                hit(
                    "Northstar Cyber targets regulated mid-market security teams",
                    "https://research.example/northstar-market-map",
                    "Northstar Cyber sells an attack surface management platform used by security teams at regional banks, insurers, and healthcare providers. Reference customers include Fjord Bank and Veridian Health.",
                ),
                hit(
                    "Northstar Cyber buyer note on external exposure visibility",
                    "https://research.example/northstar-customer-voice",
                    "Customers buy Northstar Cyber for focused external exposure visibility rather than full CNAPP breadth, especially when regulated teams need fast deployment.",
                ),
            ]);
        }

        if query.contains("platform architecture") || query.contains("telemetry") {
            return Ok(vec![hit(
                "Northstar Cyber platform architecture overview",
                "https://research.example/northstar-architecture",
                "Northstar Cyber collects passive DNS, certificate transparency, cloud asset, and SaaS exposure telemetry through native collectors and APIs. The platform routes findings into analyst workflows and ticketing systems.",
            )]);
        }

        if query.contains("arr growth") || query.contains("ownership") {
            return Ok(vec![
                hit(
                    "Northstar Cyber board update cites ARR growth",
                    "https://research.example/northstar-arr",
                    "Northstar Cyber reached $14.2M ARR in FY2025, growing 58% year over year, and remained gross-margin positive.",
                ),
                hit(
                    "Northstar Cyber cap table and ownership note",
                    "https://research.example/northstar-cap-table",
                    "Northstar Cyber completed a Series B led by Granite Peak Ventures with RiverNorth participating.",
                ),
            ]);
        }

        if query.contains("competitive positioning")
            || query.contains("win-loss")
            || query.contains("competition")
        {
            return Ok(vec![
                hit(
                    "Northstar Cyber competitive win-loss review",
                    "https://research.example/northstar-win-loss",
                    "Northstar Cyber most often competes with Wiz, Tenable, and Censys in exposure management evaluations.",
                ),
                hit(
                    "Northstar Cyber enterprise field note",
                    "https://research.example/northstar-enterprise-fit",
                    "Sources disagree on whether Northstar Cyber can displace broader CNAPP suites in large enterprise bundles.",
                ),
            ]);
        }

        Ok(Vec::new())
    }
}

fn hit(title: &str, url: &str, content: &str) -> SearchHit {
    SearchHit {
        title: title.into(),
        url: url.into(),
        content: content.into(),
        provider: "stub-research".into(),
    }
}

struct StubDdLlm;

#[async_trait::async_trait]
impl DdLlm for StubDdLlm {
    async fn complete(&self, prompt: &str) -> Result<String, DdError> {
        if prompt.contains("Extract key facts as JSON array") {
            return Ok(serde_json::json!({
                "facts": extract_facts(prompt)
            })
            .to_string());
        }

        if prompt.contains("What critical gaps remain?") {
            let missing = missing_categories(prompt);
            let mut strategies = Vec::new();

            if missing.contains(&"financials") {
                strategies.push(serde_json::json!({
                    "query": "ARR growth and ownership Northstar Cyber",
                    "mode": "breadth",
                    "reason": "economics and ownership are still missing"
                }));
            }
            if missing.contains(&"competition") {
                strategies.push(serde_json::json!({
                    "query": "competitive positioning and win-loss against exposure management vendors",
                    "mode": "depth",
                    "reason": "competitive strength is unresolved"
                }));
            }

            return Ok(serde_json::json!({ "strategies": strategies }).to_string());
        }

        if prompt.contains("Produce a final analysis as JSON") {
            return Ok(serde_json::json!({
                "summary": "Northstar Cyber shows credible buyer pull and a real product surface. The panel formation front-loads enough coverage to reach synthesis faster, while the self-organizing formation gets to the same broad answer after reopening gaps.",
                "market_analysis": "The company sits in external exposure management rather than broad CNAPP suites, which appears to resonate with regulated mid-market buyers.",
                "competitive_landscape": "Northstar Cyber most often collides with Wiz, Tenable, and Censys. The mid-market story is credible, but enterprise displacement remains contested.",
                "technology_assessment": "The platform ingests passive DNS, certificate transparency, cloud asset, and SaaS exposure telemetry, then routes findings into analyst workflows.",
                "risk_factors": [
                    "Enterprise up-market fit remains contested",
                    "Commercial scale still needs pressure-testing"
                ],
                "growth_opportunities": [
                    "Expand within regulated verticals already showing buyer pull",
                    "Partner into broader security workflows rather than selling a full CNAPP replacement"
                ],
                "recommendation": "Prefer the formation that reaches full DD coverage with fewer cycles, then proceed to management meeting and reference calls."
            })
            .to_string());
        }

        Err(DdError::BadResponse {
            provider: "stub-llm".into(),
            detail: "unexpected prompt shape".into(),
        })
    }
}

fn extract_facts(prompt: &str) -> Vec<serde_json::Value> {
    let mut facts = Vec::new();

    if prompt.contains("Northstar Cyber targets regulated mid-market security teams") {
        facts.push(serde_json::json!({
            "claim": "Northstar Cyber packages weekly exposure triage workflows and executive-ready reporting for lean security teams.",
            "category": "product",
            "source_indices": [0],
            "confidence": 0.88
        }));
        facts.push(serde_json::json!({
            "claim": "Reference customers include regional banks, insurers, and healthcare providers such as Fjord Bank and Veridian Health.",
            "category": "customers",
            "source_indices": [0],
            "confidence": 0.77
        }));
    }

    if prompt.contains("Northstar Cyber buyer note on external exposure visibility") {
        facts.push(serde_json::json!({
            "claim": "Customers buy Northstar Cyber for focused external exposure visibility rather than full CNAPP breadth.",
            "category": "market",
            "source_indices": [1],
            "confidence": 0.74
        }));
    }

    if prompt.contains("Northstar Cyber platform architecture overview") {
        facts.push(serde_json::json!({
            "claim": "The platform collects passive DNS, certificate transparency, cloud asset, and SaaS exposure telemetry for attack surface management through native collectors and APIs.",
            "category": "technology",
            "source_indices": [2],
            "confidence": 0.86
        }));
        facts.push(serde_json::json!({
            "claim": "Northstar Cyber routes exposure findings into analyst workflows and ticketing systems.",
            "category": "technology",
            "source_indices": [2],
            "confidence": 0.71
        }));
    }

    if prompt.contains("Northstar Cyber board update cites ARR growth") {
        facts.push(serde_json::json!({
            "claim": "Northstar Cyber reached $14.2M ARR in FY2025 and grew 58% year over year.",
            "category": "financials",
            "source_indices": [3],
            "confidence": 0.91
        }));
        facts.push(serde_json::json!({
            "claim": "Northstar Cyber completed a Series B led by Granite Peak Ventures with RiverNorth participating.",
            "category": "financials",
            "source_indices": [4],
            "confidence": 0.83
        }));
    }

    if prompt.contains("Northstar Cyber competitive win-loss review") {
        facts.push(serde_json::json!({
            "claim": "Northstar Cyber most often competes with Wiz, Tenable, and Censys in exposure management evaluations.",
            "category": "competition",
            "source_indices": [5],
            "confidence": 0.78
        }));
    }

    if prompt.contains("Northstar Cyber enterprise field note") {
        facts.push(serde_json::json!({
            "claim": "Sources disagree on whether Northstar Cyber can displace broader CNAPP suites in large enterprise bundles.",
            "category": "competition",
            "source_indices": [6],
            "confidence": 0.62
        }));
    }

    facts
}

fn missing_categories(prompt: &str) -> Vec<&str> {
    let Some(start) = prompt.find("Missing expected categories:\n") else {
        return Vec::new();
    };
    let tail = &prompt[start + "Missing expected categories:\n".len()..];
    let section = tail.split("\n\n").next().unwrap_or("").trim();
    if section == "none" || section.is_empty() {
        return Vec::new();
    }
    section
        .split(',')
        .map(str::trim)
        .filter(|category| !category.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use organism_pack::{HookPatterns, consolidate_dd_hypotheses, extract_hooks_from_facts};

    #[tokio::test]
    async fn panel_wins_tournament_against_self_organizing() {
        let (result, previews) = run_tournament().await.expect("tournament should run");

        assert_eq!(previews.len(), 2);
        assert_eq!(result.all_scores.len(), 2);
        assert_eq!(result.winner.label, FormationShape::Panel.label());

        let panel = result
            .all_scores
            .iter()
            .find(|score| score.label == FormationShape::Panel.label())
            .expect("panel score");
        let swarm = result
            .all_scores
            .iter()
            .find(|score| score.label == FormationShape::SelfOrganizing.label())
            .expect("swarm score");

        assert!(panel.converged);
        assert!(swarm.converged);
        assert!(panel.cycles < swarm.cycles);
        assert!(panel.score > swarm.score);
        assert!(!result.priors.is_empty());
    }

    #[tokio::test]
    async fn tournament_formations_reach_dd_coverage() {
        let intent = build_intent();
        let (panel, _) = build_formation(FormationShape::Panel, &intent).expect("panel formation");
        let result = panel.run().await.expect("panel should converge");

        let summaries =
            consolidate_dd_hypotheses(result.converge_result.context.get(ContextKey::Hypotheses));
        let hooks = extract_hooks_from_facts(SUBJECT, &summaries, &HookPatterns::default());

        assert!(summaries.iter().any(|fact| fact.category == "financials"));
        assert!(summaries.iter().any(|fact| fact.category == "competition"));
        assert!(hooks.competitors.iter().any(|name| name == "Wiz"));
    }
}
