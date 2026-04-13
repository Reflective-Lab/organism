// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Interactive Nordic Market Expansion spike binary.
//!
//! Run with:
//!   ANTHROPIC_API_KEY=... BRAVE_API_KEY=... cargo run --bin market_expansion

use std::io::{self, Write};

use converge_core::gates::hitl::GateDecision;
use converge_core::{Context, ContextKey, RunResult};
use converge_provider::ProviderRegistry;
use organism_application::spike_market::scenario::{
    ExperimentTopic, SpikeConfig, build_consensus_engine_with_config,
    build_experiment_engine_from_registry, candidate_cities, discover_providers,
};

fn main() {
    // Load .env file if present (silently ignore if missing)
    if let Ok(path) = dotenvy::dotenv() {
        eprintln!("  Loaded env from {}", path.display());
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║  Spike 2: Nordic Market Expansion Decision (Interactive)            ║");
    println!("║                                                                      ║");
    println!("║  Should the company open a European R&D center, and if so, where?    ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    // ── Discover available providers ──────────────────────────────────
    let registry = ProviderRegistry::from_env();
    let available = registry.available_providers();
    if available.is_empty() {
        eprintln!(
            "✗ No API keys found. Set at least one LLM provider key (e.g. ANTHROPIC_API_KEY) and BRAVE_API_KEY."
        );
        std::process::exit(1);
    }
    println!("  Detected API keys for: {}", available.join(", "));

    // ── Interactive configuration ───────────────────────────────────
    let config = prompt_config();

    println!();
    print_phase("Configuration Summary");
    println!("  Budget ceiling:     ≤ {}K EUR", config.budget_k);
    println!("  Min talent score:   ≥ {}", config.min_talent);
    println!("  HITL threshold:     {:.2}", config.hitl_threshold);
    println!(
        "  Agent confidence:   {:.2}",
        config.recommendation_confidence
    );
    println!();

    let will_hitl = config.recommendation_confidence <= config.hitl_threshold;
    if will_hitl {
        println!(
            "  → Confidence ({:.2}) ≤ threshold ({:.2}): the engine WILL pause for your approval.",
            config.recommendation_confidence, config.hitl_threshold
        );
    } else {
        println!(
            "  → Confidence ({:.2}) > threshold ({:.2}): the engine will auto-approve — no HITL pause.",
            config.recommendation_confidence, config.hitl_threshold
        );
    }

    // Show which cities are excluded by constraints
    let cities = candidate_cities();
    let excluded: Vec<&str> = cities
        .iter()
        .filter(|c| c.entry_cost_k > config.budget_k || c.talent_score < config.min_talent)
        .map(|c| c.name.as_str())
        .collect();
    let eligible: Vec<&str> = cities
        .iter()
        .filter(|c| c.entry_cost_k <= config.budget_k && c.talent_score >= config.min_talent)
        .map(|c| c.name.as_str())
        .collect();

    if excluded.is_empty() {
        println!("  → All 8 cities are eligible under these constraints.");
    } else {
        println!(
            "  → {} cities excluded by constraints: {}",
            excluded.len(),
            excluded.join(", ")
        );
        println!("  → {} eligible: {}", eligible.len(), eligible.join(", "));
    }

    if eligible.is_empty() {
        println!(
            "\n  ✗ No cities qualify — CP-SAT will return Infeasible. Adjust your constraints."
        );
        std::process::exit(1);
    }

    // ── Setup providers via capability matching ────────────────────
    println!();
    print_phase("Selecting providers by capability (with health check)");
    println!("  Each candidate gets a 1-token ping to verify the key/quota works.");
    println!("  If it fails, the next best candidate is tried automatically.");
    println!();

    let providers = match discover_providers(&registry) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("✗ Provider discovery failed: {e}");
            std::process::exit(1);
        }
    };

    let ps = &providers.planner_selection;
    println!(
        "  ✓ Planner  (fast/cheap):  {}/{} — fitness {:.3}",
        ps.selected.provider, ps.selected.model, ps.fitness.total
    );
    println!("               breakdown:   {}", ps.fitness);
    let as_ = &providers.analyst_selection;
    println!(
        "  ✓ Analyst  (powerful):    {}/{} — fitness {:.3}",
        as_.selected.provider, as_.selected.model, as_.fitness.total
    );
    println!("               breakdown:   {}", as_.fitness);
    println!("  ✓ Search:                 Brave Web Search");

    // ── Show candidate cities ───────────────────────────────────────
    println!();
    print_phase("Candidate Cities (static baseline data)");
    println!("  These scores are the starting point. The LLM experiments will produce");
    println!("  their own scores from live web research — potentially overriding these.");
    println!();
    println!(
        "  {:<12} {:>8} {:>7} {:>7} {:>5} {:>8} {}",
        "City", "Cost(€K)", "Talent", "Market", "Tax", "Score", ""
    );
    println!("  {}", "─".repeat(62));
    for city in &cities {
        let excluded_marker = if city.entry_cost_k > config.budget_k {
            " ← over budget"
        } else if city.talent_score < config.min_talent {
            " ← below talent min"
        } else {
            ""
        };
        println!(
            "  {:<12} {:>8} {:>7} {:>7} {:>5} {:>8.1}{}",
            city.name,
            city.entry_cost_k,
            city.talent_score,
            city.market_access,
            city.tax_incentive,
            city.weighted_score(),
            excluded_marker
        );
    }

    wait_for_enter("\nPress Enter to start the 3 research experiments...");

    // ── Run experiments ─────────────────────────────────────────────
    println!();
    println!("  Each experiment runs its own convergence engine with 3 agents:");
    println!(
        "    1. SearchPlannerAgent ({}/{}) — turns the question into 3 search queries",
        providers.planner_selection.selected.provider, providers.planner_selection.selected.model
    );
    println!("    2. WebSearchAgent (Brave)     — executes the searches, collects results");
    println!(
        "    3. ResearchAnalystAgent ({}/{}) — reads results, scores all 8 cities",
        providers.analyst_selection.selected.provider, providers.analyst_selection.selected.model
    );
    println!("  The engine loops until all 3 agents have fired and no new data changes.");

    let topics = [
        ExperimentTopic::MarketDemand,
        ExperimentTopic::CompetitiveLandscape,
        ExperimentTopic::GoToMarketCost,
    ];

    let mut experiment_contexts = Vec::new();
    let mut total_cycles = 0u32;

    for (i, topic) in topics.iter().enumerate() {
        println!();
        print_phase(&format!(
            "Experiment {}/3: {}",
            i + 1,
            topic.name().replace('_', " ")
        ));
        println!("  Research question:");
        for line in textwrap(topic.research_question(), 70) {
            println!("    {line}");
        }
        println!();

        let mut engine = build_experiment_engine_from_registry(*topic, &providers);

        print!("  Running convergence loop");
        io::stdout().flush().ok();

        let result = engine.run(Context::new());

        match result {
            Ok(result) => {
                total_cycles += result.cycles;
                println!(" ✓ ({} cycles)", result.cycles);

                // Show search queries
                for fact in result.context.get(ContextKey::Seeds) {
                    if fact.id.starts_with("search_queries:") {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                            if let Some(queries) = v.get("queries").and_then(|q| q.as_array()) {
                                println!("  Haiku generated queries:");
                                for q in queries {
                                    println!("    • {}", q.as_str().unwrap_or("?"));
                                }
                            }
                        }
                    }
                }

                // Show analysis
                for fact in result.context.get(ContextKey::Hypotheses) {
                    if fact.id.starts_with("analysis:") {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                            if let Some(summary) = v.get("summary").and_then(|s| s.as_str()) {
                                println!("  Analyst's analysis:");
                                for line in textwrap(summary, 68) {
                                    println!("    {line}");
                                }
                            }
                            if let Some(scores) = v.get("scores").and_then(|s| s.as_object()) {
                                println!(
                                    "  Analyst's city scores (blended with baseline in consensus):"
                                );
                                let mut sorted: Vec<_> = scores.iter().collect();
                                sorted.sort_by(|a, b| {
                                    b.1.as_f64()
                                        .unwrap_or(0.0)
                                        .partial_cmp(&a.1.as_f64().unwrap_or(0.0))
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                                for (city, score) in &sorted {
                                    println!(
                                        "    {:<12} {:>3}",
                                        city,
                                        score
                                            .as_f64()
                                            .map_or("?".to_string(), |s| format!("{s:.0}"))
                                    );
                                }
                            } else if v.get("raw_response").is_some() {
                                println!(
                                    "  ⚠ Analyst returned non-structured response (no city scores parsed)."
                                );
                                println!("    Baseline scores will be used for this dimension.");
                            }
                        }
                    }
                }

                // Show diagnostics/errors
                for fact in result.context.get(ContextKey::Diagnostic) {
                    println!("  ⚠ Diagnostic: {}", fact.content);
                }

                experiment_contexts.push(result.context);
            }
            Err(e) => {
                println!(" ✗ FAILED: {e}");
                eprintln!("Experiment {} failed, aborting.", topic.name());
                std::process::exit(1);
            }
        }
    }

    wait_for_enter("\nPress Enter to run the consensus engine...");

    // ── Build consensus context ─────────────────────────────────────
    println!();
    print_phase("Consensus Engine");
    println!("  The consensus engine takes the 3 experiment outputs and runs 4 agents:");
    println!("    1. AggregationAgent  — averages the 3 experiment scores per city");
    println!("    2. VotingAgent       — Borda-ranks cities by composite score");
    println!("    3. OptimizationAgent — CP-SAT solver picks the best city within constraints");
    println!(
        "       (budget ≤ {}K EUR, talent ≥ {})",
        config.budget_k, config.min_talent
    );
    println!("    4. RecommendationAgent — packages result as a ProposedFact");
    println!(
        "       (confidence {:.2})",
        config.recommendation_confidence
    );
    println!();
    println!("  Invariants enforced:");
    println!("    • BudgetConstraint (Structural)   — blocks over-budget recommendations");
    println!("    • MinimumScore (Structural)        — blocks under-talent recommendations");
    println!("    • ConsensusRequired (Acceptance)   — all 3 experiments must contribute");
    println!();
    println!("  HITL policy: threshold = {:.2}", config.hitl_threshold);
    if will_hitl {
        println!(
            "  → Agent confidence {:.2} ≤ {:.2} threshold: expect a pause for your decision.",
            config.recommendation_confidence, config.hitl_threshold
        );
    } else {
        println!(
            "  → Agent confidence {:.2} > {:.2} threshold: will auto-promote, no pause.",
            config.recommendation_confidence, config.hitl_threshold
        );
    }
    println!();

    let mut consensus_ctx = Context::new();
    for exp_ctx in &experiment_contexts {
        for fact in exp_ctx.get(ContextKey::Hypotheses) {
            if fact.id.starts_with("analysis:") {
                let _ = consensus_ctx.add_fact(fact.clone());
            }
        }
    }

    let mut engine = build_consensus_engine_with_config(&config);
    let run_result = engine.run_with_hitl(consensus_ctx);

    match run_result {
        RunResult::HitlPause(pause) => {
            println!("  ┌─────────────────────────────────────────────────────────────┐");
            println!("  │  ⏸  HITL GATE — Human Approval Required                    │");
            println!("  └─────────────────────────────────────────────────────────────┘");
            println!();
            println!("  The RecommendationAgent emitted a ProposedFact — not a Fact.");
            println!(
                "  Because its confidence ({:.2}) ≤ the HITL threshold ({:.2}),",
                config.recommendation_confidence, config.hitl_threshold
            );
            println!("  the engine paused convergence and is asking you to decide.");
            println!();
            println!("  If you approve: the proposal becomes a Fact, convergence resumes.");
            println!("  If you reject:  the proposal is discarded, a Diagnostic fact is");
            println!("    recorded so the agent won't re-propose, and the engine converges");
            println!("    without a recommendation.");
            println!();
            println!("  Gate ID:    {}", pause.request.gate_id);
            println!("  Cycle:      {}", pause.cycle);
            println!();

            // Parse and display the proposal
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&pause.request.summary) {
                if let Some(rec) = v.get("recommendation").and_then(|r| r.as_str()) {
                    println!("  Recommendation: {rec}");
                }
                if let Some(conf) = v.get("confidence").and_then(|c| c.as_f64()) {
                    println!("  Confidence:     {conf:.2}");
                }
                if let Some(rationale) = v.get("rationale").and_then(|r| r.as_str()) {
                    println!("  Rationale:");
                    for line in textwrap(rationale, 64) {
                        println!("    {line}");
                    }
                }
            } else {
                println!(
                    "  Proposal: {}",
                    &pause.request.summary[..pause.request.summary.len().min(200)]
                );
            }

            println!();
            println!("  ─────────────────────────────────────────────────────────────");

            let gate_id = pause.request.gate_id.clone();
            let decision = prompt_hitl_decision(&gate_id);

            let decision_desc = if decision.is_approved() {
                "APPROVED → proposal promoted to Fact, convergence resumes"
            } else {
                "REJECTED → proposal discarded, engine converges without recommendation"
            };
            println!("\n  Decision: {decision_desc}\n");

            let resume_result = engine.resume(*pause, decision);
            handle_final_result(resume_result, total_cycles, &config);
        }
        RunResult::Complete(Ok(result)) => {
            total_cycles += result.cycles;
            println!(
                "  ✓ Consensus converged in {} cycles — no HITL pause (confidence > threshold)",
                result.cycles
            );
            print_final_output(&result.context, total_cycles, &config);
        }
        RunResult::Complete(Err(e)) => {
            eprintln!("  ✗ Consensus failed: {e}");
            std::process::exit(1);
        }
    }
}

// ── Interactive config ──────────────────────────────────────────────

fn prompt_config() -> SpikeConfig {
    let defaults = SpikeConfig::default();

    println!("▸ Experiment Configuration");
    println!("  Configure the parameters that control the decision pipeline.");
    println!("  Press Enter to accept the default value shown in [brackets].\n");

    let budget_k = prompt_u32(
        "  Budget ceiling (K EUR)",
        defaults.budget_k,
        "  The maximum entry cost a city can have. Cities above this are excluded\n  \
         by the CP-SAT solver and the BudgetConstraint invariant.\n  \
         Lower = fewer eligible cities. Try 300 to exclude London, Zurich, Stockholm, Copenhagen.",
    );

    let min_talent = prompt_u32(
        "  Minimum talent score",
        defaults.min_talent,
        "  Cities with talent below this are excluded by the CP-SAT solver\n  \
         and the MinimumScore invariant.\n  \
         Higher = stricter filter. Try 85 to only allow top-tier talent cities.",
    );

    let hitl_threshold = prompt_f64(
        "  HITL confidence threshold",
        defaults.hitl_threshold,
        "  When a ProposedFact has confidence ≤ this value, the engine pauses\n  \
         and asks for human approval. Higher threshold = more things get gated.\n  \
         Set to 0.0 to disable HITL entirely. Set to 1.0 to gate everything.",
    );

    let recommendation_confidence = prompt_f64(
        "  Recommendation confidence",
        defaults.recommendation_confidence,
        "  The confidence the RecommendationAgent assigns to its output.\n  \
         If this ≤ HITL threshold → you'll be asked to approve.\n  \
         If this > HITL threshold → auto-promoted, no human involvement.\n  \
         Try 0.95 to skip HITL, or 0.50 for low confidence.",
    );

    SpikeConfig {
        budget_k,
        min_talent,
        hitl_threshold,
        recommendation_confidence,
    }
}

fn prompt_u32(label: &str, default: u32, explanation: &str) -> u32 {
    println!("{explanation}");
    print!("{label} [{default}]: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();
    let value = if input.is_empty() {
        default
    } else {
        input.parse().unwrap_or_else(|_| {
            println!("  Invalid number, using default {default}");
            default
        })
    };
    println!();
    value
}

fn prompt_f64(label: &str, default: f64, explanation: &str) -> f64 {
    println!("{explanation}");
    print!("{label} [{default:.2}]: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let input = input.trim();
    let value = if input.is_empty() {
        default
    } else {
        input.parse().unwrap_or_else(|_| {
            println!("  Invalid number, using default {default:.2}");
            default
        })
    };
    println!();
    value
}

// ── HITL prompt ─────────────────────────────────────────────────────

fn prompt_hitl_decision(gate_id: &converge_core::types::id::GateId) -> GateDecision {
    loop {
        print!("  Your decision [a]pprove / [r]eject? > ");
        io::stdout().flush().ok();

        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "a" | "approve" => {
                return GateDecision::approve(gate_id.clone(), "human-operator");
            }
            "r" | "reject" => {
                print!("  Reason (optional): ");
                io::stdout().flush().ok();
                let mut reason = String::new();
                io::stdin().read_line(&mut reason).ok();
                let reason = reason.trim();
                let reason = if reason.is_empty() {
                    None
                } else {
                    Some(reason.to_string())
                };
                return GateDecision::reject(gate_id.clone(), "human-operator", reason);
            }
            _ => {
                println!("  Please enter 'a' or 'r'.");
            }
        }
    }
}

// ── Output ──────────────────────────────────────────────────────────

fn handle_final_result(result: RunResult, mut total_cycles: u32, config: &SpikeConfig) {
    match result {
        RunResult::Complete(Ok(result)) => {
            total_cycles += result.cycles;
            println!(
                "  ✓ Consensus converged in {} cycles after HITL decision",
                result.cycles
            );
            print_final_output(&result.context, total_cycles, config);
        }
        RunResult::Complete(Err(e)) => {
            eprintln!("  ✗ Consensus failed after HITL: {e}");
            std::process::exit(1);
        }
        RunResult::HitlPause(_) => {
            eprintln!("  ✗ Unexpected second HITL pause");
            std::process::exit(1);
        }
    }
}

fn print_final_output(ctx: &Context, total_cycles: u32, config: &SpikeConfig) {
    println!();

    // Ranking
    if let Some(ranking_fact) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "city_ranking")
    {
        if let Ok(ranking) = serde_json::from_str::<Vec<serde_json::Value>>(&ranking_fact.content) {
            // Count how many experiments contributed scores
            let experiment_count = ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .filter(|f| {
                    f.id.starts_with("analysis:")
                        && serde_json::from_str::<serde_json::Value>(&f.content)
                            .ok()
                            .and_then(|v| v.get("scores")?.as_object().map(|_| ()))
                            .is_some()
                })
                .count();
            print_phase(&format!(
                "City Rankings (composite: {experiment_count}/3 experiments + baseline)"
            ));
            for entry in &ranking {
                let rank = entry.get("rank").and_then(|v| v.as_u64()).unwrap_or(0);
                let city = entry.get("city").and_then(|v| v.as_str()).unwrap_or("?");
                let score = entry
                    .get("composite_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let medal = match rank {
                    1 => " 🥇",
                    2 => " 🥈",
                    3 => " 🥉",
                    _ => "",
                };
                println!("  #{rank}: {city:<12} (score: {score:.1}){medal}");
            }
            println!();
        }
    }

    // Optimization
    if let Some(opt) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "optimization_result")
    {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&opt.content) {
            print_phase("CP-SAT Optimization Result");
            println!("  The SAT solver picked the highest-scoring city that satisfies:");
            println!(
                "    • entry cost ≤ {}K EUR   • talent ≥ {}   • exactly 1 city",
                config.budget_k, config.min_talent
            );
            println!();
            println!(
                "  Status:       {}",
                v.get("status").and_then(|s| s.as_str()).unwrap_or("?")
            );
            println!(
                "  Selected:     {}",
                v.get("selected_city")
                    .and_then(|s| s.as_str())
                    .unwrap_or("?")
            );
            if let Some(d) = v.get("city_details") {
                println!(
                    "  Entry Cost:   {}K EUR",
                    d.get("entry_cost_k").and_then(|v| v.as_u64()).unwrap_or(0)
                );
                println!(
                    "  Talent Score: {}",
                    d.get("talent_score").and_then(|v| v.as_u64()).unwrap_or(0)
                );
                println!(
                    "  Market:       {}",
                    d.get("market_access").and_then(|v| v.as_u64()).unwrap_or(0)
                );
                println!(
                    "  Tax:          {}",
                    d.get("tax_incentive").and_then(|v| v.as_u64()).unwrap_or(0)
                );
            }
            println!(
                "  Solve time:   {:.3}s",
                v.get("wall_time_secs")
                    .and_then(|s| s.as_f64())
                    .unwrap_or(0.0)
            );
            println!();
        }
    }

    // Recommendation
    if let Some(rec) = ctx
        .get(ContextKey::Evaluations)
        .iter()
        .find(|f| f.id == "recommendation")
    {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&rec.content) {
            let city = v
                .get("selected_city")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            println!("╔══════════════════════════════════════════════════════════════════════╗");
            println!(
                "║  ✓ FINAL RECOMMENDATION: Establish R&D center in {:<17} ║",
                city
            );
            println!("╚══════════════════════════════════════════════════════════════════════╝");
            if let Some(rationale) = v.get("rationale").and_then(|r| r.as_str()) {
                println!();
                for line in textwrap(rationale, 72) {
                    println!("  {line}");
                }
            }
        }
    } else {
        println!("╔══════════════════════════════════════════════════════════════════════╗");
        println!("║  ⚠ No recommendation — HITL gate was rejected.                     ║");
        println!("║  The engine converged without producing a final recommendation.     ║");
        println!("╚══════════════════════════════════════════════════════════════════════╝");
    }

    println!();
    println!("  Total convergence cycles: {total_cycles}");
    println!();
}

// ── Helpers ─────────────────────────────────────────────────────────

fn print_phase(title: &str) {
    println!("▸ {title}");
}

fn wait_for_enter(msg: &str) {
    print!("{msg}");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
}

fn textwrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > width && !current.is_empty() {
            lines.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
