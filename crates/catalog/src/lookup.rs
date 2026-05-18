//! [`CatalogLookup`] trait + two reference implementations:
//! [`KeywordLookup`] (deterministic, structural+substring) and
//! [`ChatBackendLookup`] (advisory rerank via any
//! [`converge_provider::DynChatBackend`]).
//!
//! ## Advisory, not authoritative
//!
//! `CatalogLookup::suggest` returns advisory rankings only. Callers
//! **MUST** verify each returned id exists in the catalog and then apply
//! the deterministic filters owned by the host — context-key availability,
//! fact-family compatibility, backend requirements, replay mode, cost,
//! governance class, and any host-specific policy — before any executable
//! selection.
//!
//! LLM output is grist for ranking, never authority.

use std::sync::Arc;

use async_trait::async_trait;
use converge_provider::{
    ChatMessage, ChatRequest, ChatRole, DynChatBackend, LlmError, ResponseFormat,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{CatalogSuggestorDescriptor, DiscoveryCatalog};

/// Advisory ranking result from a catalog lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Id of a [`crate::CatalogSuggestorDescriptor`] in the catalog. Callers
    /// MUST re-verify presence via [`DiscoveryCatalog::get`] before acting.
    pub descriptor_id: String,
    /// Implementation-defined score in `[0.0, 1.0]` where higher is more
    /// relevant. Backends should document their scoring semantics.
    pub score: f32,
    /// Optional human-readable rationale. LLM backends typically populate
    /// this; keyword backends typically don't.
    pub rationale: Option<String>,
}

/// Errors returned from advisory lookup.
#[derive(Debug, Error)]
pub enum LookupError {
    /// The backend ran but produced output that could not be parsed.
    #[error("lookup backend produced unparseable output: {0}")]
    UnparseableOutput(String),
    /// The backend ran but referenced descriptor ids that do not exist in
    /// the catalog. These are dropped before [`Suggestion`]s are returned;
    /// this error is reserved for the case where ALL referenced ids are
    /// invalid (i.e. no valid suggestion remained).
    #[error("lookup backend returned no valid descriptor ids")]
    NoValidIds,
    /// The underlying chat backend failed.
    #[error("chat backend error: {0}")]
    Chat(#[from] LlmError),
}

/// Catalog lookup contract — backends advise on which descriptors fit a
/// natural-language query.
#[async_trait]
pub trait CatalogLookup: Send + Sync {
    /// Suggest up to `limit` descriptor ids that match the query, ranked
    /// by relevance. Advisory only — see module docs.
    async fn suggest(
        &self,
        query: &str,
        catalog: &DiscoveryCatalog,
        limit: usize,
    ) -> Result<Vec<Suggestion>, LookupError>;
}

// ---------------------------------------------------------------------------
// KeywordLookup — deterministic, no provider
// ---------------------------------------------------------------------------

/// Deterministic substring + structural lookup. No network, no provider.
/// Suitable as a default backend and as the only backend in tests.
///
/// Scoring is the sum of: 3.0 per `summary` hit, 2.0 per `use_when` hit,
/// 1.0 per `examples` hit. Score is normalized to `[0.0, 1.0]` by dividing
/// by the maximum observed raw score in the result set.
#[derive(Debug, Clone, Default)]
pub struct KeywordLookup;

impl KeywordLookup {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CatalogLookup for KeywordLookup {
    async fn suggest(
        &self,
        query: &str,
        catalog: &DiscoveryCatalog,
        limit: usize,
    ) -> Result<Vec<Suggestion>, LookupError> {
        let needle = query.to_lowercase();
        let mut scored: Vec<(f32, &CatalogSuggestorDescriptor)> = catalog
            .iter()
            .map(|entry| (raw_score(entry, &needle), entry))
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|(a, ea), (b, eb)| b.total_cmp(a).then_with(|| ea.id().cmp(eb.id())));

        let max_score = scored.first().map_or(1.0, |(s, _)| *s);
        Ok(scored
            .into_iter()
            .take(limit)
            .map(|(score, entry)| Suggestion {
                descriptor_id: entry.id().to_string(),
                score: if max_score > 0.0 {
                    score / max_score
                } else {
                    0.0
                },
                rationale: None,
            })
            .collect())
    }
}

fn raw_score(entry: &CatalogSuggestorDescriptor, needle_lowercase: &str) -> f32 {
    let d = &entry.discovery;
    let summary_hits = count_hits(&d.summary, needle_lowercase);
    let use_when_hits = count_hits(&d.use_when, needle_lowercase);
    let example_hits: usize = d
        .examples
        .iter()
        .map(|ex| count_hits(ex, needle_lowercase))
        .sum();
    (summary_hits as f32) * 3.0 + (use_when_hits as f32) * 2.0 + (example_hits as f32)
}

fn count_hits(haystack: &str, needle_lowercase: &str) -> usize {
    if needle_lowercase.is_empty() {
        return 0;
    }
    haystack.to_lowercase().matches(needle_lowercase).count()
}

// ---------------------------------------------------------------------------
// ChatBackendLookup — advisory rerank via any DynChatBackend
// ---------------------------------------------------------------------------

/// LLM-backed advisory lookup. Provider-agnostic: takes any
/// [`DynChatBackend`] (Claude, OpenAI, local, mock).
///
/// The lookup serializes the catalog's discovery metadata into a compact
/// prompt and asks the model to rank descriptor ids by relevance to the
/// query. Returned ids are verified against the catalog; unknown ids are
/// dropped. Output is advisory — see module docs.
pub struct ChatBackendLookup {
    backend: Arc<dyn DynChatBackend>,
    model: Option<String>,
    max_tokens: u32,
}

impl ChatBackendLookup {
    /// Builds a chat-backend lookup using the host-provided backend.
    /// `model` is optional and forwarded to the backend; pass `None` to
    /// let the backend choose its default.
    #[must_use]
    pub fn new(backend: Arc<dyn DynChatBackend>) -> Self {
        Self {
            backend,
            model: None,
            max_tokens: 1024,
        }
    }

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

impl std::fmt::Debug for ChatBackendLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatBackendLookup")
            .field("model", &self.model)
            .field("max_tokens", &self.max_tokens)
            .field("backend", &"<dyn DynChatBackend>")
            .finish()
    }
}

#[async_trait]
impl CatalogLookup for ChatBackendLookup {
    async fn suggest(
        &self,
        query: &str,
        catalog: &DiscoveryCatalog,
        limit: usize,
    ) -> Result<Vec<Suggestion>, LookupError> {
        if catalog.is_empty() {
            return Ok(Vec::new());
        }

        let prompt = build_prompt(query, catalog, limit);
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: prompt,
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            system: Some(SYSTEM_INSTRUCTION.to_string()),
            tools: Vec::new(),
            response_format: ResponseFormat::Json,
            max_tokens: Some(self.max_tokens),
            temperature: Some(0.0),
            stop_sequences: Vec::new(),
            model: self.model.clone(),
        };

        let response = self.backend.chat(request).await?;
        let parsed: Vec<RawSuggestion> = serde_json::from_str(&response.content)
            .map_err(|err| LookupError::UnparseableOutput(err.to_string()))?;

        let valid: Vec<Suggestion> = parsed
            .into_iter()
            .filter_map(|raw| {
                catalog.get(&raw.descriptor_id).map(|_| Suggestion {
                    descriptor_id: raw.descriptor_id,
                    score: raw.score.clamp(0.0, 1.0),
                    rationale: raw.rationale,
                })
            })
            .take(limit)
            .collect();

        if valid.is_empty() {
            Err(LookupError::NoValidIds)
        } else {
            Ok(valid)
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawSuggestion {
    descriptor_id: String,
    score: f32,
    #[serde(default)]
    rationale: Option<String>,
}

const SYSTEM_INSTRUCTION: &str = "You are a catalog reranker. \
You will receive a user query and a JSON list of catalog entries, each with a \
descriptor_id and short discovery metadata (summary, use_when, examples, \
loop_contributions). Rank the entries by how well each fits the query. \
Respond with a JSON array of objects with this exact shape: \
[{\"descriptor_id\": \"...\", \"score\": 0.0, \"rationale\": \"...\"}]. \
score is a float in [0.0, 1.0] where higher means more relevant. \
rationale is one short sentence. Only include entries you would actually \
recommend; do not pad the list. Never invent descriptor_ids — use only ids \
from the input.";

fn build_prompt(query: &str, catalog: &DiscoveryCatalog, limit: usize) -> String {
    let entries: Vec<serde_json::Value> = catalog
        .iter()
        .map(|entry| {
            serde_json::json!({
                "descriptor_id": entry.id(),
                "summary": entry.discovery.summary,
                "use_when": entry.discovery.use_when,
                "examples": entry.discovery.examples,
                "loop_contributions": entry.discovery.loop_contributions,
            })
        })
        .collect();

    format!(
        "Query: {query}\n\nCatalog ({n} entries):\n{catalog_json}\n\n\
         Return at most {limit} ranked suggestions, best first.",
        query = query,
        n = entries.len(),
        catalog_json = serde_json::to_string_pretty(&entries).unwrap_or_default(),
        limit = limit,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use converge_kernel::formation::{ProfileSnapshot, SuggestorCapability, SuggestorRole};
    use converge_pack::FactFamilyId;
    use converge_provider::{
        BoxFuture, ChatResponse, CostClass, FinishReason, LatencyClass, TokenUsage,
    };

    use super::*;
    use crate::{DiscoveryMetadata, SuggestorDescriptor};

    fn snap(name: &str) -> ProfileSnapshot {
        ProfileSnapshot {
            name: name.to_string(),
            role: SuggestorRole::Signal,
            output_keys: Vec::new(),
            cost_hint: CostClass::Low,
            latency_hint: LatencyClass::Interactive,
            capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
            confidence_min: 0.7,
            confidence_max: 0.95,
        }
    }

    fn entry(
        id: &str,
        summary: &str,
        use_when: &str,
        examples: Vec<&str>,
    ) -> CatalogSuggestorDescriptor {
        let descriptor = SuggestorDescriptor::new(id, snap(id));
        let mut discovery = DiscoveryMetadata::new(summary, use_when);
        for ex in examples {
            discovery = discovery.with_example(ex);
        }
        discovery = discovery.with_produces(FactFamilyId::from(format!("test.{id}")));
        CatalogSuggestorDescriptor::new(descriptor, discovery)
    }

    fn fixture() -> DiscoveryCatalog {
        DiscoveryCatalog::new()
            .with_entry(entry(
                "gleif",
                "Look up legal entity in GLEIF LEI registry.",
                "When verifying a company is a registered legal entity.",
                vec![
                    "verify this vendor is a real company",
                    "find the LEI for Acme",
                ],
            ))
            .with_entry(entry(
                "ofac",
                "Screen entity against OFAC sanctions lists.",
                "When checking a vendor against US sanctions.",
                vec!["sanctions check"],
            ))
            .with_entry(entry(
                "cpsat",
                "CP-SAT constraint solver.",
                "When you need an optimal assignment under constraints.",
                vec!["pick the best schedule"],
            ))
    }

    // --- KeywordLookup --------------------------------------------------

    #[tokio::test]
    async fn keyword_lookup_returns_matches_ranked_by_score() {
        let catalog = fixture();
        let lookup = KeywordLookup::new();
        let results = lookup.suggest("vendor", &catalog, 10).await.unwrap();

        assert!(!results.is_empty());
        // The top result should be one of the vendor-related entries.
        let top_id = &results[0].descriptor_id;
        assert!(matches!(top_id.as_str(), "gleif" | "ofac"));
        // Top score normalizes to 1.0.
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn keyword_lookup_is_case_insensitive() {
        let catalog = fixture();
        let lookup = KeywordLookup::new();
        let upper = lookup.suggest("LEGAL", &catalog, 10).await.unwrap();
        let lower = lookup.suggest("legal", &catalog, 10).await.unwrap();
        let upper_ids: Vec<_> = upper.iter().map(|s| &s.descriptor_id).collect();
        let lower_ids: Vec<_> = lower.iter().map(|s| &s.descriptor_id).collect();
        assert_eq!(upper_ids, lower_ids);
    }

    #[tokio::test]
    async fn keyword_lookup_returns_empty_for_no_match() {
        let catalog = fixture();
        let lookup = KeywordLookup::new();
        let results = lookup
            .suggest("zzz-nothing-matches-this", &catalog, 10)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn keyword_lookup_respects_limit() {
        let catalog = fixture();
        let lookup = KeywordLookup::new();
        let results = lookup.suggest("when", &catalog, 2).await.unwrap();
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn keyword_lookup_summary_weighted_higher_than_examples() {
        // "constraint" appears in cpsat's summary; "constraint" also in
        // its use_when. Ensure cpsat scores higher than any entry where
        // "constraint" only appears via a tangential match.
        let catalog = fixture();
        let lookup = KeywordLookup::new();
        let results = lookup.suggest("constraint", &catalog, 10).await.unwrap();
        assert_eq!(results[0].descriptor_id, "cpsat");
    }

    // --- ChatBackendLookup with mock ------------------------------------

    /// Mock backend returning canned content based on the last query
    /// substring observed. Records every request for assertion.
    struct MockBackend {
        canned: HashMap<&'static str, String>,
        recorded: Mutex<Vec<ChatRequest>>,
    }

    impl MockBackend {
        fn new(canned: HashMap<&'static str, String>) -> Self {
            Self {
                canned,
                recorded: Mutex::new(Vec::new()),
            }
        }
    }

    impl DynChatBackend for MockBackend {
        fn chat(&self, req: ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>> {
            let user_content = req
                .messages
                .iter()
                .find(|m| matches!(m.role, ChatRole::User))
                .map(|m| m.content.clone())
                .unwrap_or_default();
            self.recorded.lock().unwrap().push(req);

            let content = self
                .canned
                .iter()
                .find_map(|(needle, body)| {
                    if user_content.to_lowercase().contains(&needle.to_lowercase()) {
                        Some(body.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "[]".to_string());

            Box::pin(async move {
                Ok(ChatResponse {
                    content,
                    tool_calls: Vec::new(),
                    usage: Some(TokenUsage::default()),
                    model: None,
                    finish_reason: Some(FinishReason::Stop),
                    metadata: HashMap::new(),
                })
            })
        }
    }

    #[tokio::test]
    async fn chat_backend_lookup_returns_parsed_suggestions() {
        let canned = HashMap::from([(
            "vendor",
            r#"[
                {"descriptor_id":"gleif","score":0.9,"rationale":"Verifies legal entity."},
                {"descriptor_id":"ofac","score":0.6,"rationale":"Sanctions screen."}
            ]"#
            .to_string(),
        )]);
        let backend: Arc<dyn DynChatBackend> = Arc::new(MockBackend::new(canned));
        let catalog = fixture();
        let lookup = ChatBackendLookup::new(backend);

        let results = lookup.suggest("verify vendor", &catalog, 5).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].descriptor_id, "gleif");
        assert!((results[0].score - 0.9).abs() < 1e-5);
        assert!(results[0].rationale.is_some());
    }

    #[tokio::test]
    async fn chat_backend_lookup_drops_unknown_ids() {
        let canned = HashMap::from([(
            "anything",
            r#"[
                {"descriptor_id":"gleif","score":0.9},
                {"descriptor_id":"does-not-exist","score":0.8}
            ]"#
            .to_string(),
        )]);
        let backend: Arc<dyn DynChatBackend> = Arc::new(MockBackend::new(canned));
        let catalog = fixture();
        let lookup = ChatBackendLookup::new(backend);

        let results = lookup.suggest("anything", &catalog, 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].descriptor_id, "gleif");
    }

    #[tokio::test]
    async fn chat_backend_lookup_returns_empty_for_empty_catalog() {
        let canned = HashMap::new();
        let backend: Arc<dyn DynChatBackend> = Arc::new(MockBackend::new(canned));
        let catalog = DiscoveryCatalog::new();
        let lookup = ChatBackendLookup::new(backend);

        let results = lookup.suggest("anything", &catalog, 5).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn chat_backend_lookup_errors_on_unparseable_output() {
        let canned = HashMap::from([("foo", "not json at all".to_string())]);
        let backend: Arc<dyn DynChatBackend> = Arc::new(MockBackend::new(canned));
        let catalog = fixture();
        let lookup = ChatBackendLookup::new(backend);

        let err = lookup.suggest("foo", &catalog, 5).await.unwrap_err();
        assert!(matches!(err, LookupError::UnparseableOutput(_)));
    }

    #[tokio::test]
    async fn chat_backend_lookup_errors_when_no_valid_ids() {
        let canned = HashMap::from([(
            "foo",
            r#"[{"descriptor_id":"ghost","score":0.5}]"#.to_string(),
        )]);
        let backend: Arc<dyn DynChatBackend> = Arc::new(MockBackend::new(canned));
        let catalog = fixture();
        let lookup = ChatBackendLookup::new(backend);

        let err = lookup.suggest("foo", &catalog, 5).await.unwrap_err();
        assert!(matches!(err, LookupError::NoValidIds));
    }

    #[tokio::test]
    async fn chat_backend_lookup_score_clamped_to_unit_interval() {
        let canned = HashMap::from([(
            "foo",
            r#"[{"descriptor_id":"gleif","score":5.0},{"descriptor_id":"ofac","score":-0.5}]"#
                .to_string(),
        )]);
        let backend: Arc<dyn DynChatBackend> = Arc::new(MockBackend::new(canned));
        let catalog = fixture();
        let lookup = ChatBackendLookup::new(backend);

        let results = lookup.suggest("foo", &catalog, 5).await.unwrap();
        for r in &results {
            assert!(
                (0.0..=1.0).contains(&r.score),
                "score out of range: {}",
                r.score
            );
        }
    }
}
