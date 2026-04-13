// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Experiment agents for the Nordic Market Expansion spike.
//!
//! Each experiment uses 3 agents:
//! 1. SearchPlannerAgent — generates targeted search queries (LLM)
//! 2. WebSearchAgent — executes searches via Brave API
//! 3. ResearchAnalystAgent — analyzes results, scores cities (LLM)

use std::sync::Arc;

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};
use converge_provider::brave::{BraveSearchProvider, BraveSearchRequest};
use converge_provider::provider_api::{LlmProvider, LlmRequest};

use crate::spike_market::scenario::candidate_cities;

// ── SearchPlannerAgent ──────────────────────────────────────────────

/// Generates 3 targeted search queries from the research question using an LLM.
pub struct SearchPlannerAgent {
    topic: String,
    research_question: String,
    llm: Arc<dyn LlmProvider>,
}

impl SearchPlannerAgent {
    pub fn new(topic: String, research_question: String, llm: Arc<dyn LlmProvider>) -> Self {
        Self {
            topic,
            research_question,
            llm,
        }
    }

    fn seed_id(&self) -> String {
        format!("search_queries:{}", self.topic)
    }
}

impl Agent for SearchPlannerAgent {
    fn name(&self) -> &str {
        "SearchPlannerAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        !ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == self.seed_id())
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let system = "You are a research assistant. Generate exactly 3 web search queries \
            that would help answer the research question. Output one query per line, \
            nothing else. No numbering, no explanation.";

        let request = LlmRequest::new(&self.research_question)
            .with_system(system)
            .with_max_tokens(256)
            .with_temperature(0.3);

        match self.llm.complete(&request) {
            Ok(response) => {
                let queries: Vec<String> = response
                    .content
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .take(3)
                    .collect();

                let content = serde_json::json!({
                    "topic": self.topic,
                    "queries": queries,
                })
                .to_string();

                AgentEffect::with_fact(Fact::new(ContextKey::Seeds, self.seed_id(), content))
            }
            Err(e) => AgentEffect::with_fact(Fact::new(
                ContextKey::Diagnostic,
                format!("error:search_planner:{}", self.topic),
                format!("SearchPlannerAgent failed: {e}"),
            )),
        }
    }
}

// ── WebSearchAgent ──────────────────────────────────────────────────

/// Executes Brave web searches for each query, collects results.
pub struct WebSearchAgent {
    topic: String,
    search: Arc<BraveSearchProvider>,
}

impl WebSearchAgent {
    pub fn new(topic: String, search: Arc<BraveSearchProvider>) -> Self {
        Self { topic, search }
    }

    fn input_id(&self) -> String {
        format!("search_queries:{}", self.topic)
    }

    fn output_id(&self) -> String {
        format!("search_results:{}", self.topic)
    }
}

impl Agent for WebSearchAgent {
    fn name(&self) -> &str {
        "WebSearchAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_queries = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == self.input_id());
        let has_results = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == self.output_id());
        has_queries && !has_results
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let queries_fact = ctx
            .get(ContextKey::Seeds)
            .iter()
            .find(|f| f.id == self.input_id());

        let queries_fact = match queries_fact {
            Some(f) => f,
            None => return AgentEffect::empty(),
        };

        // Parse queries from the fact content
        let queries: Vec<String> = serde_json::from_str::<serde_json::Value>(&queries_fact.content)
            .ok()
            .and_then(|v| v.get("queries")?.as_array().cloned())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mut all_results = Vec::new();

        for query in &queries {
            let request = BraveSearchRequest::new(query).with_count(5);
            match self.search.search(&request) {
                Ok(response) => {
                    let formatted = BraveSearchProvider::format_for_llm(&response, 5);
                    all_results.push(serde_json::json!({
                        "query": query,
                        "results": formatted,
                    }));
                }
                Err(e) => {
                    all_results.push(serde_json::json!({
                        "query": query,
                        "error": format!("{e}"),
                    }));
                }
            }
        }

        let content = serde_json::json!({
            "topic": self.topic,
            "search_results": all_results,
        })
        .to_string();

        AgentEffect::with_fact(Fact::new(ContextKey::Signals, self.output_id(), content))
    }
}

// ── ResearchAnalystAgent ────────────────────────────────────────────

/// Analyzes search results and scores 8 candidate cities on the experiment's dimension.
pub struct ResearchAnalystAgent {
    topic: String,
    llm: Arc<dyn LlmProvider>,
}

impl ResearchAnalystAgent {
    pub fn new(topic: String, llm: Arc<dyn LlmProvider>) -> Self {
        Self { topic, llm }
    }

    fn input_id(&self) -> String {
        format!("search_results:{}", self.topic)
    }

    fn output_id(&self) -> String {
        format!("analysis:{}", self.topic)
    }
}

impl Agent for ResearchAnalystAgent {
    fn name(&self) -> &str {
        "ResearchAnalystAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_results = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == self.input_id());
        let has_analysis = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .any(|f| f.id == self.output_id());
        has_results && !has_analysis
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let results_fact = ctx
            .get(ContextKey::Signals)
            .iter()
            .find(|f| f.id == self.input_id());

        let results_fact = match results_fact {
            Some(f) => f,
            None => return AgentEffect::empty(),
        };

        let cities = candidate_cities();
        let city_names: Vec<&str> = cities.iter().map(|c| c.name.as_str()).collect();

        let system = format!(
            "You are a market research analyst. Analyze the search results and score each \
             candidate city on a scale of 0-100 for the research dimension '{topic}'.\n\n\
             You MUST respond with raw JSON only. Do NOT wrap it in ```json``` or any markdown. \
             Do NOT include any text before or after the JSON object.\n\n\
             Required format (exactly this structure):\n\
             {{\"scores\": {{\"Stockholm\": 85, \"Berlin\": 80, \"Amsterdam\": 78, \"London\": 90, \
             \"Helsinki\": 72, \"Copenhagen\": 76, \"Zurich\": 88, \"Dublin\": 74}}, \
             \"summary\": \"One paragraph analysis\"}}\n\n\
             You must score ALL of these cities: {cities}\n\
             Base your scores on the evidence from the search results. If data is sparse, \
             use reasonable estimates based on known facts about these cities.",
            topic = self.topic,
            cities = city_names.join(", ")
        );

        let request = LlmRequest::new(&results_fact.content)
            .with_system(&system)
            .with_max_tokens(1024)
            .with_temperature(0.3);

        match self.llm.complete(&request) {
            Ok(response) => {
                // Strip markdown fences — LLMs often wrap JSON in ```json...```
                let cleaned = strip_markdown_fences(&response.content);

                // Try to parse as JSON; if it fails, wrap the raw text
                let content = if serde_json::from_str::<serde_json::Value>(&cleaned).is_ok() {
                    cleaned
                } else {
                    serde_json::json!({
                        "raw_response": response.content,
                        "topic": self.topic,
                    })
                    .to_string()
                };

                AgentEffect::with_fact(Fact::new(ContextKey::Hypotheses, self.output_id(), content))
            }
            Err(e) => AgentEffect::with_fact(Fact::new(
                ContextKey::Diagnostic,
                format!("error:analyst:{}", self.topic),
                format!("ResearchAnalystAgent failed: {e}"),
            )),
        }
    }
}

/// Strip markdown code fences from LLM output.
///
/// LLMs frequently wrap JSON in ` ```json ... ``` ` blocks. This extracts
/// the inner content so `serde_json::from_str` can parse it.
fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();

    // Handle ```json ... ``` or ``` ... ```
    if trimmed.starts_with("```") {
        let after_opening = if let Some(first_newline) = trimmed.find('\n') {
            &trimmed[first_newline + 1..]
        } else {
            return trimmed.to_string();
        };
        if let Some(closing) = after_opening.rfind("```") {
            return after_opening[..closing].trim().to_string();
        }
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_planner_accepts_when_no_queries() {
        let agent = SearchPlannerAgent {
            topic: "test".into(),
            research_question: "test question".into(),
            llm: Arc::new(MockLlm("query1\nquery2\nquery3".into())),
        };
        let ctx = Context::new();
        assert!(agent.accepts(&ctx));
    }

    #[test]
    fn search_planner_rejects_when_queries_exist() {
        let agent = SearchPlannerAgent {
            topic: "test".into(),
            research_question: "test question".into(),
            llm: Arc::new(MockLlm("query1".into())),
        };
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Seeds,
            "search_queries:test",
            "{\"topic\":\"test\",\"queries\":[\"q1\"]}",
        ))
        .unwrap();
        assert!(!agent.accepts(&ctx));
    }

    #[test]
    fn search_planner_produces_queries() {
        let agent = SearchPlannerAgent {
            topic: "test".into(),
            research_question: "What is the tech talent market?".into(),
            llm: Arc::new(MockLlm(
                "tech talent Stockholm 2025\nR&D centers Europe comparison\nNordic tech ecosystem"
                    .into(),
            )),
        };
        let ctx = Context::new();
        let effect = agent.execute(&ctx);
        assert_eq!(effect.facts.len(), 1);
        assert_eq!(effect.facts[0].key, ContextKey::Seeds);
        let parsed: serde_json::Value = serde_json::from_str(&effect.facts[0].content).unwrap();
        assert_eq!(parsed["queries"].as_array().unwrap().len(), 3);
    }

    /// Mock LLM provider for testing.
    struct MockLlm(String);

    impl LlmProvider for MockLlm {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn model(&self) -> &str {
            "mock-model"
        }

        fn complete(
            &self,
            _request: &LlmRequest,
        ) -> Result<
            converge_provider::provider_api::LlmResponse,
            converge_provider::provider_api::LlmError,
        > {
            Ok(converge_provider::provider_api::LlmResponse {
                content: self.0.clone(),
                model: "mock".into(),
                usage: converge_provider::provider_api::TokenUsage::default(),
                finish_reason: converge_provider::provider_api::FinishReason::Stop,
            })
        }
    }
}
