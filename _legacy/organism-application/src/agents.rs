// Copyright 2024-2025 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! LLM-powered agents for the organism-application.
//!
//! This module contains agents that use LLM providers to generate
//! insights beyond what deterministic agents can produce.

use converge_core::traits::{
    ChatBackend, ChatMessage, ChatRequest, ChatResponse, ChatRole, DynChatBackend, FinishReason,
    LlmError, TokenUsage,
};
use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};
use std::fmt::Write;
use std::sync::Arc;

/// LLM-powered agent that generates strategic insights from evaluations.
///
/// This agent runs after the `EvaluationAgent` and synthesizes higher-level
/// insights by analyzing the full context through an LLM.
///
/// # Pipeline Position
///
/// ```text
/// Seeds → Signals → Competitors → Strategies → Evaluations
///                                                    │
///                                                    ▼
///                                          StrategicInsightAgent
///                                                    │
///                                                    ▼
///                                              Hypotheses (insights)
/// ```
pub struct StrategicInsightAgent {
    provider: Arc<dyn DynChatBackend>,
    system_prompt: String,
}

impl StrategicInsightAgent {
    /// Creates a new `StrategicInsightAgent` with the given chat backend.
    pub fn new(provider: Arc<dyn DynChatBackend>) -> Self {
        Self {
            provider,
            system_prompt: r"You are a strategic advisor analyzing growth strategies for a business.

Given the context of market signals, competitor analysis, proposed strategies, and their evaluations,
synthesize 2-3 key strategic insights that the business should consider.

Each insight should:
1. Be actionable and specific
2. Reference the data in the context
3. Provide a clear recommendation

Format your response as a numbered list of insights, one per line.
Keep each insight concise (1-2 sentences).".to_string(),
        }
    }

    /// Creates an agent with a custom system prompt.
    pub fn with_prompt(
        provider: Arc<dyn DynChatBackend>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            system_prompt: system_prompt.into(),
        }
    }

    /// Builds the user prompt from context.
    #[allow(clippy::unused_self)]
    fn build_prompt(&self, ctx: &Context) -> String {
        let mut prompt = String::new();

        prompt.push_str("## Market Signals\n");
        for fact in ctx.get(ContextKey::Signals) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Competitor Analysis\n");
        for fact in ctx.get(ContextKey::Competitors) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Proposed Strategies\n");
        for fact in ctx.get(ContextKey::Strategies) {
            let _ = writeln!(prompt, "- {}: {}", fact.id, fact.content);
        }

        prompt.push_str("\n## Evaluations\n");
        for fact in ctx.get(ContextKey::Evaluations) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Task\nProvide 2-3 strategic insights based on this analysis.");

        prompt
    }

    /// Parses LLM response into facts.
    #[allow(clippy::unused_self)]
    fn parse_response(&self, response: &str) -> Vec<Fact> {
        let mut facts = Vec::new();

        for (i, line) in response.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Strip leading numbers like "1.", "2.", etc.
            let content = line
                .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ' ')
                .trim();

            if !content.is_empty() && content.len() > 10 {
                facts.push(Fact {
                    key: ContextKey::Hypotheses,
                    id: format!("insight:{}", i + 1),
                    content: content.to_string(),
                });
            }
        }

        // Ensure we have at least one insight
        if facts.is_empty() {
            facts.push(Fact {
                key: ContextKey::Hypotheses,
                id: "insight:fallback".into(),
                content: "LLM analysis completed but no structured insights extracted. Review raw evaluation data.".into(),
            });
        }

        facts
    }
}

impl Agent for StrategicInsightAgent {
    fn name(&self) -> &'static str {
        "StrategicInsightAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run once when evaluations exist but no hypotheses (insights) yet
        ctx.has(ContextKey::Evaluations) && !ctx.has(ContextKey::Hypotheses)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let prompt = self.build_prompt(ctx);

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: self.system_prompt.clone(),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: prompt,
                },
            ],
            max_tokens: Some(1024),
            temperature: Some(0.7),
            model: None,
        };

        // Call LLM using block_in_place + block_on for async ChatBackend
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.provider.chat(request))
        });

        match result {
            Ok(response) => {
                let facts = self.parse_response(&response.content);
                AgentEffect::with_facts(facts)
            }
            Err(e) => AgentEffect::with_facts(vec![Fact {
                key: ContextKey::Hypotheses,
                id: "insight:error".into(),
                content: format!("LLM call failed: {e}. Manual review recommended."),
            }]),
        }
    }
}

/// A simple mock chat backend for testing without API keys.
pub struct MockInsightProvider {
    response: String,
}

impl MockInsightProvider {
    /// Creates a mock provider with a predefined response.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }

    /// Creates a mock provider with default insights.
    pub fn default_insights() -> Self {
        Self::new(
            r"1. Focus on the LinkedIn B2B campaign as your primary channel - it scores highest and aligns with market signals showing LinkedIn effectiveness for B2B.

2. Invest in self-service demo capabilities as a secondary priority - while it requires development investment, it directly addresses the buyer preference for self-service identified in market signals.

3. Consider a phased approach: launch LinkedIn campaign immediately for quick wins, then build self-service demo experience for long-term competitive advantage.",
        )
    }
}

impl ChatBackend for MockInsightProvider {
    type ChatFut<'a>
        = std::future::Ready<Result<ChatResponse, LlmError>>
    where
        Self: 'a;

    fn chat<'a>(&'a self, _req: ChatRequest) -> Self::ChatFut<'a> {
        std::future::ready(Ok(ChatResponse {
            content: self.response.clone(),
            model: Some("mock-insight-v1".into()),
            usage: Some(TokenUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            }),
            finish_reason: Some(FinishReason::Stop),
        }))
    }
}

// =============================================================================
// RISK ASSESSMENT AGENT
// =============================================================================

/// LLM-powered agent that identifies risks and challenges for proposed strategies.
///
/// This agent analyzes strategies and their evaluations to identify potential
/// risks, challenges, and mitigation recommendations.
///
/// # Pipeline Position
///
/// ```text
/// Seeds → Signals → Competitors → Strategies → Evaluations
///                                                    │
///                                    ┌───────────────┼───────────────┐
///                                    ▼               ▼               ▼
///                          StrategicInsightAgent  RiskAssessmentAgent
///                                    │               │
///                                    ▼               ▼
///                              Hypotheses      Constraints (risks)
/// ```
pub struct RiskAssessmentAgent {
    provider: Arc<dyn DynChatBackend>,
    system_prompt: String,
}

impl RiskAssessmentAgent {
    /// Creates a new `RiskAssessmentAgent` with the given chat backend.
    pub fn new(provider: Arc<dyn DynChatBackend>) -> Self {
        Self {
            provider,
            system_prompt: r"You are a risk analyst evaluating business strategies.

Given the proposed strategies and their evaluations, identify 2-3 key risks or challenges
that could impact successful execution.

For each risk:
1. Name the risk clearly
2. Explain what could go wrong
3. Suggest a mitigation approach

Format your response as a numbered list, one risk per item.
Keep each risk assessment concise (2-3 sentences)."
                .to_string(),
        }
    }

    /// Creates an agent with a custom system prompt.
    pub fn with_prompt(
        provider: Arc<dyn DynChatBackend>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            system_prompt: system_prompt.into(),
        }
    }

    /// Builds the user prompt from context.
    #[allow(clippy::unused_self)]
    fn build_prompt(&self, ctx: &Context) -> String {
        let mut prompt = String::new();

        prompt.push_str("## Company Context\n");
        for fact in ctx.get(ContextKey::Seeds) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Market Signals\n");
        for fact in ctx.get(ContextKey::Signals) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Competitive Landscape\n");
        for fact in ctx.get(ContextKey::Competitors) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Proposed Strategies\n");
        for fact in ctx.get(ContextKey::Strategies) {
            let _ = writeln!(prompt, "- {}: {}", fact.id, fact.content);
        }

        prompt.push_str("\n## Strategy Evaluations\n");
        for fact in ctx.get(ContextKey::Evaluations) {
            let _ = writeln!(prompt, "- {}", fact.content);
        }

        prompt.push_str("\n## Task\nIdentify 2-3 key risks or challenges for these strategies and suggest mitigations.");

        prompt
    }

    /// Parses LLM response into risk facts.
    #[allow(clippy::unused_self)]
    fn parse_response(&self, response: &str) -> Vec<Fact> {
        let mut facts = Vec::new();
        let mut risk_count = 0;

        for line in response.lines() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Strip leading numbers like "1.", "2.", etc.
            let content = line
                .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ')' || c == ' ')
                .trim();

            if !content.is_empty() && content.len() > 20 {
                risk_count += 1;
                facts.push(Fact {
                    key: ContextKey::Constraints,
                    id: format!("risk:{risk_count}"),
                    content: content.to_string(),
                });
            }
        }

        // Ensure we have at least one risk identified
        if facts.is_empty() {
            facts.push(Fact {
                key: ContextKey::Constraints,
                id: "risk:none-identified".into(),
                content: "No significant risks identified. Recommend manual review of assumptions."
                    .into(),
            });
        }

        facts
    }
}

impl Agent for RiskAssessmentAgent {
    fn name(&self) -> &'static str {
        "RiskAssessmentAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        // Run once when strategies and evaluations exist but no constraints (risks) yet
        ctx.has(ContextKey::Strategies)
            && ctx.has(ContextKey::Evaluations)
            && !ctx.has(ContextKey::Constraints)
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let prompt = self.build_prompt(ctx);

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::System,
                    content: self.system_prompt.clone(),
                },
                ChatMessage {
                    role: ChatRole::User,
                    content: prompt,
                },
            ],
            max_tokens: Some(1024),
            temperature: Some(0.7),
            model: None,
        };

        // Call LLM using block_in_place + block_on for async ChatBackend
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.provider.chat(request))
        });

        match result {
            Ok(response) => {
                let facts = self.parse_response(&response.content);
                AgentEffect::with_facts(facts)
            }
            Err(e) => AgentEffect::with_facts(vec![Fact {
                key: ContextKey::Constraints,
                id: "risk:error".into(),
                content: format!("Risk assessment failed: {e}. Manual review recommended."),
            }]),
        }
    }
}

/// A mock chat backend for risk assessment testing.
pub struct MockRiskProvider {
    response: String,
}

impl MockRiskProvider {
    /// Creates a mock provider with a predefined response.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
        }
    }

    /// Creates a mock provider with default risk assessments.
    pub fn default_risks() -> Self {
        Self::new(
            r"1. **Resource Constraint Risk** - The self-service demo requires significant development investment while the team may be focused on the LinkedIn campaign. Mitigation: Phase the initiatives and allocate dedicated resources for each.

2. **Market Timing Risk** - The unclear competitive landscape means competitors could launch similar initiatives first. Mitigation: Conduct rapid competitor analysis within 2 weeks before committing to campaign messaging.

3. **Channel Saturation Risk** - LinkedIn B2B campaigns face increasing competition and rising costs. Mitigation: Test multiple audience segments with small budgets before scaling spend.",
        )
    }
}

impl ChatBackend for MockRiskProvider {
    type ChatFut<'a>
        = std::future::Ready<Result<ChatResponse, LlmError>>
    where
        Self: 'a;

    fn chat<'a>(&'a self, _req: ChatRequest) -> Self::ChatFut<'a> {
        std::future::ready(Ok(ChatResponse {
            content: self.response.clone(),
            model: Some("mock-risk-v1".into()),
            usage: Some(TokenUsage {
                prompt_tokens: 120,
                completion_tokens: 80,
                total_tokens: 200,
            }),
            finish_reason: Some(FinishReason::Stop),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn strategic_insight_agent_parses_numbered_list() {
        let provider = Arc::new(MockInsightProvider::default_insights());
        let agent = StrategicInsightAgent::new(provider);

        // Create a context with evaluations
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "eval:test",
            "Score: 80/100",
        ))
        .unwrap();

        assert!(agent.accepts(&ctx));

        let effect = agent.execute(&ctx);

        assert!(!effect.facts.is_empty());
        assert!(effect.facts.iter().any(|f| f.id.starts_with("insight:")));
    }

    #[test]
    fn strategic_insight_agent_runs_once() {
        let provider = Arc::new(MockInsightProvider::default_insights());
        let agent = StrategicInsightAgent::new(provider);

        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "eval:test",
            "Score: 80/100",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Hypotheses,
            "insight:1",
            "Existing insight",
        ))
        .unwrap();

        // Should not accept because Hypotheses already exist
        assert!(!agent.accepts(&ctx));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn risk_assessment_agent_identifies_risks() {
        let provider = Arc::new(MockRiskProvider::default_risks());
        let agent = RiskAssessmentAgent::new(provider);

        // Create a context with strategies and evaluations
        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strategy:test",
            "Test strategy",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "eval:test",
            "Score: 75/100",
        ))
        .unwrap();

        assert!(agent.accepts(&ctx));

        let effect = agent.execute(&ctx);

        assert!(!effect.facts.is_empty());
        assert!(effect.facts.iter().any(|f| f.id.starts_with("risk:")));
        assert!(
            effect
                .facts
                .iter()
                .all(|f| f.key == ContextKey::Constraints)
        );
    }

    #[test]
    fn risk_assessment_agent_runs_once() {
        let provider = Arc::new(MockRiskProvider::default_risks());
        let agent = RiskAssessmentAgent::new(provider);

        let mut ctx = Context::new();
        ctx.add_fact(Fact::new(
            ContextKey::Strategies,
            "strategy:test",
            "Test strategy",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Evaluations,
            "eval:test",
            "Score: 75/100",
        ))
        .unwrap();
        ctx.add_fact(Fact::new(
            ContextKey::Constraints,
            "risk:1",
            "Existing risk",
        ))
        .unwrap();

        // Should not accept because Constraints (risks) already exist
        assert!(!agent.accepts(&ctx));
    }
}
