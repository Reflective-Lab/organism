//! Due Diligence suggestor implementations.
//!
//! Generic, reusable suggestors for the DD convergence loop. Apps inject
//! their search and LLM backends via the [`DdSearch`] and [`DdLlm`] traits.
//! Organism owns the prompts, parsing, and convergence patterns.
//!
//! # Layer responsibilities
//!
//! | Layer | Owns |
//! |-------|------|
//! | **Organism** | DD suggestors, prompts, fact parsing, convergence patterns |
//! | **App** | `DdSearch` + `DdLlm` implementations |
//! | **Converge** | Engine, context, axioms, promotion gates |

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex};

use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::suggestor::SharedBudget;

// ── Error types ───────────────────────────────────────────────────

/// Typed errors from DD backends. Apps classify raw provider errors
/// into these variants. Organism uses the variant to decide whether
/// to retry, abort, or record a constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DdError {
    /// Account/billing problem — stop the entire run.
    CreditsExhausted { provider: String, detail: String },
    /// Throttled — could retry after backoff, but suggestor won't.
    RateLimited { provider: String, retry_after_ms: Option<u64> },
    /// Provider is down or unreachable.
    ProviderUnavailable { provider: String, detail: String },
    /// Provider returned something we couldn't use.
    BadResponse { provider: String, detail: String },
    /// The input was too large for the provider.
    PromptTooLarge { provider: String, tokens: Option<usize> },
    /// JSON parsing failed on provider output.
    ParseFailed { provider: String, detail: String },
}

impl fmt::Display for DdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreditsExhausted { provider, detail } =>
                write!(f, "[{provider}] credits exhausted: {detail}"),
            Self::RateLimited { provider, .. } =>
                write!(f, "[{provider}] rate limited"),
            Self::ProviderUnavailable { provider, detail } =>
                write!(f, "[{provider}] unavailable: {detail}"),
            Self::BadResponse { provider, detail } =>
                write!(f, "[{provider}] bad response: {detail}"),
            Self::PromptTooLarge { provider, tokens } =>
                write!(f, "[{provider}] prompt too large ({})", tokens.map_or("unknown".into(), |t| format!("{t} tokens"))),
            Self::ParseFailed { provider, detail } =>
                write!(f, "[{provider}] parse failed: {detail}"),
        }
    }
}

impl DdError {
    /// Is this an infrastructure failure that should NOT calibrate learning priors?
    pub fn is_infra_failure(&self) -> bool {
        matches!(
            self,
            Self::CreditsExhausted { .. }
                | Self::RateLimited { .. }
                | Self::ProviderUnavailable { .. }
        )
    }

    /// Should the entire DD run abort on this error?
    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::CreditsExhausted { .. })
    }

    /// The constraint fact ID for this error kind.
    fn constraint_id(&self, suggestor: &str) -> String {
        let kind = match self {
            Self::CreditsExhausted { .. } => "credits-exhausted",
            Self::RateLimited { .. } => "rate-limited",
            Self::ProviderUnavailable { .. } => "provider-unavailable",
            Self::BadResponse { .. } => "bad-response",
            Self::PromptTooLarge { .. } => "prompt-too-large",
            Self::ParseFailed { .. } => "parse-failed",
        };
        format!("dd:constraint:{suggestor}:{kind}")
    }
}

/// Build a constraint fact from a DD error so it's visible in context.
fn error_to_constraint(error: &DdError, suggestor: &str) -> ProposedFact {
    let id = error.constraint_id(suggestor);
    let content = serde_json::json!({
        "type": "error",
        "error": serde_json::to_value(error).unwrap_or_default(),
        "is_infra_failure": error.is_infra_failure(),
        "is_fatal": error.is_fatal(),
        "message": error.to_string(),
    })
    .to_string();
    ProposedFact::new(ContextKey::Constraints, &id, content, suggestor)
        .with_confidence(1.0)
}

// ── Backend traits ───────────────────────────────────────────────

/// Async search backend. Apps implement this by wrapping their
/// search providers (Brave, Tavily, etc.).
///
/// Apps are responsible for classifying raw HTTP/provider errors into
/// [`DdError`] variants so organism can make informed decisions.
#[async_trait::async_trait]
pub trait DdSearch: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, DdError>;
}

/// Async LLM backend. Apps implement this by wrapping their
/// LLM providers (Anthropic, OpenAI, etc.).
///
/// Apps are responsible for classifying raw HTTP/provider errors into
/// [`DdError`] variants so organism can make informed decisions.
#[async_trait::async_trait]
pub trait DdLlm: Send + Sync {
    async fn complete(&self, prompt: &str) -> Result<String, DdError>;
}

/// A search hit returned by a [`DdSearch`] implementation.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub title: String,
    pub url: String,
    pub content: String,
    pub provider: String,
}

// ── Failover wrappers ─────────────────────────────────────────────

/// Tries LLM backends in order. On retryable errors (credits exhausted,
/// rate limited, provider unavailable), moves to the next backend.
/// On non-retryable errors (parse failed, bad response), returns
/// immediately — a different provider won't fix bad output.
pub struct FailoverDdLlm {
    backends: Vec<Arc<dyn DdLlm>>,
}

impl FailoverDdLlm {
    pub fn new(backends: Vec<Arc<dyn DdLlm>>) -> Self {
        Self { backends }
    }
}

#[async_trait::async_trait]
impl DdLlm for FailoverDdLlm {
    async fn complete(&self, prompt: &str) -> Result<String, DdError> {
        let mut last_error = None;
        for backend in &self.backends {
            match backend.complete(prompt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let should_failover = e.is_infra_failure();
                    eprintln!("[failover] {} — {}", e, if should_failover { "trying next" } else { "not retryable" });
                    if !should_failover {
                        return Err(e);
                    }
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| DdError::ProviderUnavailable {
            provider: "failover".into(),
            detail: "no backends configured".into(),
        }))
    }
}

/// Tries search backends in order with the same failover logic.
pub struct FailoverDdSearch {
    backends: Vec<Arc<dyn DdSearch>>,
}

impl FailoverDdSearch {
    pub fn new(backends: Vec<Arc<dyn DdSearch>>) -> Self {
        Self { backends }
    }
}

#[async_trait::async_trait]
impl DdSearch for FailoverDdSearch {
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, DdError> {
        let mut last_error = None;
        for backend in &self.backends {
            match backend.search(query).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let should_failover = e.is_infra_failure();
                    eprintln!("[failover] {} — {}", e, if should_failover { "trying next" } else { "not retryable" });
                    if !should_failover {
                        return Err(e);
                    }
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| DdError::ProviderUnavailable {
            provider: "failover".into(),
            detail: "no backends configured".into(),
        }))
    }
}

// ── Breadth Research ──────────────────────────────────────────────

/// Reacts to strategies tagged with a breadth marker.
/// Searches wide and emits signal facts.
pub struct BreadthResearchSuggestor {
    subject: String,
    budget: Arc<SharedBudget>,
    search: Arc<dyn DdSearch>,
    tag: String,
    processed: Mutex<HashSet<String>>,
}

impl BreadthResearchSuggestor {
    pub fn new(
        subject: impl Into<String>,
        budget: Arc<SharedBudget>,
        search: Arc<dyn DdSearch>,
    ) -> Self {
        Self {
            subject: subject.into(),
            budget,
            search,
            tag: "breadth".into(),
            processed: Mutex::new(HashSet::new()),
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    fn unprocessed_strategies(&self, ctx: &dyn Context) -> Vec<String> {
        let processed = self.processed.lock().unwrap();
        ctx.get(ContextKey::Strategies)
            .iter()
            .filter(|f| f.content.contains(&self.tag))
            .filter(|f| !processed.contains(&f.id))
            .map(|f| f.content.clone())
            .collect()
    }
}

#[async_trait::async_trait]
impl Suggestor for BreadthResearchSuggestor {
    fn name(&self) -> &str {
        "dd-breadth-research"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        self.budget.remaining("searches") > 0 && !self.unprocessed_strategies(ctx).is_empty()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = self.unprocessed_strategies(ctx);
        let mut proposals = Vec::new();

        for strategy in strategies {
            if !self.budget.try_use("searches") {
                break;
            }

            let query = format!("{} {strategy}", self.subject);
            match self.search.search(&query).await {
                Ok(hits) => {
                    for hit in &hits {
                        if !is_relevant(&hit.title, &hit.content, &hit.url, &self.subject) {
                            continue;
                        }
                        let id = format!("signal-breadth-{}", Uuid::new_v4());
                        let content = serde_json::json!({
                            "title": hit.title,
                            "url": hit.url,
                            "content": hit.content,
                            "provider": hit.provider,
                            "query": query,
                        })
                        .to_string();
                        proposals.push(
                            ProposedFact::new(ContextKey::Signals, &id, content, "dd-breadth-research")
                                .with_confidence(1.0),
                        );
                    }
                }
                Err(e) => {
                    proposals.push(error_to_constraint(&e, "dd-breadth-research"));
                    if e.is_fatal() { break; }
                }
            }

            self.processed.lock().unwrap().insert(
                ctx.get(ContextKey::Strategies)
                    .iter()
                    .find(|f| f.content == strategy)
                    .map(|f| f.id.clone())
                    .unwrap_or_default(),
            );
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Depth Research ────────────────────────────────────────────────

/// Reacts to strategies tagged with a depth marker.
/// Searches deep and emits signal facts.
pub struct DepthResearchSuggestor {
    subject: String,
    budget: Arc<SharedBudget>,
    search: Arc<dyn DdSearch>,
    tag: String,
    processed: Mutex<HashSet<String>>,
}

impl DepthResearchSuggestor {
    pub fn new(
        subject: impl Into<String>,
        budget: Arc<SharedBudget>,
        search: Arc<dyn DdSearch>,
    ) -> Self {
        Self {
            subject: subject.into(),
            budget,
            search,
            tag: "depth".into(),
            processed: Mutex::new(HashSet::new()),
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    fn unprocessed_strategies(&self, ctx: &dyn Context) -> Vec<String> {
        let processed = self.processed.lock().unwrap();
        ctx.get(ContextKey::Strategies)
            .iter()
            .filter(|f| f.content.contains(&self.tag))
            .filter(|f| !processed.contains(&f.id))
            .map(|f| f.content.clone())
            .collect()
    }
}

#[async_trait::async_trait]
impl Suggestor for DepthResearchSuggestor {
    fn name(&self) -> &str {
        "dd-depth-research"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        self.budget.remaining("searches") > 0 && !self.unprocessed_strategies(ctx).is_empty()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = self.unprocessed_strategies(ctx);
        let mut proposals = Vec::new();

        for strategy in strategies {
            if !self.budget.try_use("searches") {
                break;
            }

            let query = format!("{} {strategy}", self.subject);
            match self.search.search(&query).await {
                Ok(hits) => {
                    for hit in &hits {
                        if !is_relevant(&hit.title, &hit.content, &hit.url, &self.subject) {
                            continue;
                        }
                        let id = format!("signal-depth-{}", Uuid::new_v4());
                        let content = serde_json::json!({
                            "title": hit.title,
                            "url": hit.url,
                            "content": hit.content,
                            "provider": hit.provider,
                            "query": query,
                        })
                        .to_string();
                        proposals.push(
                            ProposedFact::new(ContextKey::Signals, &id, content, "dd-depth-research")
                                .with_confidence(1.0),
                        );
                    }
                }
                Err(e) => {
                    proposals.push(error_to_constraint(&e, "dd-depth-research"));
                    if e.is_fatal() { break; }
                }
            }

            self.processed.lock().unwrap().insert(
                ctx.get(ContextKey::Strategies)
                    .iter()
                    .find(|f| f.content == strategy)
                    .map(|f| f.id.clone())
                    .unwrap_or_default(),
            );
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Fact Extractor ────────────────────────────────────────────────

/// Reads raw signals and extracts tagged factual claims via LLM.
/// Organism owns the extraction prompt and parsing.
pub struct FactExtractorSuggestor {
    subject: String,
    budget: Arc<SharedBudget>,
    llm: Arc<dyn DdLlm>,
    last_signal_count: Mutex<usize>,
}

impl FactExtractorSuggestor {
    pub fn new(
        subject: impl Into<String>,
        budget: Arc<SharedBudget>,
        llm: Arc<dyn DdLlm>,
    ) -> Self {
        Self {
            subject: subject.into(),
            budget,
            llm,
            last_signal_count: Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for FactExtractorSuggestor {
    fn name(&self) -> &str {
        "dd-fact-extractor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let current = ctx.count(ContextKey::Signals);
        let last = *self.last_signal_count.lock().unwrap();
        self.budget.remaining("llm") > 0 && current > last
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if !self.budget.try_use("llm") {
            return AgentEffect::empty();
        }

        let all_signals = ctx.get(ContextKey::Signals);
        *self.last_signal_count.lock().unwrap() = all_signals.len();

        // Cap signals to avoid oversized prompts that cause LLM truncation
        let signals: Vec<_> = all_signals.iter().take(15).cloned().collect();
        let prompt = prompts::fact_extraction(&self.subject, &signals);

        let mut proposals = Vec::new();
        match self.llm.complete(&prompt).await {
            Ok(raw) => {
                let cleaned = strip_fences(&raw);
                match serde_json::from_str::<Vec<serde_json::Value>>(cleaned) {
                    Ok(facts) => {
                        for (i, fact) in facts.iter().enumerate() {
                            let id = format!("hypothesis-{}-{i}", Uuid::new_v4());
                            proposals.push(
                                ProposedFact::new(
                                    ContextKey::Hypotheses,
                                    &id,
                                    fact.to_string(),
                                    "dd-fact-extractor",
                                )
                                .with_confidence(fact["confidence"].as_f64().unwrap_or(0.5)),
                            );
                        }
                    }
                    Err(e) => {
                        let parse_err = DdError::ParseFailed {
                            provider: "llm".into(),
                            detail: format!("{e} (first 200 chars: {})", &cleaned[..cleaned.len().min(200)]),
                        };
                        proposals.push(error_to_constraint(&parse_err, "dd-fact-extractor"));
                    }
                }
            }
            Err(e) => {
                proposals.push(error_to_constraint(&e, "dd-fact-extractor"));
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Gap Detector ──────────────────────────────────────────────────

/// Reviews hypotheses, identifies critical gaps, proposes follow-up strategies.
/// Organism owns the gap-detection prompt and strategy parsing.
pub struct GapDetectorSuggestor {
    subject: String,
    budget: Arc<SharedBudget>,
    llm: Arc<dyn DdLlm>,
    last_hypothesis_count: Mutex<usize>,
    generation_count: Mutex<usize>,
    max_generations: usize,
    min_hypotheses: usize,
}

impl GapDetectorSuggestor {
    pub fn new(
        subject: impl Into<String>,
        budget: Arc<SharedBudget>,
        llm: Arc<dyn DdLlm>,
    ) -> Self {
        Self {
            subject: subject.into(),
            budget,
            llm,
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
impl Suggestor for GapDetectorSuggestor {
    fn name(&self) -> &str {
        "dd-gap-detector"
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
            && self.budget.remaining("llm") > 0
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if !self.budget.try_use("llm") {
            return AgentEffect::empty();
        }

        let hypotheses = ctx.get(ContextKey::Hypotheses);
        *self.last_hypothesis_count.lock().unwrap() = hypotheses.len();
        let generation = {
            let mut g = self.generation_count.lock().unwrap();
            *g += 1;
            *g
        };

        let prompt = prompts::gap_detection(&self.subject, &hypotheses, generation, self.max_generations);
        let mut proposals = Vec::new();

        match self.llm.complete(&prompt).await {
            Ok(raw) => {
                let cleaned = strip_fences(&raw);
                if let Ok(strategies) = serde_json::from_str::<Vec<serde_json::Value>>(cleaned) {
                    for (i, s) in strategies.iter().enumerate() {
                        let mode = s["mode"].as_str().unwrap_or("breadth");
                        let query = s["query"].as_str().unwrap_or("");
                        let reason = s["reason"].as_str().unwrap_or("");
                        let content = format!("[{mode}] {query} -- {reason}");
                        let id = format!("strategy-gap-{i}-{}", Uuid::new_v4());

                        proposals.push(ProposedFact::new(
                            ContextKey::Strategies,
                            &id,
                            content,
                            "dd-gap-detector",
                        ));
                    }
                }
            }
            Err(e) => {
                proposals.push(error_to_constraint(&e, "dd-gap-detector"));
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Contradiction Finder ──────────────────────────────────────────

/// Detects conflicting claims across hypotheses on the same topic.
/// Pure data analysis — no LLM needed.
pub struct ContradictionFinderSuggestor {
    last_hypothesis_count: Mutex<usize>,
}

impl ContradictionFinderSuggestor {
    pub fn new() -> Self {
        Self {
            last_hypothesis_count: Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for ContradictionFinderSuggestor {
    fn name(&self) -> &str {
        "dd-contradiction-finder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let current = ctx.count(ContextKey::Hypotheses);
        let last = *self.last_hypothesis_count.lock().unwrap();
        current > last && current >= 3
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let hypotheses = ctx.get(ContextKey::Hypotheses);
        *self.last_hypothesis_count.lock().unwrap() = hypotheses.len();

        // Group hypotheses by topic (from JSON "category" field)
        let mut by_category: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for fact in hypotheses {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                let category = v["category"].as_str().unwrap_or("unknown").to_string();
                let claim = v["claim"].as_str().unwrap_or("").to_string();
                if !claim.is_empty() {
                    by_category
                        .entry(category)
                        .or_default()
                        .push((fact.id.clone(), claim));
                }
            }
        }

        let mut proposals = Vec::new();
        let existing_evaluations: HashSet<String> = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .map(|f| f.id.clone())
            .collect();

        // Flag categories where claims contain contradictory signals
        for (category, claims) in &by_category {
            if claims.len() < 2 {
                continue;
            }

            // Look for numeric disagreements or explicit contradiction markers
            let has_contradiction = claims.iter().any(|(_, c)| {
                c.to_lowercase().contains("contradiction")
                    || c.to_lowercase().contains("disagree")
                    || c.to_lowercase().contains("conflict")
            });

            if has_contradiction {
                let id = format!("contradiction-{category}-{}", Uuid::new_v4());
                if existing_evaluations.contains(&id) {
                    continue;
                }

                let claim_ids: Vec<&str> = claims.iter().map(|(id, _)| id.as_str()).collect();
                let content = serde_json::json!({
                    "category": category,
                    "type": "contradiction",
                    "claim_count": claims.len(),
                    "claim_ids": claim_ids,
                    "description": format!("Contradictory claims detected in {category} — sources disagree"),
                    "needs_human_review": true,
                })
                .to_string();

                proposals.push(
                    ProposedFact::new(ContextKey::Evaluations, &id, content, "dd-contradiction-finder")
                        .with_confidence(0.9),
                );
            }
        }

        AgentEffect::with_proposals(proposals)
    }
}

// ── Synthesis ─────────────────────────────────────────────────────

/// Produces final analysis when hypotheses stabilize.
/// Organism owns the synthesis prompt.
pub struct SynthesisSuggestor {
    subject: String,
    budget: Arc<SharedBudget>,
    llm: Arc<dyn DdLlm>,
    last_hypothesis_count: Mutex<usize>,
    stable_cycles: Mutex<usize>,
    required_stable_cycles: usize,
}

impl SynthesisSuggestor {
    pub fn new(
        subject: impl Into<String>,
        budget: Arc<SharedBudget>,
        llm: Arc<dyn DdLlm>,
    ) -> Self {
        Self {
            subject: subject.into(),
            budget,
            llm,
            last_hypothesis_count: Mutex::new(0),
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
impl Suggestor for SynthesisSuggestor {
    fn name(&self) -> &str {
        "dd-synthesis"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let current = ctx.count(ContextKey::Hypotheses);
        let mut last = self.last_hypothesis_count.lock().unwrap();
        let mut stable = self.stable_cycles.lock().unwrap();

        if current == *last && current > 0 {
            *stable += 1;
        } else {
            *stable = 0;
            *last = current;
        }

        *stable >= self.required_stable_cycles
            && !ctx.has(ContextKey::Proposals)
            && self.budget.remaining("llm") > 0
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        if !self.budget.try_use("llm") {
            return AgentEffect::empty();
        }

        let hypotheses = ctx.get(ContextKey::Hypotheses);
        let prompt = prompts::synthesis(&self.subject, &hypotheses);

        match self.llm.complete(&prompt).await {
            Ok(raw) => {
                let id = format!("synthesis-{}", Uuid::new_v4());
                AgentEffect::with_proposal(
                    ProposedFact::new(ContextKey::Proposals, &id, raw, "dd-synthesis")
                        .with_confidence(0.8),
                )
            }
            Err(e) => AgentEffect::with_proposal(error_to_constraint(&e, "dd-synthesis")),
        }
    }
}

// ── Prompts (organism-owned DD intelligence) ──────────────────────

pub mod prompts {
    use converge_pack::Fact;

    pub fn fact_extraction(subject: &str, signals: &[Fact]) -> String {
        let sources_text: String = signals
            .iter()
            .enumerate()
            .filter_map(|(i, f)| {
                let v: serde_json::Value = serde_json::from_str(&f.content).ok()?;
                Some(format!(
                    "[Source {i}] ({}) {}\n  URL: {}\n  {}",
                    v["provider"].as_str().unwrap_or("?"),
                    v["title"].as_str().unwrap_or(""),
                    v["url"].as_str().unwrap_or(""),
                    v["content"].as_str().unwrap_or("")
                ))
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"You are an analyst extracting facts about {subject} from research sources.

{sources_text}

Extract key facts as JSON array. ONLY valid JSON, no fences:
[
  {{
    "claim": "specific factual claim",
    "category": "product|customers|technology|competition|market|financials|team|risk|governance",
    "source_indices": [0, 3],
    "confidence": 0.9
  }}
]

Rules:
- Every fact MUST cite source_indices
- 0.9+ for primary sources, 0.7 for secondary, 0.5 for inferred
- Flag contradictions between sources as separate facts with category "risk""#
        )
    }

    pub fn gap_detection(
        subject: &str,
        hypotheses: &[Fact],
        generation: usize,
        max_generations: usize,
    ) -> String {
        let facts_text: String = hypotheses
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are a PE analyst reviewing extracted facts about {subject}.

Current facts:
{facts_text}

What critical gaps remain? Focus on:
- Missing financials (ARR, growth, margins)
- Unknown ownership/investors
- Unclear competitive positioning
- Missing tech stack details
- Unknown customer concentration

Return JSON array of search strategies:
[
  {{"query": "search terms", "mode": "breadth|depth", "reason": "why this matters"}}
]

This is research pass {generation} of {max_generations}. Only propose searches for gaps that are CRITICAL for investment decision-making.
Pass 1: broad gaps (max 4). Pass 2+: only truly unresolved items (max 2).
ONLY valid JSON, no markdown fences."#
        )
    }

    pub fn synthesis(subject: &str, hypotheses: &[Fact]) -> String {
        let facts_text: String = hypotheses
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"You are a senior PE analyst producing a final due diligence synthesis for {subject}.

All extracted facts:
{facts_text}

Produce a final analysis as JSON:
{{
  "summary": "2-3 paragraph executive summary",
  "market_analysis": "market analysis",
  "competitive_landscape": "competitive analysis",
  "technology_assessment": "tech assessment",
  "risk_factors": ["risk 1", "risk 2"],
  "growth_opportunities": ["opp 1", "opp 2"],
  "recommendation": "investment recommendation"
}}

ONLY valid JSON, no markdown fences. All values plain strings."#
        )
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn strip_fences(raw: &str) -> &str {
    let s = raw.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s);
    s.strip_suffix("```").unwrap_or(s).trim()
}

fn is_relevant(title: &str, content: &str, url: &str, subject: &str) -> bool {
    let s = subject.to_lowercase();
    let t = title.to_lowercase();
    let b = content.to_lowercase();
    let u = url.to_lowercase();
    t.contains(&s)
        || b.contains(&s)
        || u.contains(&s.replace(' ', ""))
        || u.contains(&s.replace(' ', "-"))
}
