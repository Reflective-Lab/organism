// Copyright 2024-2026 Aprio One AB, Sweden
// SPDX-License-Identifier: LicenseRef-Proprietary

//! Dataset-backed research agents for Spike 3.

use std::collections::BTreeMap;
use std::sync::Arc;

use converge_core::{Agent, AgentEffect, Context, ContextKey, Fact};
use converge_provider::brave::{BraveSearchProvider, BraveSearchRequest};
use converge_provider::provider_api::{LlmProvider, LlmRequest};

use crate::spike_capacity::scenario::{
    DeliveryHistoryRecord, ExperimentTopic, TeamCapacityRecord, load_capacity_bundle,
};

pub struct ExperimentDataLoadAgent {
    topic: ExperimentTopic,
}

impl ExperimentDataLoadAgent {
    pub fn new(topic: ExperimentTopic) -> Self {
        Self { topic }
    }

    fn output_id(&self) -> String {
        format!("dataset:{}", self.topic.name())
    }
}

impl Agent for ExperimentDataLoadAgent {
    fn name(&self) -> &str {
        "ExperimentDataLoadAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        !ctx.get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == self.output_id())
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let bundle = match load_capacity_bundle() {
            Ok(bundle) => bundle,
            Err(err) => {
                return AgentEffect::with_fact(Fact::new(
                    ContextKey::Diagnostic,
                    format!("dataset-error:{}", self.topic.name()),
                    err,
                ));
            }
        };

        let content = match self.topic {
            ExperimentTopic::DemandResearch => serde_json::json!({
                "dataset_version": bundle.demand.dataset_version,
                "records": bundle.demand.records,
            }),
            ExperimentTopic::DeliveryDeepResearch => serde_json::json!({
                "dataset_version": bundle.history.dataset_version,
                "records": bundle.history.records,
            }),
            ExperimentTopic::WorkforceResearch => serde_json::json!({
                "dataset_version": bundle.capacity.dataset_version,
                "records": bundle.capacity.records,
            }),
        };

        AgentEffect::with_fact(Fact::new(
            ContextKey::Signals,
            self.output_id(),
            content.to_string(),
        ))
    }
}

pub struct ResearchAnalysisAgent {
    topic: ExperimentTopic,
}

impl ResearchAnalysisAgent {
    pub fn new(topic: ExperimentTopic) -> Self {
        Self { topic }
    }

    fn input_id(&self) -> String {
        format!("dataset:{}", self.topic.name())
    }

    fn output_id(&self) -> String {
        format!("analysis:{}", self.topic.name())
    }
}

impl Agent for ResearchAnalysisAgent {
    fn name(&self) -> &str {
        "ResearchAnalysisAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_dataset = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == self.input_id());
        let has_analysis = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .any(|f| f.id == self.output_id());
        has_dataset && !has_analysis
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let dataset = match ctx
            .get(ContextKey::Signals)
            .iter()
            .find(|f| f.id == self.input_id())
        {
            Some(dataset) => dataset,
            None => return AgentEffect::empty(),
        };

        let content = match self.topic {
            ExperimentTopic::DemandResearch => analyze_demand(&dataset.content),
            ExperimentTopic::DeliveryDeepResearch => analyze_delivery(&dataset.content),
            ExperimentTopic::WorkforceResearch => analyze_workforce(&dataset.content),
        };

        AgentEffect::with_fact(Fact::new(ContextKey::Hypotheses, self.output_id(), content))
    }
}

pub struct SearchPlannerAgent {
    topic: ExperimentTopic,
    research_question: String,
    llm: Arc<dyn LlmProvider>,
}

impl SearchPlannerAgent {
    pub fn new(
        topic: ExperimentTopic,
        research_question: String,
        llm: Arc<dyn LlmProvider>,
    ) -> Self {
        Self {
            topic,
            research_question,
            llm,
        }
    }

    fn output_id(&self) -> String {
        format!("search_queries:{}", self.topic.name())
    }
}

impl Agent for SearchPlannerAgent {
    fn name(&self) -> &str {
        "SearchPlannerAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_dataset = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == format!("dataset:{}", self.topic.name()));
        let has_queries = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id == self.output_id());
        has_dataset && !has_queries
    }

    fn execute(&self, _ctx: &Context) -> AgentEffect {
        let system = "You are preparing web research for a capacity planning huddle. \
            Generate exactly 4 search queries that would improve planning quality. \
            Return one query per line and nothing else.";
        let request = LlmRequest::new(&self.research_question)
            .with_system(system)
            .with_max_tokens(256)
            .with_temperature(0.2);

        match self.llm.complete(&request) {
            Ok(response) => {
                let queries: Vec<String> = response
                    .content
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .take(4)
                    .map(ToString::to_string)
                    .collect();

                AgentEffect::with_fact(Fact::new(
                    ContextKey::Seeds,
                    self.output_id(),
                    serde_json::json!({
                        "topic": self.topic.name(),
                        "queries": queries,
                    })
                    .to_string(),
                ))
            }
            Err(err) => AgentEffect::with_fact(Fact::new(
                ContextKey::Diagnostic,
                format!("search_planner_error:{}", self.topic.name()),
                err.to_string(),
            )),
        }
    }
}

pub struct WebSearchAgent {
    topic: ExperimentTopic,
    search: Arc<BraveSearchProvider>,
}

impl WebSearchAgent {
    pub fn new(topic: ExperimentTopic, search: Arc<BraveSearchProvider>) -> Self {
        Self { topic, search }
    }

    fn input_id(&self) -> String {
        format!("search_queries:{}", self.topic.name())
    }

    fn output_id(&self) -> String {
        format!("search_results:{}", self.topic.name())
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
        let queries_fact = match ctx
            .get(ContextKey::Seeds)
            .iter()
            .find(|f| f.id == self.input_id())
        {
            Some(fact) => fact,
            None => return AgentEffect::empty(),
        };

        let queries: Vec<String> = serde_json::from_str::<serde_json::Value>(&queries_fact.content)
            .ok()
            .and_then(|v| v.get("queries")?.as_array().cloned())
            .map(|arr| {
                arr.into_iter()
                    .filter_map(|value| value.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let mut results = Vec::new();
        for query in &queries {
            let request = BraveSearchRequest::new(query).with_count(5);
            match self.search.search(&request) {
                Ok(response) => {
                    results.push(serde_json::json!({
                        "query": query,
                        "results": BraveSearchProvider::format_for_llm(&response, 5),
                    }));
                }
                Err(err) => {
                    results.push(serde_json::json!({
                        "query": query,
                        "error": err.to_string(),
                    }));
                }
            }
        }

        AgentEffect::with_fact(Fact::new(
            ContextKey::Signals,
            self.output_id(),
            serde_json::json!({
                "topic": self.topic.name(),
                "search_results": results,
            })
            .to_string(),
        ))
    }
}

pub struct HybridResearchAnalysisAgent {
    topic: ExperimentTopic,
    llm: Arc<dyn LlmProvider>,
}

impl HybridResearchAnalysisAgent {
    pub fn new(topic: ExperimentTopic, llm: Arc<dyn LlmProvider>) -> Self {
        Self { topic, llm }
    }

    fn dataset_id(&self) -> String {
        format!("dataset:{}", self.topic.name())
    }

    fn search_id(&self) -> String {
        format!("search_results:{}", self.topic.name())
    }

    fn output_id(&self) -> String {
        format!("analysis:{}", self.topic.name())
    }
}

impl Agent for HybridResearchAnalysisAgent {
    fn name(&self) -> &str {
        "HybridResearchAnalysisAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &Context) -> bool {
        let has_dataset = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == self.dataset_id());
        let has_search = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|f| f.id == self.search_id());
        let has_analysis = ctx
            .get(ContextKey::Hypotheses)
            .iter()
            .any(|f| f.id == self.output_id());
        has_dataset && has_search && !has_analysis
    }

    fn execute(&self, ctx: &Context) -> AgentEffect {
        let dataset = match ctx
            .get(ContextKey::Signals)
            .iter()
            .find(|f| f.id == self.dataset_id())
        {
            Some(dataset) => dataset,
            None => return AgentEffect::empty(),
        };
        let search = match ctx
            .get(ContextKey::Signals)
            .iter()
            .find(|f| f.id == self.search_id())
        {
            Some(search) => search,
            None => return AgentEffect::empty(),
        };

        let baseline = build_local_analysis(self.topic, &dataset.content);
        let system = format!(
            "You are a capacity planning analyst synthesizing local datasets with live web research.\n\
             Return raw JSON only with this structure:\n\
             {{\"dataset_version\":\"...\",\"summary\":\"...\",\"external_signal\":\"...\",\"recommended_watchouts\":[\"...\",\"...\"],\"source_count\":4}}\n\
             Keep the summary concise and business-relevant for a planning huddle. Baseline dataset analysis:\n{baseline}"
        );

        let prompt = format!(
            "Dataset payload:\n{}\n\nWeb research payload:\n{}\n\nTopic:{}",
            dataset.content,
            search.content,
            self.topic.name()
        );
        let request = LlmRequest::new(prompt)
            .with_system(system)
            .with_max_tokens(600)
            .with_temperature(0.2);

        let content = match self.llm.complete(&request) {
            Ok(response) => {
                let cleaned = strip_markdown_fences(&response.content);
                if serde_json::from_str::<serde_json::Value>(&cleaned).is_ok() {
                    cleaned
                } else {
                    serde_json::json!({
                        "dataset_version": dataset_version(&dataset.content),
                        "summary": summary_from_analysis(&baseline),
                        "external_signal": "LLM response could not be parsed cleanly; using baseline analysis with live research attached separately.",
                        "recommended_watchouts": ["Validate live signals before commitment"],
                        "source_count": count_sources(&search.content),
                    })
                    .to_string()
                }
            }
            Err(_) => serde_json::json!({
                "dataset_version": dataset_version(&dataset.content),
                "summary": summary_from_analysis(&baseline),
                "external_signal": "Live provider failed; falling back to dataset-only analysis.",
                "recommended_watchouts": ["Re-run with healthy provider before commitment"],
                "source_count": count_sources(&search.content),
            })
            .to_string(),
        };

        AgentEffect::with_fact(Fact::new(ContextKey::Hypotheses, self.output_id(), content))
    }
}

fn analyze_demand(content: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(content).unwrap_or_default();
    let records = parsed
        .get("records")
        .and_then(|v| serde_json::from_value::<Vec<serde_json::Value>>(v.clone()).ok())
        .unwrap_or_default();

    let mut demand_by_skill = BTreeMap::<String, f64>::new();
    let mut confidence_sum = 0.0;

    for record in records {
        let skill = record
            .get("skill")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let units = record
            .get("demand_units")
            .and_then(|v| v.as_f64())
            .unwrap_or_default();
        let confidence = record
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or_default();
        *demand_by_skill.entry(skill.to_string()).or_default() += units;
        confidence_sum += confidence;
    }

    let top_skill = demand_by_skill
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(skill, _)| skill.clone())
        .unwrap_or_default();

    serde_json::json!({
        "dataset_version": parsed.get("dataset_version").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "summary": format!(
            "Forecast demand is concentrated in backend and data work. The heaviest skill pressure is on {top_skill}, and the average forecast confidence across the horizon is {:.0}%.",
            confidence_sum * 100.0 / 10.0
        ),
        "demand_by_skill": demand_by_skill,
        "top_skill": top_skill,
    })
    .to_string()
}

fn build_local_analysis(topic: ExperimentTopic, content: &str) -> String {
    match topic {
        ExperimentTopic::DemandResearch => analyze_demand(content),
        ExperimentTopic::DeliveryDeepResearch => analyze_delivery(content),
        ExperimentTopic::WorkforceResearch => analyze_workforce(content),
    }
}

fn summary_from_analysis(content: &str) -> String {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("summary")
                .and_then(|summary| summary.as_str().map(ToString::to_string))
        })
        .unwrap_or_else(|| content.to_string())
}

fn dataset_version(content: &str) -> String {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("dataset_version")
                .and_then(|version| version.as_str().map(ToString::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn count_sources(content: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(content)
        .ok()
        .and_then(|v| {
            v.get("search_results")
                .and_then(|s| s.as_array().map(Vec::len))
        })
        .unwrap_or(0)
}

fn strip_markdown_fences(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        trimmed.to_string()
    }
}

fn analyze_delivery(content: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(content).unwrap_or_default();
    let records: Vec<DeliveryHistoryRecord> = parsed
        .get("records")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let highest_risk = records
        .iter()
        .max_by(|a, b| {
            (a.lateness_rate + a.spillover_rate)
                .partial_cmp(&(b.lateness_rate + b.spillover_rate))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();

    let avg_lateness = records.iter().map(|r| r.lateness_rate).sum::<f64>() / records.len() as f64;

    serde_json::json!({
        "dataset_version": parsed.get("dataset_version").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "summary": format!(
            "Deep delivery research shows persistent lateness concentrated in {}. Average lateness is {:.0}%, with spillover indicating planning debt instead of isolated execution misses.",
            highest_risk.as_ref().map_or("unknown", |r| r.skill.as_str()),
            avg_lateness * 100.0
        ),
        "risk_by_skill": records.iter().map(|r| (r.skill.clone(), r.lateness_rate + r.spillover_rate)).collect::<BTreeMap<_, _>>(),
        "highest_risk_skill": highest_risk.map(|r| r.skill).unwrap_or_default(),
    })
    .to_string()
}

fn analyze_workforce(content: &str) -> String {
    let parsed: serde_json::Value = serde_json::from_str(content).unwrap_or_default();
    let records: Vec<TeamCapacityRecord> = parsed
        .get("records")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut capacity_by_skill = BTreeMap::<String, f64>::new();
    for team in &records {
        let per_skill = team.available_capacity / team.skills.len() as f64;
        for skill in &team.skills {
            *capacity_by_skill.entry(skill.clone()).or_default() += per_skill;
        }
    }

    let tightest_skill = capacity_by_skill
        .iter()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(skill, _)| skill.clone())
        .unwrap_or_default();

    serde_json::json!({
        "dataset_version": parsed.get("dataset_version").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "summary": format!(
            "Current workforce capacity is concentrated in backend/platform while {} remains structurally thin. Existing teams can absorb some demand, but not the full forecast without additional capacity shaping.",
            tightest_skill
        ),
        "capacity_by_skill": capacity_by_skill,
        "tightest_skill": tightest_skill,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demand_analysis_has_summary() {
        let bundle = load_capacity_bundle().unwrap();
        let content = serde_json::json!({
            "dataset_version": bundle.demand.dataset_version,
            "records": bundle.demand.records,
        })
        .to_string();
        let analysis = analyze_demand(&content);
        assert!(analysis.contains("Forecast demand"));
    }
}
