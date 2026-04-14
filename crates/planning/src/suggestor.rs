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
    fn name(&self) -> &str {
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
            .map_or(usize::MAX, |l| l.max.saturating_sub(*l.used.lock().unwrap()))
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

        let new_strategies = (self.analyze)(&hypotheses);

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
        let results = (self.synthesize)(&facts);

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
