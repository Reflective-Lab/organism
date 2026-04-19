//! Converge suggestor patterns for Organism planning.
//!
//! These bridge Organism's planning concepts into Converge's convergence
//! loop. Instead of running planning as a standalone step, these patterns
//! let planning participate as reactive suggestors in the Engine cycle.
//!
//! # Patterns
//!
//! - [`HuddleSeedSuggestor`] — seeds initial strategies from a Huddle into
//!   Converge context on the first cycle.
//! - [`SharedBudget`] — cross-suggestor resource tracking for bounded loops.

use std::sync::{Arc, Mutex};

use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use organism_intent::IntentPacket;

use crate::{Plan, ReasoningSystem};

// ── Huddle Seed Suggestor ─────────────────────────────────────────

/// Seeds Organism planning output into Converge context as strategy facts.
///
/// On first activation (no strategies in context), runs the provided plans
/// and emits each plan's steps as strategy `ProposedFact`s. After seeding,
/// it never fires again — downstream suggestors react to the strategies.
pub struct HuddleSeedSuggestor {
    intent: IntentPacket,
    plans: Vec<NamedPlan>,
    seeded: Mutex<bool>,
}

/// A plan with a stable identifier for tracking.
pub struct NamedPlan {
    pub id: String,
    pub plan: Plan,
}

impl HuddleSeedSuggestor {
    pub fn new(intent: IntentPacket, plans: Vec<NamedPlan>) -> Self {
        Self {
            intent,
            plans,
            seeded: Mutex::new(false),
        }
    }

    /// Build from an intent and a set of plans with auto-generated IDs
    /// based on each plan's reasoning system.
    pub fn from_plans(intent: IntentPacket, plans: Vec<Plan>) -> Self {
        let named: Vec<NamedPlan> = plans
            .into_iter()
            .enumerate()
            .map(|(i, plan)| {
                let system = match plan.contributor {
                    ReasoningSystem::DomainModel => "domain",
                    ReasoningSystem::CausalAnalysis => "causal",
                    ReasoningSystem::ConstraintSolver => "constraint",
                    ReasoningSystem::CostEstimation => "cost",
                    ReasoningSystem::LlmReasoning => "llm",
                    ReasoningSystem::MlPrediction => "ml",
                };
                NamedPlan {
                    id: format!("huddle-{system}-{i}"),
                    plan,
                }
            })
            .collect();
        Self::new(intent, named)
    }

    /// Access the intent that seeded this suggestor.
    pub fn intent(&self) -> &IntentPacket {
        &self.intent
    }
}

#[async_trait::async_trait]
impl Suggestor for HuddleSeedSuggestor {
    fn name(&self) -> &'static str {
        "organism-huddle-seed"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, _ctx: &dyn Context) -> bool {
        !*self.seeded.lock().unwrap()
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        *self.seeded.lock().unwrap() = true;

        let mut proposals = Vec::new();

        // Seed the intent description
        proposals.push(ProposedFact::new(
            ContextKey::Seeds,
            "intent",
            &self.intent.outcome,
            "organism-huddle-seed",
        ));

        // Each plan becomes strategy facts
        for named in &self.plans {
            let content = named
                .plan
                .steps
                .iter()
                .map(|step| step.action.as_str())
                .collect::<Vec<_>>()
                .join(" | ");

            let strategy_content = format!(
                "[{}] {} -- {}",
                reasoning_system_tag(named.plan.contributor),
                content,
                named.plan.rationale,
            );

            proposals.push(ProposedFact::new(
                ContextKey::Strategies,
                &named.id,
                strategy_content,
                "organism-huddle-seed",
            ));
        }

        AgentEffect::with_proposals(proposals)
    }
}

fn reasoning_system_tag(system: ReasoningSystem) -> &'static str {
    match system {
        ReasoningSystem::DomainModel => "domain-model",
        ReasoningSystem::CausalAnalysis => "causal-analysis",
        ReasoningSystem::ConstraintSolver => "constraint-solver",
        ReasoningSystem::CostEstimation => "cost-estimation",
        ReasoningSystem::LlmReasoning => "llm-reasoning",
        ReasoningSystem::MlPrediction => "ml-prediction",
    }
}

// ── Shared Budget ─────────────────────────────────────────────────

/// Cross-suggestor resource tracking for bounded convergence loops.
///
/// Multiple suggestors share a single budget to enforce global resource
/// limits. Each resource kind (searches, LLM calls, etc.) has an
/// independent counter.
pub struct SharedBudget {
    limits: Vec<ResourceLimit>,
}

struct ResourceLimit {
    name: String,
    max: usize,
    used: Mutex<usize>,
}

impl SharedBudget {
    pub fn new() -> Self {
        Self { limits: Vec::new() }
    }

    /// Add a resource limit. Returns self for chaining.
    pub fn with_limit(mut self, name: impl Into<String>, max: usize) -> Self {
        self.limits.push(ResourceLimit {
            name: name.into(),
            max,
            used: Mutex::new(0),
        });
        self
    }

    /// Try to consume one unit of the named resource.
    /// Returns `true` if the resource was available, `false` if exhausted.
    pub fn try_use(&self, name: &str) -> bool {
        if let Some(limit) = self.limits.iter().find(|l| l.name == name) {
            let mut used = limit.used.lock().unwrap();
            if *used >= limit.max {
                return false;
            }
            *used += 1;
            true
        } else {
            true
        }
    }

    /// Remaining units for the named resource.
    pub fn remaining(&self, name: &str) -> usize {
        self.limits
            .iter()
            .find(|l| l.name == name)
            .map_or(usize::MAX, |l| {
                l.max.saturating_sub(*l.used.lock().unwrap())
            })
    }

    /// Total used for the named resource.
    pub fn used(&self, name: &str) -> usize {
        self.limits
            .iter()
            .find(|l| l.name == name)
            .map_or(0, |l| *l.used.lock().unwrap())
    }
}

// ── Gap Detector Suggestor ────────────────────────────────────────

/// Detects gaps in accumulated hypotheses and proposes new strategies.
///
/// This is the debate-as-suggestor pattern discovered in Monterro's
/// convergent due diligence: instead of a standalone debate loop, gap
/// detection participates in the Converge cycle. When new hypotheses
/// arrive, it analyzes them and proposes follow-up strategies that
/// trigger further research.
///
/// The `analyze` closure receives the current hypotheses and returns
/// proposed strategy facts (id, content pairs).
pub struct GapDetectorSuggestor<F> {
    name: String,
    budget: Arc<SharedBudget>,
    resource_name: String,
    analyze: F,
    last_hypothesis_count: Mutex<usize>,
    generation_count: Mutex<usize>,
    max_generations: usize,
    min_hypotheses: usize,
}

impl<F> GapDetectorSuggestor<F>
where
    F: Fn(&[converge_pack::Fact]) -> Vec<(String, String)> + Send + Sync,
{
    pub fn new(
        name: impl Into<String>,
        budget: Arc<SharedBudget>,
        resource_name: impl Into<String>,
        analyze: F,
    ) -> Self {
        Self {
            name: name.into(),
            budget,
            resource_name: resource_name.into(),
            analyze,
            last_hypothesis_count: Mutex::new(0),
            generation_count: Mutex::new(0),
            max_generations: 3,
            min_hypotheses: 5,
        }
    }

    pub fn with_max_generations(mut self, max: usize) -> Self {
        self.max_generations = max;
        self
    }

    pub fn with_min_hypotheses(mut self, min: usize) -> Self {
        self.min_hypotheses = min;
        self
    }
}

#[async_trait::async_trait]
impl<F> Suggestor for GapDetectorSuggestor<F>
where
    F: Fn(&[converge_pack::Fact]) -> Vec<(String, String)> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let current = ctx.count(ContextKey::Hypotheses);
        let last = *self.last_hypothesis_count.lock().unwrap();
        let gens = *self.generation_count.lock().unwrap();

        current >= self.min_hypotheses
            && current > last
            && gens < self.max_generations
            && self.budget.remaining(&self.resource_name) > 0
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if !self.budget.try_use(&self.resource_name) {
            return AgentEffect::empty();
        }

        let hypotheses = ctx.get(ContextKey::Hypotheses);
        *self.last_hypothesis_count.lock().unwrap() = hypotheses.len();
        *self.generation_count.lock().unwrap() += 1;

        let new_strategies = (self.analyze)(hypotheses);

        let proposals: Vec<ProposedFact> = new_strategies
            .into_iter()
            .map(|(id, content)| {
                ProposedFact::new(ContextKey::Strategies, &id, content, &self.name)
            })
            .collect();

        AgentEffect::with_proposals(proposals)
    }
}

// ── Stability Suggestor ───────────────────────────────────────────

/// Fires when a context key has been stable for N cycles.
///
/// This is the synthesis pattern: wait for hypotheses to stabilize,
/// then produce a final output. The `synthesize` closure receives
/// all facts in the watched key and produces proposals.
pub struct StabilitySuggestor<F> {
    name: String,
    watch_key: ContextKey,
    output_key: ContextKey,
    budget: Arc<SharedBudget>,
    resource_name: String,
    synthesize: F,
    last_count: Mutex<usize>,
    stable_cycles: Mutex<usize>,
    required_stable_cycles: usize,
}

impl<F> StabilitySuggestor<F>
where
    F: Fn(&[converge_pack::Fact]) -> Vec<(String, String, f64)> + Send + Sync,
{
    pub fn new(
        name: impl Into<String>,
        watch_key: ContextKey,
        output_key: ContextKey,
        budget: Arc<SharedBudget>,
        resource_name: impl Into<String>,
        synthesize: F,
    ) -> Self {
        Self {
            name: name.into(),
            watch_key,
            output_key,
            budget,
            resource_name: resource_name.into(),
            synthesize,
            last_count: Mutex::new(0),
            stable_cycles: Mutex::new(0),
            required_stable_cycles: 2,
        }
    }

    pub fn with_required_stable_cycles(mut self, n: usize) -> Self {
        self.required_stable_cycles = n;
        self
    }
}

#[async_trait::async_trait]
impl<F> Suggestor for StabilitySuggestor<F>
where
    F: Fn(&[converge_pack::Fact]) -> Vec<(String, String, f64)> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        // Can't return a reference to a local, so use a static slice
        // for the common case. Callers must ensure watch_key is Hypotheses
        // for this to be meaningful.
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let current = ctx.count(self.watch_key);
        let mut last = self.last_count.lock().unwrap();
        let mut stable = self.stable_cycles.lock().unwrap();

        if current == *last && current > 0 {
            *stable += 1;
        } else {
            *stable = 0;
            *last = current;
        }

        *stable >= self.required_stable_cycles
            && !ctx.has(self.output_key)
            && self.budget.remaining(&self.resource_name) > 0
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if !self.budget.try_use(&self.resource_name) {
            return AgentEffect::empty();
        }

        let facts = ctx.get(self.watch_key);
        let results = (self.synthesize)(facts);

        let proposals: Vec<ProposedFact> = results
            .into_iter()
            .map(|(id, content, confidence)| {
                ProposedFact::new(self.output_key, &id, content, &self.name)
                    .with_confidence(confidence)
            })
            .collect();

        AgentEffect::with_proposals(proposals)
    }
}

// ── Hypothesis Tracker Suggestor ─────────────────────────────────

/// Observes hypothesis and evaluation facts across convergence cycles,
/// recording the lifecycle of each hypothesis (formed → confirmed/falsified).
///
/// Does not emit facts — returns `AgentEffect::empty()`. The resolved
/// hypotheses are read by the call site post-run via [`Self::resolved`]
/// and emitted to the ExperienceStore as `HypothesisResolved` events.
pub struct HypothesisTrackerSuggestor {
    domain: String,
    confidence_threshold: f64,
    hypotheses: Arc<Mutex<Vec<crate::TrackedHypothesis>>>,
    last_hypothesis_count: Mutex<usize>,
    last_evaluation_count: Mutex<usize>,
    current_cycle: Mutex<u32>,
}

impl HypothesisTrackerSuggestor {
    pub fn new(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            confidence_threshold: 0.7,
            hypotheses: Arc::new(Mutex::new(Vec::new())),
            last_hypothesis_count: Mutex::new(0),
            last_evaluation_count: Mutex::new(0),
            current_cycle: Mutex::new(0),
        }
    }

    #[must_use]
    pub fn with_confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    pub fn resolved(&self) -> Vec<crate::TrackedHypothesis> {
        self.hypotheses
            .lock()
            .unwrap()
            .iter()
            .filter(|h| !matches!(h.outcome, crate::HypothesisOutcome::Open))
            .cloned()
            .collect()
    }

    pub fn roster(&self) -> Arc<Mutex<Vec<crate::TrackedHypothesis>>> {
        Arc::clone(&self.hypotheses)
    }
}

#[async_trait::async_trait]
impl Suggestor for HypothesisTrackerSuggestor {
    fn name(&self) -> &'static str {
        "organism-hypothesis-tracker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let h_count = ctx.count(ContextKey::Hypotheses);
        let e_count = ctx.count(ContextKey::Evaluations);
        let last_h = *self.last_hypothesis_count.lock().unwrap();
        let last_e = *self.last_evaluation_count.lock().unwrap();

        h_count > last_h || e_count > last_e || ctx.has(ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let hypothesis_facts = ctx.get(ContextKey::Hypotheses);
        let evaluation_facts = ctx.get(ContextKey::Evaluations);
        let has_proposals = ctx.has(ContextKey::Proposals);

        *self.last_hypothesis_count.lock().unwrap() = hypothesis_facts.len();
        *self.last_evaluation_count.lock().unwrap() = evaluation_facts.len();

        let cycle = {
            let mut c = self.current_cycle.lock().unwrap();
            *c += 1;
            *c
        };

        // Collect contradiction fact IDs from evaluations for falsification matching.
        // Convention: evaluation facts referencing a hypothesis use the hypothesis
        // fact ID as a substring in their content (same pattern as DD's
        // ContradictionFinderSuggestor).
        let contradiction_targets: Vec<(String, String)> = evaluation_facts
            .iter()
            .map(|f| (f.id.clone(), f.content.clone()))
            .collect();

        let mut roster = self.hypotheses.lock().unwrap();

        // Register new hypotheses
        let known_ids: std::collections::HashSet<String> =
            roster.iter().map(|h| h.fact_id.clone()).collect();

        for fact in hypothesis_facts {
            if known_ids.contains(&fact.id) {
                continue;
            }

            let confidence: f64 = fact
                .content
                .parse()
                .ok()
                .or_else(|| {
                    serde_json::from_str::<serde_json::Value>(&fact.content)
                        .ok()
                        .and_then(|v| v.get("confidence")?.as_f64())
                })
                .unwrap_or(0.5);

            roster.push(crate::TrackedHypothesis {
                fact_id: fact.id.clone(),
                domain: self.domain.clone(),
                claim: fact.content.clone(),
                confidence,
                formed_cycle: cycle,
                resolved_cycle: None,
                outcome: crate::HypothesisOutcome::Open,
            });
        }

        // Check for falsification via contradictions
        for h in roster.iter_mut() {
            if !matches!(h.outcome, crate::HypothesisOutcome::Open) {
                continue;
            }

            for (eval_id, eval_content) in &contradiction_targets {
                if eval_content.contains(&h.fact_id) {
                    h.outcome = crate::HypothesisOutcome::Falsified {
                        contradiction_id: eval_id.clone(),
                    };
                    h.resolved_cycle = Some(cycle);
                    break;
                }
            }
        }

        // On synthesis (proposals present), finalize remaining open hypotheses
        if has_proposals {
            for h in roster.iter_mut() {
                if !matches!(h.outcome, crate::HypothesisOutcome::Open) {
                    continue;
                }
                if h.confidence >= self.confidence_threshold {
                    h.outcome = crate::HypothesisOutcome::Confirmed;
                } else {
                    h.outcome = crate::HypothesisOutcome::Unresolved;
                }
                h.resolved_cycle = Some(cycle);
            }
        }

        AgentEffect::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_budget_tracks_resources() {
        let budget = SharedBudget::new()
            .with_limit("searches", 3)
            .with_limit("llm", 2);

        assert_eq!(budget.remaining("searches"), 3);
        assert!(budget.try_use("searches"));
        assert!(budget.try_use("searches"));
        assert!(budget.try_use("searches"));
        assert!(!budget.try_use("searches"));
        assert_eq!(budget.remaining("searches"), 0);
        assert_eq!(budget.used("searches"), 3);

        assert_eq!(budget.remaining("llm"), 2);
        assert!(budget.try_use("llm"));
        assert_eq!(budget.remaining("llm"), 1);
    }

    #[test]
    fn unknown_resource_is_unlimited() {
        let budget = SharedBudget::new();
        assert!(budget.try_use("anything"));
        assert_eq!(budget.remaining("anything"), usize::MAX);
    }

    #[test]
    fn huddle_seed_fires_once() {
        use chrono::{Duration, Utc};

        let intent = IntentPacket::new("test intent", Utc::now() + Duration::hours(1));
        let mut plan = Plan::new(&intent, "test rationale");
        plan.contributor = ReasoningSystem::DomainModel;
        plan.steps = vec![crate::PlanStep {
            action: "search for things".into(),
            expected_effect: "find things".into(),
        }];

        let suggestor = HuddleSeedSuggestor::from_plans(intent, vec![plan]);

        // First call: accepts
        assert_eq!(suggestor.name(), "organism-huddle-seed");
        assert!(suggestor.dependencies().is_empty());
    }
}
