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

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex};

use converge_pack::{AgentEffect, Context, ContextKey, Fact, ProposedFact, Suggestor};
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
    RateLimited {
        provider: String,
        retry_after_ms: Option<u64>,
    },
    /// Provider is down or unreachable.
    ProviderUnavailable { provider: String, detail: String },
    /// Provider returned something we couldn't use.
    BadResponse { provider: String, detail: String },
    /// The input was too large for the provider.
    PromptTooLarge {
        provider: String,
        tokens: Option<usize>,
    },
    /// JSON parsing failed on provider output.
    ParseFailed { provider: String, detail: String },
}

impl fmt::Display for DdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreditsExhausted { provider, detail } => {
                write!(f, "[{provider}] credits exhausted: {detail}")
            }
            Self::RateLimited { provider, .. } => write!(f, "[{provider}] rate limited"),
            Self::ProviderUnavailable { provider, detail } => {
                write!(f, "[{provider}] unavailable: {detail}")
            }
            Self::BadResponse { provider, detail } => {
                write!(f, "[{provider}] bad response: {detail}")
            }
            Self::PromptTooLarge { provider, tokens } => write!(
                f,
                "[{provider}] prompt too large ({})",
                tokens.map_or("unknown".into(), |t| format!("{t} tokens"))
            ),
            Self::ParseFailed { provider, detail } => {
                write!(f, "[{provider}] parse failed: {detail}")
            }
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
    ProposedFact::new(ContextKey::Constraints, &id, content, suggestor).with_confidence(1.0)
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
                    eprintln!(
                        "[failover] {} — {}",
                        e,
                        if should_failover {
                            "trying next"
                        } else {
                            "not retryable"
                        }
                    );
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
                    eprintln!(
                        "[failover] {} — {}",
                        e,
                        if should_failover {
                            "trying next"
                        } else {
                            "not retryable"
                        }
                    );
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
    fn name(&self) -> &'static str {
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
                            ProposedFact::new(
                                ContextKey::Signals,
                                &id,
                                content,
                                "dd-breadth-research",
                            )
                            .with_confidence(1.0),
                        );
                    }
                }
                Err(e) => {
                    proposals.push(error_to_constraint(&e, "dd-breadth-research"));
                    if e.is_fatal() {
                        break;
                    }
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
    fn name(&self) -> &'static str {
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
                            ProposedFact::new(
                                ContextKey::Signals,
                                &id,
                                content,
                                "dd-depth-research",
                            )
                            .with_confidence(1.0),
                        );
                    }
                }
                Err(e) => {
                    proposals.push(error_to_constraint(&e, "dd-depth-research"));
                    if e.is_fatal() {
                        break;
                    }
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
    processed_signal_count: Mutex<usize>,
}

impl FactExtractorSuggestor {
    pub fn new(subject: impl Into<String>, budget: Arc<SharedBudget>, llm: Arc<dyn DdLlm>) -> Self {
        Self {
            subject: subject.into(),
            budget,
            llm,
            processed_signal_count: Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for FactExtractorSuggestor {
    fn name(&self) -> &'static str {
        "dd-fact-extractor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let current = ctx.count(ContextKey::Signals);
        let processed = *self.processed_signal_count.lock().unwrap();
        self.budget.remaining("llm") > 0 && current > processed
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let all_signals = ctx.get(ContextKey::Signals);
        let (start, end) = next_batch_bounds(
            all_signals.len(),
            *self.processed_signal_count.lock().unwrap(),
            15,
        );
        let signals: Vec<_> = all_signals
            .iter()
            .skip(start)
            .take(end - start)
            .cloned()
            .collect();

        if signals.is_empty() || !self.budget.try_use("llm") {
            return AgentEffect::empty();
        }

        *self.processed_signal_count.lock().unwrap() = end;
        let prompt = prompts::fact_extraction(&self.subject, &signals);
        let mut seen_fact_keys: HashSet<String> = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .filter_map(|fact| existing_fact_signature(&fact.content))
            .collect();

        let mut proposals = Vec::new();
        match self.llm.complete(&prompt).await {
            Ok(raw) => match parse_json_array_response(&raw, "facts") {
                Ok(facts) => {
                    for (i, fact) in facts.iter().enumerate() {
                        let Some(normalized_fact) = normalize_dd_fact(fact) else {
                            continue;
                        };
                        let signature = dd_fact_signature(&normalized_fact);
                        if !seen_fact_keys.insert(signature) {
                            continue;
                        }
                        let id = format!("hypothesis-{}-{i}", Uuid::new_v4());
                        proposals.push(
                            ProposedFact::new(
                                ContextKey::Hypotheses,
                                &id,
                                normalized_fact.to_string(),
                                "dd-fact-extractor",
                            )
                            .with_confidence(normalized_fact["confidence"].as_f64().unwrap_or(0.5)),
                        );
                    }
                }
                Err(detail) => {
                    let parse_err = DdError::ParseFailed {
                        provider: "llm".into(),
                        detail: format!(
                            "{detail} (first 200 chars: {})",
                            &raw[..raw.len().min(200)]
                        ),
                    };
                    proposals.push(error_to_constraint(&parse_err, "dd-fact-extractor"));
                }
            },
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
    pub fn new(subject: impl Into<String>, budget: Arc<SharedBudget>, llm: Arc<dyn DdLlm>) -> Self {
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
    fn name(&self) -> &'static str {
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

        let prompt =
            prompts::gap_detection(&self.subject, hypotheses, generation, self.max_generations);
        let mut proposals = Vec::new();
        let mut seen_strategy_contents: HashSet<String> = ctx
            .get(ContextKey::Strategies)
            .iter()
            .map(|fact| fact.content.clone())
            .collect();

        match self.llm.complete(&prompt).await {
            Ok(raw) => match parse_json_array_response(&raw, "strategies") {
                Ok(strategies) => {
                    for (i, s) in strategies.iter().enumerate() {
                        let mode = s["mode"].as_str().unwrap_or("breadth");
                        let query = s["query"].as_str().unwrap_or("");
                        let reason = s["reason"].as_str().unwrap_or("");
                        let content = format!("[{mode}] {query} -- {reason}");
                        if query.trim().is_empty()
                            || !seen_strategy_contents.insert(content.clone())
                        {
                            continue;
                        }
                        let id = format!("strategy-gap-{i}-{}", Uuid::new_v4());

                        proposals.push(ProposedFact::new(
                            ContextKey::Strategies,
                            &id,
                            content,
                            "dd-gap-detector",
                        ));
                    }
                }
                Err(detail) => {
                    let parse_err = DdError::ParseFailed {
                        provider: "llm".into(),
                        detail: format!(
                            "{detail} (first 200 chars: {})",
                            &raw[..raw.len().min(200)]
                        ),
                    };
                    proposals.push(error_to_constraint(&parse_err, "dd-gap-detector"));
                }
            },
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
    fn name(&self) -> &'static str {
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
                    ProposedFact::new(
                        ContextKey::Evaluations,
                        &id,
                        content,
                        "dd-contradiction-finder",
                    )
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
    pub fn new(subject: impl Into<String>, budget: Arc<SharedBudget>, llm: Arc<dyn DdLlm>) -> Self {
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
    fn name(&self) -> &'static str {
        "dd-synthesis"
    }

    fn dependencies(&self) -> &[ContextKey] {
        // Synthesis is stability-driven, not dirty-key driven.
        // It must stay schedulable even on cycles where hypotheses stop changing.
        &[]
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
        let consolidated = consolidate_dd_hypotheses(hypotheses);
        let prompt = prompts::synthesis(&self.subject, &consolidated);

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DdFactSummary {
    pub category: String,
    pub claim: String,
    pub confidence: f64,
    pub support_count: usize,
    pub evidence_count: usize,
}

#[derive(Debug, Clone)]
struct ConsolidationCandidate {
    summary: DdFactSummary,
    distinctive_tokens: HashSet<String>,
    topic_tokens: HashSet<String>,
    numeric_tokens: Vec<String>,
    approximate: bool,
    priority_score: f64,
}

// ── Prompts (organism-owned DD intelligence) ──────────────────────

pub mod prompts {
    use super::{DdFactSummary, covered_dd_categories, missing_expected_dd_categories};
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
{{
  "facts": [
    {{
      "claim": "specific factual claim",
      "category": "product|customers|technology|competition|market|financials|team|risk|governance",
      "source_indices": [0, 3],
      "confidence": 0.9
    }}
  ]
}}

Rules:
- Return an object with a top-level "facts" array
- Return at most 20 facts, prioritized by investment relevance
- Return DISTINCT facts only. Do not restate the same claim with cosmetic wording changes.
- Aim to cover these DD categories when evidence exists: product, customers, technology, competition, market, financials
- Use category "technology" for platform, architecture, integrations, attack-surface management mechanics, threat-intelligence infrastructure, APIs, or technical moat
- Do NOT label a clearly technical platform claim as "product" just because it mentions a product name
- Every fact MUST cite source_indices
- 0.9+ for primary sources, 0.7 for secondary, 0.5 for inferred
- Flag contradictions between sources as separate facts with category "risk"
- If no reliable facts can be extracted, return {{"facts":[]}}"#
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
        let covered_categories = covered_dd_categories(hypotheses);
        let missing_categories = missing_expected_dd_categories(hypotheses);
        let covered_text = if covered_categories.is_empty() {
            "none yet".to_string()
        } else {
            covered_categories.join(", ")
        };
        let missing_text = if missing_categories.is_empty() {
            "none".to_string()
        } else {
            missing_categories.join(", ")
        };

        format!(
            r#"You are a PE analyst reviewing extracted facts about {subject}.

Current facts:
{facts_text}

Covered categories:
{covered_text}

Missing expected categories:
{missing_text}

What critical gaps remain? Focus on:
- Missing financials (ARR, growth, margins)
- Unknown ownership/investors
- Unclear competitive positioning
- Missing tech stack details
- Unknown customer concentration

Return JSON object:
{{
  "strategies": [
    {{"query": "search terms", "mode": "breadth|depth", "reason": "why this matters"}}
  ]
}}

This is research pass {generation} of {max_generations}. Only propose searches for gaps that are CRITICAL for investment decision-making.
Pass 1: broad gaps (max 4). Pass 2+: only truly unresolved items (max 2).
ONLY valid JSON, no markdown fences. If no critical gaps remain, return {{"strategies":[]}}."#
        )
    }

    pub fn synthesis(subject: &str, hypotheses: &[DdFactSummary]) -> String {
        let facts_text = if hypotheses.is_empty() {
            "No consolidated facts were available.".to_string()
        } else {
            hypotheses
                .iter()
                .map(|fact| {
                    format!(
                        "- [{} | confidence {:.2} | support {} | evidence {}] {}",
                        fact.category,
                        fact.confidence,
                        fact.support_count,
                        fact.evidence_count,
                        fact.claim
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            r#"You are a senior PE analyst producing a final due diligence synthesis for {subject}.

Consolidated facts:
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

pub fn consolidate_dd_hypotheses(hypotheses: &[Fact]) -> Vec<DdFactSummary> {
    consolidate_dd_fact_values(
        hypotheses
            .iter()
            .filter_map(|fact| serde_json::from_str::<serde_json::Value>(&fact.content).ok())
            .collect::<Vec<_>>(),
    )
}

// ── Hook Extraction ──────────────────────────────────────────────

/// Entities extracted from DD facts for knowledge graph connections.
#[derive(Debug, Clone, Default)]
pub struct DdHooks {
    pub investors: Vec<String>,
    pub business_areas: Vec<String>,
    pub regions: Vec<String>,
    pub competitors: Vec<String>,
}

/// Configurable patterns for hook extraction.
#[derive(Debug, Clone)]
pub struct HookPatterns {
    pub business_areas: Vec<(String, String)>,
    pub regions: Vec<(String, String)>,
    pub entity_triggers: Vec<String>,
}

impl Default for HookPatterns {
    fn default() -> Self {
        Self {
            business_areas: vec![
                ("saas".into(), "SaaS".into()),
                (" grc".into(), "Governance, Risk & Compliance (GRC)".into()),
                (",grc".into(), "Governance, Risk & Compliance (GRC)".into()),
                (
                    "governance, risk".into(),
                    "Governance, Risk & Compliance (GRC)".into(),
                ),
                (" esg".into(), "ESG Reporting".into()),
                ("sustainability".into(), "ESG Reporting".into()),
                ("compliance".into(), "Compliance Management".into()),
                ("strategic planning".into(), "Strategic Planning".into()),
                ("quality management".into(), "Quality Management".into()),
                ("risk management".into(), "Risk Management".into()),
                ("edc".into(), "Electronic Data Capture (EDC)".into()),
                ("clinical trial".into(), "Clinical Trial Management".into()),
                ("eclinical".into(), "eClinical Solutions".into()),
                ("cybersecurity".into(), "Cybersecurity".into()),
                ("vulnerability".into(), "Vulnerability Management".into()),
                ("threat intelligence".into(), "Threat Intelligence".into()),
                ("penetration testing".into(), "Penetration Testing".into()),
                ("attack surface".into(), "Attack Surface Management".into()),
                ("workforce management".into(), "Workforce Management".into()),
                (
                    "scheduling".into(),
                    "Scheduling & Resource Management".into(),
                ),
                ("timetabling".into(), "Timetabling".into()),
                (
                    "higher education".into(),
                    "Higher Education Software".into(),
                ),
                ("edtech".into(), "EdTech".into()),
                (
                    "business intelligence".into(),
                    "Business Intelligence".into(),
                ),
                ("analytics".into(), "Analytics".into()),
                ("fintech".into(), "FinTech".into()),
                ("payment".into(), "Payment Solutions".into()),
                ("information security".into(), "Information Security".into()),
                ("regulatory".into(), "Regulatory Technology".into()),
                ("crm".into(), "CRM".into()),
                ("marketing automation".into(), "Marketing Automation".into()),
                ("sales automation".into(), "Sales Automation".into()),
            ],
            regions: vec![
                ("nordic".into(), "Nordics".into()),
                ("scandinav".into(), "Nordics".into()),
                ("sweden".into(), "Nordics".into()),
                ("norway".into(), "Nordics".into()),
                ("denmark".into(), "Nordics".into()),
                ("finland".into(), "Nordics".into()),
                ("europe".into(), "Europe".into()),
                ("north america".into(), "North America".into()),
                ("united states".into(), "North America".into()),
                (" us ".into(), "North America".into()),
                ("apac".into(), "APAC".into()),
                ("asia".into(), "APAC".into()),
                ("japan".into(), "Japan".into()),
                ("united kingdom".into(), "United Kingdom".into()),
                (" uk ".into(), "United Kingdom".into()),
                ("germany".into(), "Europe".into()),
                ("france".into(), "France".into()),
                ("global".into(), "Global".into()),
            ],
            entity_triggers: vec![
                "acquired by ".into(),
                "acquired ".into(),
                "investment from ".into(),
                "invested by ".into(),
                "backed by ".into(),
                "funded by ".into(),
                "partnership with ".into(),
                "partner ".into(),
                "competes with ".into(),
                "competitor ".into(),
                "competitors like ".into(),
                "competitors such as ".into(),
                "alternatives include ".into(),
                "compared to ".into(),
                "compared against ".into(),
                "compared against competitors like ".into(),
            ],
        }
    }
}

/// Extract graph hooks (investors, business areas, regions, competitors) from consolidated facts.
pub fn extract_hooks_from_facts(
    subject: &str,
    facts: &[DdFactSummary],
    patterns: &HookPatterns,
) -> DdHooks {
    let mut business_areas = std::collections::BTreeSet::new();
    let mut competitors = std::collections::BTreeSet::new();
    let mut investors = std::collections::BTreeSet::new();
    let mut regions = std::collections::BTreeSet::new();

    let subject_lower = subject.to_lowercase();

    for fact in facts {
        let claim = &fact.claim;
        let claim_lower = claim.to_lowercase();

        // Business areas from any claim
        for (pattern, label) in &patterns.business_areas {
            if claim_lower.contains(pattern.as_str()) {
                business_areas.insert(label.clone());
            }
        }

        // Competitors from competition claims
        match fact.category.as_str() {
            "competition" | "competitors" => {
                for name in extract_named_entities(claim, &subject_lower, &patterns.entity_triggers)
                {
                    competitors.insert(name);
                }
            }
            "financials" => {
                for name in extract_named_entities(claim, &subject_lower, &patterns.entity_triggers)
                {
                    investors.insert(name);
                }
            }
            _ => {}
        }

        // Regions from any claim
        let mut seen_regions = std::collections::HashSet::new();
        for (pattern, label) in &patterns.regions {
            if claim_lower.contains(pattern.as_str()) && seen_regions.insert(label.clone()) {
                regions.insert(label.clone());
            }
        }
    }

    DdHooks {
        investors: investors.into_iter().collect(),
        business_areas: business_areas.into_iter().collect(),
        regions: regions.into_iter().collect(),
        competitors: competitors.into_iter().collect(),
    }
}

fn extract_named_entities(claim: &str, exclude_lower: &str, triggers: &[String]) -> Vec<String> {
    let mut entities = Vec::new();
    let claim_lower = claim.to_lowercase();

    for trigger in triggers {
        if let Some(pos) = claim_lower.find(trigger.as_str()) {
            let after = &claim[pos + trigger.len()..];
            let entity = after
                .split([',', '.', ';', '(', ')'])
                .next()
                .unwrap_or("")
                .trim();
            if !entity.is_empty() && entity.len() < 60 && entity.to_lowercase() != exclude_lower {
                entities.push(entity.to_string());
            }
        }
    }

    entities
}

fn next_batch_bounds(
    total_items: usize,
    processed_items: usize,
    max_batch: usize,
) -> (usize, usize) {
    let start = processed_items.min(total_items);
    let end = (start + max_batch).min(total_items);
    (start, end)
}

fn consolidate_dd_fact_values<I>(values: I) -> Vec<DdFactSummary>
where
    I: IntoIterator<Item = serde_json::Value>,
{
    let mut by_signature: HashMap<String, DdFactSummary> = HashMap::new();
    for value in values {
        let Some(normalized) = normalize_dd_fact(&value) else {
            continue;
        };
        let Some(summary) = summary_from_normalized_fact(&normalized) else {
            continue;
        };
        let signature = dd_fact_signature(&normalized);
        if let Some(existing) = by_signature.get_mut(&signature) {
            merge_exact_summary(existing, summary);
        } else {
            by_signature.insert(signature, summary);
        }
    }

    let summaries: Vec<DdFactSummary> = by_signature.into_values().collect();
    if summaries.is_empty() {
        return Vec::new();
    }

    let token_frequencies = token_document_frequency(&summaries);
    let total_summaries = summaries.len();
    let mut candidates: Vec<ConsolidationCandidate> = summaries
        .into_iter()
        .map(|summary| build_consolidation_candidate(summary, &token_frequencies, total_summaries))
        .collect();
    candidates.sort_by(compare_candidates);

    let mut kept = Vec::new();
    let mut counts_by_category: HashMap<String, usize> = HashMap::new();
    for candidate in candidates {
        if kept
            .iter()
            .any(|existing| should_skip_candidate(&candidate, existing))
        {
            continue;
        }

        let count = counts_by_category
            .get(candidate.summary.category.as_str())
            .copied()
            .unwrap_or(0);
        if count >= category_fact_cap(candidate.summary.category.as_str()) {
            continue;
        }

        counts_by_category
            .entry(candidate.summary.category.clone())
            .and_modify(|value| *value += 1)
            .or_insert(1);
        kept.push(candidate);
    }

    kept.sort_by(compare_candidates);
    kept.into_iter()
        .map(|candidate| candidate.summary)
        .collect()
}

fn summary_from_normalized_fact(fact: &serde_json::Value) -> Option<DdFactSummary> {
    let category = fact.get("category")?.as_str()?.to_string();
    let claim = fact.get("claim")?.as_str()?.trim().to_string();
    if claim.is_empty() {
        return None;
    }

    let confidence = fact
        .get("confidence")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let evidence_count = fact
        .get("source_indices")
        .and_then(serde_json::Value::as_array)
        .map_or(1, |values| values.len().max(1));

    Some(DdFactSummary {
        category,
        claim,
        confidence,
        support_count: 1,
        evidence_count,
    })
}

#[allow(clippy::float_cmp)]
fn merge_exact_summary(existing: &mut DdFactSummary, candidate: DdFactSummary) {
    existing.support_count += candidate.support_count;
    existing.evidence_count += candidate.evidence_count;
    if candidate.confidence > existing.confidence
        || (candidate.confidence == existing.confidence
            && candidate.claim.len() > existing.claim.len())
    {
        existing.claim = candidate.claim;
    }
    existing.confidence = existing.confidence.max(candidate.confidence);
}

fn build_consolidation_candidate(
    summary: DdFactSummary,
    token_frequencies: &HashMap<String, usize>,
    total_summaries: usize,
) -> ConsolidationCandidate {
    let claim_tokens = informative_claim_tokens(&summary.claim);
    let topic_tokens: HashSet<String> = claim_tokens
        .iter()
        .filter(|token| !token.chars().any(|ch| ch.is_ascii_digit()))
        .cloned()
        .collect();
    let distinctive_tokens: HashSet<String> = claim_tokens
        .iter()
        .filter(|token| {
            token_frequencies.get(*token).copied().unwrap_or_default() * 2 <= total_summaries + 1
        })
        .cloned()
        .collect();
    let approximate = claim_is_approximate(&summary.claim);
    let numeric_tokens = numeric_claim_tokens(&summary.claim);
    let priority_score = fact_priority_score(&summary, approximate, numeric_tokens.len());

    ConsolidationCandidate {
        summary,
        distinctive_tokens: if distinctive_tokens.is_empty() {
            claim_tokens.iter().cloned().collect()
        } else {
            distinctive_tokens
        },
        topic_tokens: if topic_tokens.is_empty() {
            claim_tokens.iter().cloned().collect()
        } else {
            topic_tokens
        },
        numeric_tokens,
        approximate,
        priority_score,
    }
}

fn token_document_frequency(summaries: &[DdFactSummary]) -> HashMap<String, usize> {
    let mut frequencies = HashMap::new();
    for summary in summaries {
        let mut seen = HashSet::new();
        for token in informative_claim_tokens(&summary.claim) {
            if seen.insert(token.clone()) {
                frequencies
                    .entry(token)
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        }
    }
    frequencies
}

fn informative_claim_tokens(claim: &str) -> Vec<String> {
    canonicalize_claim(claim)
        .split_whitespace()
        .filter(|token| token.len() > 2 && !dd_stopwords().contains(token))
        .map(ToOwned::to_owned)
        .collect()
}

fn numeric_claim_tokens(claim: &str) -> Vec<String> {
    canonicalize_claim(claim)
        .split_whitespace()
        .filter(|token| token.chars().any(|ch| ch.is_ascii_digit()))
        .map(ToOwned::to_owned)
        .collect()
}

fn claim_is_approximate(claim: &str) -> bool {
    let normalized = claim.to_ascii_lowercase();
    [
        "estimated",
        "estimate",
        "approximately",
        "approx",
        "about ",
        "over ",
        "under ",
        "close to",
        "around ",
        "range of",
        "between ",
        "currently",
        "historically",
        "more than",
        "less than",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

#[allow(clippy::cast_precision_loss)]
fn fact_priority_score(
    summary: &DdFactSummary,
    approximate: bool,
    numeric_token_count: usize,
) -> f64 {
    let confidence_score = summary.confidence * 100.0;
    let support_bonus = summary.support_count as f64 * 6.0;
    let evidence_bonus = summary.evidence_count as f64 * 2.0;
    let exactness_bonus = if approximate { 0.0 } else { 5.0 };
    let numeric_bonus = if numeric_token_count > 0 { 2.0 } else { 0.0 };
    confidence_score + support_bonus + evidence_bonus + exactness_bonus + numeric_bonus
}

fn compare_candidates(left: &ConsolidationCandidate, right: &ConsolidationCandidate) -> Ordering {
    category_sort_order(left.summary.category.as_str())
        .cmp(&category_sort_order(right.summary.category.as_str()))
        .then_with(|| {
            right
                .priority_score
                .partial_cmp(&left.priority_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| right.summary.claim.len().cmp(&left.summary.claim.len()))
}

fn category_sort_order(category: &str) -> usize {
    match category {
        "product" => 0,
        "customers" => 1,
        "technology" => 2,
        "competition" => 3,
        "market" => 4,
        "financials" => 5,
        "team" => 6,
        "governance" => 7,
        "risk" => 8,
        _ => 9,
    }
}

fn category_fact_cap(category: &str) -> usize {
    match category {
        "technology" => 5,
        "financials" => 4,
        "customers" | "competition" => 3,
        _ => 2,
    }
}

fn should_skip_candidate(
    candidate: &ConsolidationCandidate,
    existing: &ConsolidationCandidate,
) -> bool {
    if candidate.summary.category != existing.summary.category {
        return false;
    }

    let similarity = token_similarity(&candidate.distinctive_tokens, &existing.distinctive_tokens);
    let topic_similarity = token_similarity(&candidate.topic_tokens, &existing.topic_tokens);
    if similarity >= 0.86 {
        return true;
    }

    if !candidate.numeric_tokens.is_empty()
        && candidate.numeric_tokens == existing.numeric_tokens
        && similarity >= 0.55
    {
        return true;
    }

    candidate.approximate
        && topic_similarity >= 0.5
        && (!existing.approximate || candidate.numeric_tokens == existing.numeric_tokens)
}

#[allow(clippy::cast_precision_loss)]
fn token_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn dd_stopwords() -> &'static [&'static str] {
    &[
        "and",
        "for",
        "the",
        "with",
        "into",
        "that",
        "from",
        "their",
        "this",
        "those",
        "these",
        "across",
        "through",
        "using",
        "used",
        "helps",
        "help",
        "offer",
        "offers",
        "provides",
        "provide",
        "company",
        "companies",
        "solution",
        "solutions",
    ]
}

fn existing_fact_signature(content: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(content).ok()?;
    let normalized = normalize_dd_fact(&value)?;
    Some(dd_fact_signature(&normalized))
}

fn normalize_dd_fact(fact: &serde_json::Value) -> Option<serde_json::Value> {
    let claim = fact.get("claim")?.as_str()?.trim();
    if claim.is_empty() {
        return None;
    }

    let category = normalize_dd_category(
        fact.get("category").and_then(serde_json::Value::as_str),
        claim,
    );
    let confidence = fact
        .get("confidence")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);
    let source_indices = fact
        .get("source_indices")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter(|value| value.is_i64() || value.is_u64())
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(serde_json::json!({
        "claim": claim,
        "category": category,
        "source_indices": source_indices,
        "confidence": confidence,
    }))
}

fn normalize_dd_category(raw_category: Option<&str>, claim: &str) -> &'static str {
    match raw_category
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "product" => {
            if claim_looks_technical(claim) {
                "technology"
            } else {
                "product"
            }
        }
        "customer" | "customers" => "customers",
        "technology" | "tech" | "platform" | "architecture" | "engineering" | "integrations"
        | "integration" | "stack" => "technology",
        "competition" | "competitor" | "competitors" => "competition",
        "market" | "positioning" => "market",
        "financial" | "financials" | "finance" | "funding" | "ownership" | "investors" => {
            "financials"
        }
        "team" | "leadership" | "management" => "team",
        "risk" => "risk",
        "governance" => "governance",
        _ => infer_dd_category_from_claim(claim),
    }
}

fn infer_dd_category_from_claim(claim: &str) -> &'static str {
    let claim = claim.to_ascii_lowercase();
    if claim_looks_technical(&claim) {
        "technology"
    } else if claim.contains("customer")
        || claim.contains("clients")
        || claim.contains("serves ")
        || claim.contains("countries")
    {
        "customers"
    } else if claim.contains("funding")
        || claim.contains("raised")
        || claim.contains("investor")
        || claim.contains("acquired")
        || claim.contains("revenue")
        || claim.contains("arr")
    {
        "financials"
    } else if claim.contains("competitor") || claim.contains("competes") {
        "competition"
    } else if claim.contains("market") || claim.contains("major player") || claim.contains("idc") {
        "market"
    } else if claim.contains("chief ")
        || claim.contains("officer")
        || claim.contains("executive")
        || claim.contains("leadership")
    {
        "team"
    } else {
        "product"
    }
}

fn claim_looks_technical(claim: &str) -> bool {
    let claim = claim.to_ascii_lowercase();
    [
        "technology",
        "architecture",
        "platform",
        "integration",
        "integrations",
        "api",
        "apis",
        "cloud",
        "threat intelligence",
        "attack surface",
        "monitor",
        "monitoring",
        "ctem",
        "exposure management",
        "internet-facing",
        "dark web",
        "open web",
        "deep web",
        "technical moat",
    ]
    .iter()
    .any(|needle| claim.contains(needle))
}

fn dd_fact_signature(fact: &serde_json::Value) -> String {
    let category = fact
        .get("category")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("product");
    let claim = fact
        .get("claim")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    format!("{category}:{}", canonicalize_claim(claim))
}

fn canonicalize_claim(claim: &str) -> String {
    claim
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn covered_dd_categories(hypotheses: &[Fact]) -> Vec<String> {
    let mut categories: Vec<String> = hypotheses
        .iter()
        .filter_map(|fact| {
            let value = serde_json::from_str::<serde_json::Value>(&fact.content).ok()?;
            let normalized = normalize_dd_fact(&value)?;
            normalized["category"].as_str().map(ToOwned::to_owned)
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    categories.sort();
    categories
}

fn missing_expected_dd_categories(hypotheses: &[Fact]) -> Vec<&'static str> {
    let covered: HashSet<String> = covered_dd_categories(hypotheses).into_iter().collect();
    expected_dd_categories()
        .into_iter()
        .filter(|category| !covered.contains(*category))
        .collect()
}

fn expected_dd_categories() -> [&'static str; 6] {
    [
        "product",
        "customers",
        "technology",
        "competition",
        "market",
        "financials",
    ]
}

fn parse_json_array_response(
    raw: &str,
    field_name: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let cleaned = strip_fences(raw);
    try_parse_json_array(cleaned, field_name).or_else(|first_error| {
        extract_first_json_value(cleaned)
            .filter(|candidate| *candidate != cleaned)
            .ok_or(first_error.clone())
            .and_then(|candidate| {
                try_parse_json_array(candidate, field_name).map_err(|second_error| {
                    format!("{first_error}; recovered JSON failed: {second_error}")
                })
            })
    })
}

fn try_parse_json_array(raw: &str, field_name: &str) -> Result<Vec<serde_json::Value>, String> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Array(values)) => Ok(values),
        Ok(serde_json::Value::Object(map)) => map
            .get(field_name)
            .and_then(serde_json::Value::as_array)
            .cloned()
            .ok_or_else(|| format!("expected object field `{field_name}` containing an array")),
        Ok(_) => Err(format!(
            "expected top-level JSON array or object with `{field_name}`"
        )),
        Err(error) => Err(error.to_string()),
    }
}

fn extract_first_json_value(raw: &str) -> Option<&str> {
    let (start, _) = raw.char_indices().find(|(_, ch)| matches!(ch, '{' | '['))?;
    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in raw[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '}' | ']' => {
                if stack.pop() != Some(ch) {
                    return None;
                }
                if stack.is_empty() {
                    let end = start + offset + ch.len_utf8();
                    return Some(&raw[start..end]);
                }
            }
            _ => {}
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use converge_pack::{Context, ContextKey, Fact, ProposedFact, Suggestor};

    use super::{
        DdError, DdLlm, SharedBudget, SynthesisSuggestor, canonicalize_claim,
        consolidate_dd_fact_values, extract_first_json_value, next_batch_bounds, normalize_dd_fact,
        parse_json_array_response,
    };

    struct StubLlm;

    #[async_trait::async_trait]
    impl DdLlm for StubLlm {
        async fn complete(&self, prompt: &str) -> Result<String, DdError> {
            let _ = prompt;
            Ok("{}".to_string())
        }
    }

    struct StubContext {
        hypothesis_count: usize,
        has_proposals: bool,
    }

    impl Context for StubContext {
        fn has(&self, key: ContextKey) -> bool {
            match key {
                ContextKey::Hypotheses => self.hypothesis_count > 0,
                ContextKey::Proposals => self.has_proposals,
                _ => false,
            }
        }

        fn get(&self, _key: ContextKey) -> &[Fact] {
            &[]
        }

        fn get_proposals(&self, _key: ContextKey) -> &[ProposedFact] {
            &[]
        }

        fn count(&self, key: ContextKey) -> usize {
            match key {
                ContextKey::Hypotheses => self.hypothesis_count,
                _ => 0,
            }
        }
    }

    #[test]
    fn synthesis_suggestor_is_always_schedulable() {
        let budget = Arc::new(SharedBudget::new().with_limit("llm", 1));
        let suggestor = SynthesisSuggestor::new("Acme", budget, Arc::new(StubLlm));

        assert!(suggestor.dependencies().is_empty());
    }

    #[test]
    fn synthesis_accepts_after_hypotheses_stabilize() {
        let budget = Arc::new(SharedBudget::new().with_limit("llm", 1));
        let suggestor = SynthesisSuggestor::new("Acme", budget, Arc::new(StubLlm))
            .with_required_stable_cycles(2);

        let first_fact_wave = StubContext {
            hypothesis_count: 5,
            has_proposals: false,
        };
        let first_stable_cycle = StubContext {
            hypothesis_count: 5,
            has_proposals: false,
        };
        let second_stable_cycle = StubContext {
            hypothesis_count: 5,
            has_proposals: false,
        };

        assert!(!suggestor.accepts(&first_fact_wave));
        assert!(!suggestor.accepts(&first_stable_cycle));
        assert!(suggestor.accepts(&second_stable_cycle));
    }

    #[test]
    fn parse_json_array_response_accepts_wrapped_object() {
        let parsed = parse_json_array_response(
            r#"{"facts":[{"claim":"Acme sells software","confidence":0.9}]}"#,
            "facts",
        )
        .expect("wrapped array should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["claim"], "Acme sells software");
    }

    #[test]
    fn parse_json_array_response_accepts_legacy_array_shape() {
        let parsed = parse_json_array_response(
            r#"[{"query":"Acme competitors","mode":"breadth","reason":"market"}]"#,
            "strategies",
        )
        .expect("legacy array should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["query"], "Acme competitors");
    }

    #[test]
    fn parse_json_array_response_recovers_json_from_prose() {
        let parsed = parse_json_array_response(
            "Here is the JSON you requested:\n```json\n{\"facts\":[{\"claim\":\"Acme grows\",\"confidence\":0.7}]}\n```\nThanks.",
            "facts",
        )
        .expect("embedded JSON should parse");

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["claim"], "Acme grows");
    }

    #[test]
    fn extract_first_json_value_handles_nested_arrays_and_objects() {
        let extracted = extract_first_json_value(
            "prefix {\"facts\":[{\"claim\":\"A\",\"source_indices\":[0,1]}]} suffix",
        )
        .expect("should find first JSON value");

        assert_eq!(
            extracted,
            r#"{"facts":[{"claim":"A","source_indices":[0,1]}]}"#
        );
    }

    #[test]
    fn next_batch_bounds_advances_through_unprocessed_signals() {
        assert_eq!(next_batch_bounds(37, 0, 15), (0, 15));
        assert_eq!(next_batch_bounds(37, 15, 15), (15, 30));
        assert_eq!(next_batch_bounds(37, 30, 15), (30, 37));
        assert_eq!(next_batch_bounds(37, 37, 15), (37, 37));
    }

    #[test]
    fn normalize_dd_fact_reclassifies_technical_product_claims() {
        let normalized = normalize_dd_fact(&serde_json::json!({
            "claim": "Outpost24's Sweepatic Platform monitors internet-facing assets for attack surface management.",
            "category": "product",
            "source_indices": [0],
            "confidence": 0.9,
        }))
        .expect("fact should normalize");

        assert_eq!(normalized["category"], "technology");
    }

    #[test]
    fn canonicalize_claim_ignores_case_and_punctuation() {
        assert_eq!(
            canonicalize_claim("Outpost24 raised $23.8M!"),
            canonicalize_claim("outpost24 raised 23 8m")
        );
    }

    #[test]
    fn consolidate_dd_fact_values_merges_exact_duplicates() {
        let summaries = consolidate_dd_fact_values(vec![
            serde_json::json!({
                "claim": "Outpost24 offers a 100% open API for easy integration into security operations.",
                "category": "technology",
                "source_indices": [0],
                "confidence": 0.9,
            }),
            serde_json::json!({
                "claim": "Outpost24 offers a 100% open API for easy integration into security operations.",
                "category": "technology",
                "source_indices": [1],
                "confidence": 0.8,
            }),
        ]);

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].support_count, 2);
        assert_eq!(summaries[0].evidence_count, 2);
    }

    #[test]
    fn consolidate_dd_fact_values_drops_vague_same_topic_repeats() {
        let summaries = consolidate_dd_fact_values(vec![
            serde_json::json!({
                "claim": "Outpost24 has 195 employees.",
                "category": "team",
                "source_indices": [0],
                "confidence": 0.9,
            }),
            serde_json::json!({
                "claim": "Outpost24 has over 200 employees.",
                "category": "team",
                "source_indices": [1],
                "confidence": 0.7,
            }),
            serde_json::json!({
                "claim": "Outpost24 offers a 100% open API for easy integration into security operations.",
                "category": "technology",
                "source_indices": [2],
                "confidence": 0.9,
            }),
        ]);

        let team_facts: Vec<_> = summaries
            .iter()
            .filter(|summary| summary.category == "team")
            .collect();
        assert_eq!(team_facts.len(), 1);
        assert_eq!(team_facts[0].claim, "Outpost24 has 195 employees.");
    }

    #[test]
    fn consolidate_dd_fact_values_preserves_conflicting_exact_financials() {
        let summaries = consolidate_dd_fact_values(vec![
            serde_json::json!({
                "claim": "Outpost24's 2023 revenue was $42.19M.",
                "category": "financials",
                "source_indices": [0],
                "confidence": 0.9,
            }),
            serde_json::json!({
                "claim": "Outpost24 generates $67.5 million in revenue.",
                "category": "financials",
                "source_indices": [1],
                "confidence": 0.9,
            }),
        ]);

        assert_eq!(summaries.len(), 2);
    }

    // ── Negative tests ────────────────────────────────────────────

    #[test]
    fn normalize_dd_fact_rejects_empty_claim() {
        assert!(
            normalize_dd_fact(&serde_json::json!({
                "claim": "",
                "category": "product",
            }))
            .is_none()
        );
    }

    #[test]
    fn normalize_dd_fact_rejects_whitespace_only_claim() {
        assert!(
            normalize_dd_fact(&serde_json::json!({
                "claim": "   ",
                "category": "product",
            }))
            .is_none()
        );
    }

    #[test]
    fn normalize_dd_fact_rejects_missing_claim() {
        assert!(
            normalize_dd_fact(&serde_json::json!({
                "category": "product",
            }))
            .is_none()
        );
    }

    #[test]
    fn normalize_dd_fact_clamps_confidence() {
        let normalized = normalize_dd_fact(&serde_json::json!({
            "claim": "test",
            "category": "product",
            "confidence": 5.0,
        }))
        .unwrap();
        assert_eq!(normalized["confidence"], 1.0);

        let normalized = normalize_dd_fact(&serde_json::json!({
            "claim": "test",
            "category": "product",
            "confidence": -1.0,
        }))
        .unwrap();
        assert_eq!(normalized["confidence"], 0.0);
    }

    #[test]
    fn normalize_dd_fact_defaults_missing_confidence() {
        let normalized = normalize_dd_fact(&serde_json::json!({
            "claim": "test claim",
            "category": "product",
        }))
        .unwrap();
        assert_eq!(normalized["confidence"], 0.5);
    }

    #[test]
    fn normalize_dd_fact_filters_non_integer_source_indices() {
        let normalized = normalize_dd_fact(&serde_json::json!({
            "claim": "test",
            "category": "product",
            "source_indices": [0, "bad", 2, null, 3],
        }))
        .unwrap();
        let indices = normalized["source_indices"].as_array().unwrap();
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn parse_json_array_response_rejects_plain_text() {
        assert!(parse_json_array_response("just some text", "facts").is_err());
    }

    #[test]
    fn parse_json_array_response_rejects_object_with_wrong_field() {
        assert!(parse_json_array_response(r#"{"results":[{"claim":"X"}]}"#, "facts").is_err());
    }

    #[test]
    fn parse_json_array_response_rejects_scalar() {
        assert!(parse_json_array_response("42", "facts").is_err());
        assert!(parse_json_array_response("true", "facts").is_err());
        assert!(parse_json_array_response(r#""string""#, "facts").is_err());
    }

    #[test]
    fn extract_first_json_value_returns_none_for_no_json() {
        assert!(extract_first_json_value("no json here").is_none());
    }

    #[test]
    fn extract_first_json_value_returns_none_for_mismatched_braces() {
        assert!(extract_first_json_value("{unclosed").is_none());
        assert!(extract_first_json_value("[}").is_none());
    }

    #[test]
    fn extract_first_json_value_handles_escaped_quotes_in_strings() {
        let result = extract_first_json_value(r#"prefix {"key":"val\"ue"} suffix"#);
        assert!(result.is_some());
        let parsed: serde_json::Value = serde_json::from_str(result.unwrap()).unwrap();
        assert_eq!(parsed["key"], r#"val"ue"#);
    }

    #[test]
    fn consolidate_dd_fact_values_handles_empty_input() {
        assert!(consolidate_dd_fact_values(vec![]).is_empty());
    }

    #[test]
    fn consolidate_dd_fact_values_handles_all_invalid_facts() {
        let summaries = consolidate_dd_fact_values(vec![
            serde_json::json!({"claim": "", "category": "product"}),
            serde_json::json!({"no_claim": true}),
            serde_json::json!(null),
        ]);
        assert!(summaries.is_empty());
    }

    #[test]
    fn next_batch_bounds_zero_total() {
        assert_eq!(next_batch_bounds(0, 0, 15), (0, 0));
    }

    #[test]
    fn next_batch_bounds_processed_exceeds_total() {
        assert_eq!(next_batch_bounds(5, 100, 15), (5, 5));
    }

    #[test]
    fn canonicalize_claim_handles_empty_string() {
        assert_eq!(canonicalize_claim(""), "");
    }

    #[test]
    fn canonicalize_claim_handles_only_punctuation() {
        assert_eq!(canonicalize_claim("!!!...???"), "");
    }

    #[test]
    fn synthesis_does_not_accept_when_proposals_exist() {
        let budget = Arc::new(SharedBudget::new().with_limit("llm", 1));
        let suggestor = SynthesisSuggestor::new("Acme", budget, Arc::new(StubLlm));

        let ctx_with_proposals = StubContext {
            hypothesis_count: 10,
            has_proposals: true,
        };
        assert!(!suggestor.accepts(&ctx_with_proposals));
    }

    #[test]
    fn synthesis_does_not_accept_without_hypotheses() {
        let budget = Arc::new(SharedBudget::new().with_limit("llm", 1));
        let suggestor = SynthesisSuggestor::new("Acme", budget, Arc::new(StubLlm));

        let empty_ctx = StubContext {
            hypothesis_count: 0,
            has_proposals: false,
        };
        assert!(!suggestor.accepts(&empty_ctx));
    }

    #[test]
    fn dd_error_infra_vs_non_infra() {
        assert!(
            DdError::CreditsExhausted {
                provider: "x".into(),
                detail: "y".into()
            }
            .is_infra_failure()
        );
        assert!(
            DdError::RateLimited {
                provider: "x".into(),
                retry_after_ms: None
            }
            .is_infra_failure()
        );
        assert!(
            DdError::ProviderUnavailable {
                provider: "x".into(),
                detail: "y".into()
            }
            .is_infra_failure()
        );

        assert!(
            !DdError::BadResponse {
                provider: "x".into(),
                detail: "y".into()
            }
            .is_infra_failure()
        );
        assert!(
            !DdError::ParseFailed {
                provider: "x".into(),
                detail: "y".into()
            }
            .is_infra_failure()
        );
        assert!(
            !DdError::PromptTooLarge {
                provider: "x".into(),
                tokens: None
            }
            .is_infra_failure()
        );
    }

    #[test]
    fn dd_error_only_credits_exhausted_is_fatal() {
        assert!(
            DdError::CreditsExhausted {
                provider: "x".into(),
                detail: "y".into()
            }
            .is_fatal()
        );
        assert!(
            !DdError::RateLimited {
                provider: "x".into(),
                retry_after_ms: None
            }
            .is_fatal()
        );
        assert!(
            !DdError::ProviderUnavailable {
                provider: "x".into(),
                detail: "y".into()
            }
            .is_fatal()
        );
    }

    // ── Proptests ─────────────────────────────────────────────────

    #[allow(clippy::cast_precision_loss)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn canonicalize_is_idempotent(claim in ".*") {
                let first = canonicalize_claim(&claim);
                let second = canonicalize_claim(&first);
                prop_assert_eq!(first, second);
            }

            #[test]
            fn canonicalize_is_case_insensitive(claim in "[a-zA-Z0-9 ]{1,100}") {
                prop_assert_eq!(
                    canonicalize_claim(&claim),
                    canonicalize_claim(&claim.to_uppercase())
                );
            }

            #[test]
            fn normalize_dd_fact_never_panics(
                claim in ".*",
                category in ".*",
                confidence in proptest::num::f64::ANY,
            ) {
                let _ = normalize_dd_fact(&serde_json::json!({
                    "claim": claim,
                    "category": category,
                    "confidence": confidence,
                }));
            }

            #[test]
            fn normalize_preserves_non_empty_claims(
                claim in "[a-zA-Z]{1,50}",
                category in prop_oneof![
                    Just("product"), Just("technology"), Just("financials"),
                    Just("customers"), Just("competition"), Just("market"),
                ],
            ) {
                let normalized = normalize_dd_fact(&serde_json::json!({
                    "claim": claim,
                    "category": category,
                    "confidence": 0.8,
                }));
                prop_assert!(normalized.is_some());
                let n = normalized.unwrap();
                prop_assert!(!n["claim"].as_str().unwrap().is_empty());
            }

            #[test]
            fn consolidate_never_panics(
                n in 0_usize..20,
            ) {
                let categories = ["product", "technology", "financials"];
                let facts: Vec<serde_json::Value> = (0..n).map(|i| {
                    serde_json::json!({
                        "claim": format!("Fact number {i} about the company"),
                        "category": categories[i % 3],
                        "source_indices": [i],
                        "confidence": 0.5 + (i as f64 * 0.02),
                    })
                }).collect();
                let result = consolidate_dd_fact_values(facts);
                prop_assert!(result.len() <= n);
            }

            #[test]
            fn next_batch_bounds_always_valid(
                total in 0_usize..1000,
                processed in 0_usize..1000,
                max_batch in 1_usize..100,
            ) {
                let (start, end) = next_batch_bounds(total, processed, max_batch);
                prop_assert!(start <= total);
                prop_assert!(end <= total);
                prop_assert!(start <= end);
                prop_assert!(end - start <= max_batch);
            }

            #[test]
            fn extract_first_json_value_never_panics(input in ".*") {
                let _ = extract_first_json_value(&input);
            }

            #[test]
            fn parse_json_array_response_never_panics(
                input in ".*",
                field in "[a-z]{1,10}",
            ) {
                let _ = parse_json_array_response(&input, &field);
            }
        }
    }
}
