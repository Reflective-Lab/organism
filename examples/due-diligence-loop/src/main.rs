// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Due Diligence Loop — real Organism DD suggestors running as one adaptive loop.
//!
//! A self-organizing collaboration team seeds the initial research shape through
//! `organism-runtime::Formation`. Breadth/depth research gather signals, the
//! fact extractor turns them into hypotheses, gap detection opens follow-up work
//! when coverage is incomplete, contradiction finding flags disagreement, and
//! synthesis only fires when the hypothesis set stabilizes.

use std::sync::Arc;

use chrono::{Duration, Utc};
use converge_kernel::{ContextKey, ConvergeResult};
use organism_pack::{
    BreadthResearchSuggestor, CollaborationCharter, CollaborationMember, CollaborationRole,
    ContradictionFinderSuggestor, DdError, DdLlm, DdSearch, DepthResearchSuggestor,
    FactExtractorSuggestor, GapDetectorSuggestor, HookPatterns, HuddleSeedSuggestor, IntentPacket,
    Plan, PlanStep, ReasoningSystem, SearchHit, SharedBudget, SynthesisSuggestor, TeamFormation,
    TeamFormationMode, consolidate_dd_hypotheses, extract_hooks_from_facts,
};
use organism_runtime::{CollaborationParticipant, CollaborationRunner, Formation, FormationResult};

const SUBJECT: &str = "Northstar Cyber";
const FORMATION_LABEL: &str = "self-organizing-dd";

type DdRun = (
    FormationResult,
    Arc<SharedBudget>,
    CollaborationRunner<ResearchSwarmAgent>,
);

type DdFormation = (
    Formation,
    Arc<SharedBudget>,
    CollaborationRunner<ResearchSwarmAgent>,
);

#[tokio::main]
async fn main() {
    println!("=== Due Diligence Loop ===\n");
    println!("Target: {SUBJECT}");
    println!("Starting from a self-organizing formation and letting the loop discover the gaps.\n");

    match run_due_diligence().await {
        Ok((result, budget, runner)) => {
            print_formation(&runner);
            print_result(&result, &budget);
        }
        Err(error) => eprintln!("Failed: {error}"),
    }
}

async fn run_due_diligence() -> Result<DdRun, Box<dyn std::error::Error>> {
    let (formation, budget, runner) = build_formation()?;
    let result = formation.run().await?;
    Ok((result, budget, runner))
}

fn build_formation() -> Result<DdFormation, Box<dyn std::error::Error>> {
    let intent = build_intent();
    let brief = build_brief();
    let runner = build_self_organizing_runner()?;
    let plans = seed_plans(&runner, &intent);

    let budget = Arc::new(
        SharedBudget::new()
            .with_limit("searches", 8)
            .with_limit("llm", 8),
    );
    let search: Arc<dyn DdSearch> = Arc::new(StubSearch);
    let llm: Arc<dyn DdLlm> = Arc::new(StubDdLlm);

    let formation = Formation::new(FORMATION_LABEL)
        .seed(
            ContextKey::Seeds,
            "brief:northstar",
            brief,
            "investment-directive",
        )
        .agent(HuddleSeedSuggestor::from_plans(intent, plans))
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

    Ok((formation, budget, runner))
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

fn build_intent() -> IntentPacket {
    IntentPacket::new(
        format!("Run adaptive due diligence on {SUBJECT}"),
        Utc::now() + Duration::hours(12),
    )
}

fn build_self_organizing_runner()
-> Result<CollaborationRunner<ResearchSwarmAgent>, Box<dyn std::error::Error>> {
    let charter = CollaborationCharter::self_organizing();
    let team = TeamFormation::new(
        TeamFormationMode::OpenCall,
        vec![
            CollaborationMember::new("scout", "Scout Agent", CollaborationRole::Generalist),
            CollaborationMember::new("tech-diver", "Tech Diver", CollaborationRole::Generalist),
            CollaborationMember::new("watcher", "Watcher", CollaborationRole::Observer),
        ],
    );
    let participants = vec![
        ResearchSwarmAgent::new(
            "scout",
            "Scout Agent",
            CollaborationRole::Generalist,
            "commercial reconnaissance",
            ReasoningSystem::DomainModel,
        ),
        ResearchSwarmAgent::new(
            "tech-diver",
            "Tech Diver",
            CollaborationRole::Generalist,
            "technical depth",
            ReasoningSystem::CausalAnalysis,
        ),
        ResearchSwarmAgent::new(
            "watcher",
            "Watcher",
            CollaborationRole::Observer,
            "passive observation",
            ReasoningSystem::MlPrediction,
        ),
    ];

    Ok(CollaborationRunner::new(team, charter, participants)?)
}

fn seed_plans(
    runner: &CollaborationRunner<ResearchSwarmAgent>,
    intent: &IntentPacket,
) -> Vec<Plan> {
    runner
        .contributors()
        .iter()
        .filter_map(|participant| {
            let mut plan = Plan::new(
                intent,
                format!(
                    "{} seeds the swarm from a {} angle",
                    participant.display_name(),
                    participant.focus
                ),
            );
            plan.contributor = participant.system;

            match participant.id() {
                "scout" => {
                    plan.steps.push(PlanStep {
                        action: "breadth market map, customer proof, ideal customer profile".into(),
                        expected_effect: "establish commercial baseline and initial buyer pull"
                            .into(),
                    });
                    Some(plan)
                }
                "tech-diver" => {
                    plan.steps.push(PlanStep {
                        action:
                            "depth platform architecture, attack surface telemetry, integrations"
                                .into(),
                        expected_effect: "validate technical moat and implementation credibility"
                            .into(),
                    });
                    Some(plan)
                }
                _ => None,
            }
        })
        .collect()
}

fn print_formation(runner: &CollaborationRunner<ResearchSwarmAgent>) {
    println!("Formation: {FORMATION_LABEL}");
    println!("  Topology:       {:?}", runner.charter().topology);
    println!("  Formation mode: {:?}", runner.team().mode);
    println!("  Turn cadence:   {:?}", runner.turn_cadence());
    println!("  Contributors:   {}", runner.contributors().len());
    for participant in runner.contributors() {
        println!(
            "    - {} ({})",
            participant.display_name(),
            participant.focus
        );
    }
    println!();
}

fn print_result(result: &FormationResult, budget: &SharedBudget) {
    let converge_result: &ConvergeResult = &result.converge_result;
    println!(
        "Stop: {:?} | converged: {} | cycles: {}\n",
        converge_result.stop_reason, converge_result.converged, converge_result.cycles
    );

    let strategies = converge_result.context.get(ContextKey::Strategies);
    let signals = converge_result.context.get(ContextKey::Signals);
    let hypotheses = converge_result.context.get(ContextKey::Hypotheses);
    let evaluations = converge_result.context.get(ContextKey::Evaluations);
    let proposals = converge_result.context.get(ContextKey::Proposals);
    let summaries = consolidate_dd_hypotheses(hypotheses);
    let hooks = extract_hooks_from_facts(SUBJECT, &summaries, &HookPatterns::default());

    println!(
        "Searches used: {} | LLM calls used: {}",
        budget.used("searches"),
        budget.used("llm")
    );
    println!(
        "Strategies: {} | Signals: {} | Hypotheses: {} | Evaluations: {} | Proposals: {}\n",
        strategies.len(),
        signals.len(),
        hypotheses.len(),
        evaluations.len(),
        proposals.len()
    );

    println!("Strategies that shaped the loop:");
    for fact in strategies {
        println!("  - {}", fact.content);
    }
    println!();

    println!("Consolidated DD coverage:");
    for summary in &summaries {
        println!(
            "  - {:<12} confidence {:.2} | support {} | {}",
            summary.category, summary.confidence, summary.support_count, summary.claim
        );
    }
    println!();

    println!("Derived hooks:");
    println!(
        "  - business areas: {}",
        comma_or_none(&hooks.business_areas)
    );
    println!("  - competitors: {}", comma_or_none(&hooks.competitors));
    println!("  - investors: {}", comma_or_none(&hooks.investors));
    println!("  - regions: {}", comma_or_none(&hooks.regions));
    println!();

    if let Some(synthesis) = proposals.first()
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&synthesis.content)
    {
        println!("Recommendation:");
        println!(
            "  {}",
            value["recommendation"]
                .as_str()
                .unwrap_or("no recommendation")
        );
        println!();
        println!("Executive summary:");
        println!("  {}", value["summary"].as_str().unwrap_or("no summary"));
    }
}

fn comma_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.join(", ")
    }
}

#[derive(Debug, Clone)]
struct ResearchSwarmAgent {
    id: String,
    name: String,
    role: CollaborationRole,
    focus: &'static str,
    system: ReasoningSystem,
}

impl ResearchSwarmAgent {
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

impl CollaborationParticipant for ResearchSwarmAgent {
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

        if query.contains("arr growth")
            || query.contains("ownership")
            || query.contains("financial")
        {
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
            if prompt.contains("board update cites ARR growth")
                || prompt.contains("competitive win-loss review")
            {
                return Ok(serde_json::json!({
                    "facts": [
                        {
                            "claim": "Northstar Cyber reached $14.2M ARR in FY2025 and grew 58% year over year.",
                            "category": "financials",
                            "source_indices": [0],
                            "confidence": 0.91
                        },
                        {
                            "claim": "Northstar Cyber completed a Series B led by Granite Peak Ventures with RiverNorth participating.",
                            "category": "financials",
                            "source_indices": [1],
                            "confidence": 0.83
                        },
                        {
                            "claim": "Northstar Cyber most often competes with Wiz, Tenable, and Censys in exposure management evaluations.",
                            "category": "competition",
                            "source_indices": [2],
                            "confidence": 0.78
                        },
                        {
                            "claim": "Sources disagree on whether Northstar Cyber can displace broader CNAPP suites in large enterprise bundles.",
                            "category": "competition",
                            "source_indices": [3],
                            "confidence": 0.62
                        }
                    ]
                })
                .to_string());
            }

            return Ok(serde_json::json!({
                "facts": [
                    {
                        "claim": "Northstar Cyber packages weekly exposure triage workflows and executive-ready reporting for lean security teams.",
                        "category": "product",
                        "source_indices": [0],
                        "confidence": 0.88
                    },
                    {
                        "claim": "Reference customers include regional banks, insurers, and healthcare providers such as Fjord Bank and Veridian Health.",
                        "category": "customers",
                        "source_indices": [0],
                        "confidence": 0.77
                    },
                    {
                        "claim": "Customers buy Northstar Cyber for focused external exposure visibility rather than full CNAPP breadth.",
                        "category": "market",
                        "source_indices": [1],
                        "confidence": 0.74
                    },
                    {
                        "claim": "The platform collects passive DNS, certificate transparency, cloud asset, and SaaS exposure telemetry for attack surface management through native collectors and APIs.",
                        "category": "technology",
                        "source_indices": [2],
                        "confidence": 0.86
                    },
                    {
                        "claim": "Northstar Cyber routes exposure findings into analyst workflows and ticketing systems.",
                        "category": "technology",
                        "source_indices": [2],
                        "confidence": 0.71
                    }
                ]
            })
            .to_string());
        }

        if prompt.contains("What critical gaps remain?") {
            if prompt.contains("Missing expected categories:\nnone") {
                return Ok(r#"{"strategies":[]}"#.into());
            }

            return Ok(serde_json::json!({
                "strategies": [
                    {
                        "query": "ARR growth and ownership Northstar Cyber",
                        "mode": "breadth",
                        "reason": "economics and ownership are still missing"
                    },
                    {
                        "query": "competitive positioning and win-loss against exposure management vendors",
                        "mode": "depth",
                        "reason": "competitive strength is unresolved"
                    }
                ]
            })
            .to_string());
        }

        if prompt.contains("Produce a final analysis as JSON") {
            return Ok(serde_json::json!({
                "summary": "Northstar Cyber shows credible pull in regulated mid-market accounts with a narrowly focused attack surface management product. The technical evidence points to a differentiated telemetry pipeline and fast deployment profile, while the financial evidence suggests a real but still early commercial engine. Competitive evidence is good enough to proceed, but the up-market story is not fully settled.",
                "market_analysis": "The company sits in external exposure management rather than broad CNAPP suites. That focus appears to resonate with buyers who value fast deployment and clear external visibility.",
                "competitive_landscape": "Northstar Cyber most often collides with Wiz, Tenable, and Censys. The field evidence is directionally positive in the mid-market, but sources disagree on displacement power in large bundled enterprise deals.",
                "technology_assessment": "The platform ingests passive DNS, certificate transparency, cloud asset, and SaaS exposure telemetry through native collectors and APIs, then routes findings into analyst workflows. That is a real product surface, not slideware.",
                "risk_factors": [
                    "Enterprise up-market fit remains contested",
                    "Commercial scale is still proving out despite strong growth"
                ],
                "growth_opportunities": [
                    "Expand within regulated verticals already showing buyer pull",
                    "Use the focused product story to partner into broader security stacks"
                ],
                "recommendation": "Proceed to management meeting and reference calls, with specific pressure-testing on enterprise expansion, retention quality, and sales efficiency."
            })
            .to_string());
        }

        Err(DdError::BadResponse {
            provider: "stub-llm".into(),
            detail: "unexpected prompt shape".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{Fact, StopReason};
    use organism_pack::{CollaborationTopology, DdFactSummary, TeamFormationMode};

    fn has_category(hypotheses: &[Fact], category: &str) -> bool {
        hypotheses.iter().any(|fact| {
            serde_json::from_str::<serde_json::Value>(&fact.content)
                .ok()
                .and_then(|value| value["category"].as_str().map(str::to_owned))
                .is_some_and(|current| current == category)
        })
    }

    fn summaries(result: &FormationResult) -> Vec<DdFactSummary> {
        consolidate_dd_hypotheses(result.converge_result.context.get(ContextKey::Hypotheses))
    }

    #[tokio::test]
    async fn self_organizing_formation_converges_with_gap_filled_coverage() {
        let (result, _, runner) = run_due_diligence().await.expect("loop should converge");

        assert_eq!(result.label, FORMATION_LABEL);
        assert_eq!(
            runner.charter().topology,
            CollaborationTopology::SelfOrganizing
        );
        assert_eq!(runner.team().mode, TeamFormationMode::OpenCall);
        assert_eq!(runner.contributors().len(), 2);
        assert!(result.converge_result.converged);
        assert!(matches!(
            result.converge_result.stop_reason,
            StopReason::Converged
        ));
        assert!(result.converge_result.cycles >= 8);

        let hypotheses = result.converge_result.context.get(ContextKey::Hypotheses);
        assert!(has_category(hypotheses, "product"));
        assert!(has_category(hypotheses, "customers"));
        assert!(has_category(hypotheses, "market"));
        assert!(has_category(hypotheses, "technology"));
        assert!(has_category(hypotheses, "financials"));
        assert!(has_category(hypotheses, "competition"));

        let strategies = result.converge_result.context.get(ContextKey::Strategies);
        assert!(strategies.len() >= 4);
        assert!(
            strategies
                .iter()
                .any(|fact| fact.content.contains("Scout Agent"))
        );
        assert!(
            strategies
                .iter()
                .any(|fact| fact.content.contains("Tech Diver"))
        );

        let proposals = result.converge_result.context.get(ContextKey::Proposals);
        assert_eq!(proposals.len(), 1);
    }

    #[tokio::test]
    async fn dd_loop_flags_competition_contradiction_and_extracts_hooks() {
        let (result, _, _) = run_due_diligence().await.expect("loop should converge");

        let evaluations = result.converge_result.context.get(ContextKey::Evaluations);
        assert!(
            evaluations
                .iter()
                .any(|fact| fact.id.starts_with("contradiction-competition-"))
        );

        let hooks =
            extract_hooks_from_facts(SUBJECT, &summaries(&result), &HookPatterns::default());
        assert!(
            hooks
                .business_areas
                .iter()
                .any(|area| area == "Attack Surface Management")
        );
        assert!(hooks.competitors.iter().any(|name| name == "Wiz"));
    }
}
