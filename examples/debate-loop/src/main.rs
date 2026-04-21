// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Debate Loop — real LLM-backed planning with adversarial review.
//!
//! Planner proposes → Skeptic challenges → Planner revises → Converge.
//!
//! Both agents call Claude via the Anthropic API. The debate IS the
//! convergence loop — no special machinery, just Suggestors reading
//! and writing to shared context until fixed point.
//!
//! Requires: ANTHROPIC_API_KEY environment variable.

use converge_kernel::{AgentEffect, Context, ContextKey, Engine, ProposedFact, Suggestor};

use organism_pack::{CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MEDIUM, Severity, SkepticismKind};

// ── LLM Client ─────────────────────────────────────────────────────

fn call_claude(system: &str, user: &str) -> String {
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return format!(
            "[MOCK — set ANTHROPIC_API_KEY for real LLM] System: {system} | User: {user}"
        );
    }

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": system,
            "messages": [{ "role": "user", "content": user }]
        }))
        .send();

    match response {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                json["content"][0]["text"]
                    .as_str()
                    .unwrap_or("(no response)")
                    .to_string()
            } else {
                "(failed to parse response)".to_string()
            }
        }
        Err(e) => format!("(API error: {e})"),
    }
}

// ── Planner Agent ──────────────────────────────────────────────────

/// LLM-backed planner. On first run, proposes a plan from the seed intent.
/// On subsequent runs (after challenges appear), revises the plan.
struct LlmPlannerAgent;

#[async_trait::async_trait]
impl Suggestor for LlmPlannerAgent {
    fn name(&self) -> &str {
        "llm_planner"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Run if: seeds exist AND (no plan yet, OR challenges exist without a revised plan)
        if !ctx.has(ContextKey::Seeds) {
            return false;
        }
        let proposals = ctx.get(ContextKey::Proposals);
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_plan = proposals.iter().any(|p| p.id.starts_with("plan:"));
        let has_challenges = evaluations.iter().any(|e| e.id.starts_with("challenge:"));
        let has_revised = proposals.iter().any(|p| p.id.starts_with("plan:revised"));

        !has_plan || (has_challenges && !has_revised)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let intent = seeds
            .first()
            .map(|s| s.content.as_str())
            .unwrap_or("(no intent)");

        let evaluations = ctx.get(ContextKey::Evaluations);
        let challenges: Vec<String> = evaluations
            .iter()
            .filter(|e| e.id.starts_with("challenge:"))
            .map(|e| e.content.clone())
            .collect();

        if challenges.is_empty() {
            // First pass: propose initial plan
            println!("  [Planner] Proposing initial plan...");
            let plan = call_claude(
                "You are an organizational planner. Given a business intent, produce a concrete \
                 3-5 step action plan. Be specific about who does what and when. \
                 Return ONLY the plan steps, numbered.",
                &format!("Create an action plan for: {intent}"),
            );
            println!("  [Planner] {}", truncate(&plan, 200));

            AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Proposals,
                    "plan:initial",
                    serde_json::json!({
                        "version": 1,
                        "plan": plan,
                        "intent": intent,
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(0.5)
                .adjust_confidence(CONFIDENCE_STEP_MAJOR),
            )
        } else {
            // Revision pass: address challenges
            let challenge_text = challenges.join("\n\n");
            println!(
                "  [Planner] Revising plan to address {} challenge(s)...",
                challenges.len()
            );

            let proposals = ctx.get(ContextKey::Proposals);
            let original_plan = proposals
                .iter()
                .find(|p| p.id == "plan:initial")
                .map(|p| p.content.clone())
                .unwrap_or_default();

            let revised = call_claude(
                "You are an organizational planner. Your initial plan was challenged by skeptics. \
                 Revise the plan to address their concerns. Keep what works, fix what was challenged. \
                 Return ONLY the revised plan steps, numbered.",
                &format!(
                    "Original plan:\n{original_plan}\n\nChallenges raised:\n{challenge_text}\n\n\
                     Revise the plan to address these challenges."
                ),
            );
            println!("  [Planner] Revised: {}", truncate(&revised, 200));

            AgentEffect::with_proposal(
                ProposedFact::new(
                    ContextKey::Proposals,
                    "plan:revised",
                    serde_json::json!({
                        "version": 2,
                        "plan": revised,
                        "addressed_challenges": challenges.len(),
                        "intent": proposals.first().map(|p| p.content.as_str()).unwrap_or(""),
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(0.7)
                .adjust_confidence(CONFIDENCE_STEP_MEDIUM),
            )
        }
    }
}

// ── Skeptic Agent ──────────────────────────────────────────────────

/// LLM-backed adversarial skeptic. Reads the current plan and
/// challenges weak assumptions, missing constraints, or operational risks.
struct LlmSkepticAgent;

#[async_trait::async_trait]
impl Suggestor for LlmSkepticAgent {
    fn name(&self) -> &str {
        "llm_skeptic"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Run if: there's a plan but no challenges yet for the current version
        let proposals = ctx.get(ContextKey::Proposals);
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_initial_plan = proposals.iter().any(|p| p.id == "plan:initial");
        let has_revised_plan = proposals.iter().any(|p| p.id == "plan:revised");
        let has_challenges = evaluations.iter().any(|e| e.id.starts_with("challenge:"));
        let has_final_review = evaluations.iter().any(|e| e.id == "challenge:final-review");

        // Challenge initial plan (no challenges yet)
        // OR review revised plan (challenges exist but no final review)
        (has_initial_plan && !has_challenges) || (has_revised_plan && !has_final_review)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let evaluations = ctx.get(ContextKey::Evaluations);

        let has_revised = proposals.iter().any(|p| p.id == "plan:revised");
        let is_final_review =
            has_revised && evaluations.iter().any(|e| e.id.starts_with("challenge:"));

        let plan_fact = if has_revised {
            proposals.iter().find(|p| p.id == "plan:revised")
        } else {
            proposals.iter().find(|p| p.id == "plan:initial")
        };

        let plan_content = plan_fact.map(|p| p.content.as_str()).unwrap_or("(no plan)");

        let review_type = if is_final_review {
            "final review of revised plan"
        } else {
            "initial challenge of proposed plan"
        };

        println!("  [Skeptic] Running {review_type}...");

        let critique = call_claude(
            "You are an adversarial reviewer for organizational plans. Your job is to find \
             weaknesses, challenged assumptions, missing safeguards, and operational risks. \
             Be constructive but thorough. For each issue, classify it as:\n\
             - BLOCKER: plan cannot proceed without fixing this\n\
             - WARNING: plan can proceed but has significant risk\n\
             - ADVISORY: something to monitor\n\n\
             If the plan is solid after revision, say 'APPROVED — no blocking issues remain.' \
             Return your findings as a numbered list.",
            &format!("Review this plan ({review_type}):\n\n{plan_content}"),
        );

        println!("  [Skeptic] {}", truncate(&critique, 200));

        let is_approved = critique.to_lowercase().contains("approved")
            && !critique.to_lowercase().contains("blocker");

        let challenge_id = if is_final_review {
            "challenge:final-review"
        } else {
            "challenge:initial"
        };

        let kind = if is_approved {
            SkepticismKind::ConstraintChecking
        } else if critique.to_lowercase().contains("assumption") {
            SkepticismKind::AssumptionBreaking
        } else if critique.to_lowercase().contains("cost")
            || critique.to_lowercase().contains("budget")
        {
            SkepticismKind::EconomicSkepticism
        } else if critique.to_lowercase().contains("feasib")
            || critique.to_lowercase().contains("capacity")
        {
            SkepticismKind::OperationalSkepticism
        } else {
            SkepticismKind::ConstraintChecking
        };

        let severity = if is_approved {
            Severity::Advisory
        } else if critique.to_lowercase().contains("blocker") {
            Severity::Blocker
        } else {
            Severity::Warning
        };

        AgentEffect::with_proposal(
            ProposedFact::new(
                ContextKey::Evaluations,
                challenge_id,
                serde_json::json!({
                    "review_type": review_type,
                    "kind": format!("{kind:?}"),
                    "severity": format!("{severity:?}"),
                    "critique": critique,
                    "approved": is_approved,
                })
                .to_string(),
                self.name(),
            )
            .with_confidence(0.75)
            .adjust_confidence(if is_approved {
                CONFIDENCE_STEP_MEDIUM
            } else {
                -CONFIDENCE_STEP_MEDIUM
            }),
        )
    }
}

// ── Main ───────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    println!("=== Organism Debate Loop ===");
    println!("    Planner (LLM) vs Skeptic (LLM) → Converge\n");

    let has_key = std::env::var("ANTHROPIC_API_KEY").is_ok();
    if has_key {
        println!("    Mode: LIVE (calling Claude API)");
    } else {
        println!("    Mode: MOCK (set ANTHROPIC_API_KEY for real LLM debate)");
    }
    println!();

    let mut engine = Engine::new();
    engine.register_suggestor(LlmPlannerAgent);
    engine.register_suggestor(LlmSkepticAgent);

    let intent = "Hire a senior Rust engineer for the Converge team within 60 days, \
                  budget $180k-220k, must pass security clearance, remote-first but \
                  willing to travel quarterly to Stockholm.";

    println!("Intent: {intent}\n");
    println!("--- Debate begins ---\n");

    let mut ctx = converge_kernel::ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "intent-1", intent.to_string());

    match engine.run(ctx).await {
        Ok(result) => {
            println!("\n--- Debate converged ---\n");

            // Show the final state
            let proposals = result.context.get(ContextKey::Proposals);
            let evaluations = result.context.get(ContextKey::Evaluations);

            println!("Proposals ({}):", proposals.len());
            for p in proposals {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&p.content) {
                    let version = json.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
                    let plan = json.get("plan").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("  [v{version}] {}", truncate(plan, 300));
                }
            }

            println!("\nChallenges ({}):", evaluations.len());
            for e in evaluations {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&e.content) {
                    let review_type = json
                        .get("review_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let severity = json.get("severity").and_then(|v| v.as_str()).unwrap_or("?");
                    let approved = json
                        .get("approved")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let critique = json.get("critique").and_then(|v| v.as_str()).unwrap_or("?");
                    println!(
                        "  [{severity}] {review_type}{}",
                        if approved { " → APPROVED" } else { "" }
                    );
                    println!("    {}", truncate(critique, 300));
                }
            }

            let final_review = evaluations
                .iter()
                .find(|e| e.id == "challenge:final-review");
            if let Some(review) = final_review
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&review.content)
            {
                let approved = json
                    .get("approved")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                println!(
                    "\nVerdict: {}",
                    if approved {
                        "PLAN APPROVED"
                    } else {
                        "PLAN NEEDS WORK"
                    }
                );
            }
        }
        Err(e) => println!("Engine error: {e}"),
    }

    println!("\n=== Done ===");
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ↵ ");
    if s.len() <= max {
        s
    } else {
        format!("{}...", &s[..max])
    }
}
