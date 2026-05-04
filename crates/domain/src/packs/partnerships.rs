//! Partnerships pack — Vendor sourcing, evaluation, contracting.
//!
//! Fact prefixes: `partner:`, `supplier:`, `p_agreement:`, `vendor_assessment:`,
//! `integration:`, `diligence:`, `relationship:`, `contract_renewal:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "partner_sourcer",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "partner:",
        target_key: ContextKey::Proposals,
        description: "Identifies partner prospects",
    },
    AgentMeta {
        name: "vendor_assessor",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "vendor_assessment:",
        target_key: ContextKey::Proposals,
        description: "Security/compliance assessments",
    },
    AgentMeta {
        name: "contract_negotiator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "p_agreement:",
        target_key: ContextKey::Evaluations,
        description: "Negotiation support",
    },
    AgentMeta {
        name: "relationship_manager",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "relationship:",
        target_key: ContextKey::Evaluations,
        description: "Health monitoring",
    },
    AgentMeta {
        name: "performance_reviewer",
        dependencies: &[ContextKey::Evaluations],
        fact_prefix: "relationship:",
        target_key: ContextKey::Evaluations,
        description: "Annual reviews",
    },
    AgentMeta {
        name: "integration_coordinator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "integration:",
        target_key: ContextKey::Proposals,
        description: "Technical coordination",
    },
    AgentMeta {
        name: "due_diligence_coordinator",
        dependencies: &[ContextKey::Seeds],
        fact_prefix: "diligence:",
        target_key: ContextKey::Proposals,
        description: "Due diligence checklist",
    },
    AgentMeta {
        name: "partnership_renewal_tracker",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "contract_renewal:",
        target_key: ContextKey::Signals,
        description: "Renewal tracking",
    },
    AgentMeta {
        name: "risk_monitor",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "relationship:",
        target_key: ContextKey::Evaluations,
        description: "External risk detection",
    },
    AgentMeta {
        name: "offboarding_coordinator",
        dependencies: &[ContextKey::Proposals],
        fact_prefix: "partner:",
        target_key: ContextKey::Proposals,
        description: "Exit planning",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "vendor_has_assessment",
        class: InvariantClass::Structural,
        description: "Vendors must have assessment",
    },
    InvariantMeta {
        name: "partner_has_agreement",
        class: InvariantClass::Structural,
        description: "Partners must have agreement",
    },
    InvariantMeta {
        name: "integration_has_owner",
        class: InvariantClass::Structural,
        description: "Integrations must have owner",
    },
    InvariantMeta {
        name: "high_risk_vendor_requires_approval",
        class: InvariantClass::Semantic,
        description: "High-risk vendors require approval",
    },
];

pub const PROFILE: crate::pack::PackProfile = crate::pack::PackProfile {
    entities: &["partner", "supplier", "vendor", "integration", "assessment"],
    required_capabilities: &["web"],
    uses_llm: false,
    requires_hitl: true,
    handles_irreversible: false,
    keywords: &[
        "vendor",
        "partner",
        "supplier",
        "sourcing",
        "procurement",
        "assessment",
        "diligence",
    ],
};

/// Parses vendor candidates from seed data into individual vendor signals.
///
/// Maps to `vendor_assessor` metadata. This is reusable pack behavior; apps
/// provide the RFP seed and Converge handles promotion.
pub struct VendorDataSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for VendorDataSuggestor {
    fn name(&self) -> &'static str {
        "vendor_data"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Seeds) && !ctx.has(converge_pack::ContextKey::Signals)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        let seeds = ctx.get(converge_pack::ContextKey::Seeds);
        let Some(seed) = seeds.first() else {
            return converge_pack::AgentEffect::empty();
        };

        let json: serde_json::Value = serde_json::from_str(&seed.content).unwrap_or_default();
        let vendors = json.get("vendors").cloned().unwrap_or_default();

        let facts: Vec<converge_pack::ProposedFact> = vendors
            .as_array()
            .map_or(&[] as &[serde_json::Value], |v| v)
            .iter()
            .map(|vendor| {
                let id = vendor
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("?");
                converge_pack::ProposedFact::new(
                    converge_pack::ContextKey::Signals,
                    format!("vendor:{id}"),
                    vendor.to_string(),
                    self.name(),
                )
                .with_confidence(1.0)
            })
            .collect();

        converge_pack::AgentEffect::with_proposals(facts)
    }
}

/// Scores vendors by price tier.
///
/// Maps to `contract_negotiator` for the price dimension.
pub struct VendorPriceEvaluatorSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for VendorPriceEvaluatorSuggestor {
    fn name(&self) -> &'static str {
        "price_evaluator"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Signals)
            && !ctx.has(converge_pack::ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        evaluate_vendors(ctx, "price", |vendor| {
            let price = vendor
                .get("price")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(999_999.0);
            if price < 10_000.0 {
                1.0
            } else if price < 25_000.0 {
                0.7
            } else if price < 50_000.0 {
                0.4
            } else {
                0.1
            }
        })
    }
}

/// Scores vendors by compliance status.
///
/// Maps to `vendor_assessor` for the compliance dimension.
pub struct VendorComplianceEvaluatorSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for VendorComplianceEvaluatorSuggestor {
    fn name(&self) -> &'static str {
        "compliance_evaluator"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Signals)
            && !ctx.has(converge_pack::ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        evaluate_vendors(ctx, "compliance", |vendor| {
            if vendor
                .get("compliant")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
            {
                1.0
            } else {
                0.0
            }
        })
    }
}

/// Scores vendors by years in business as a simple risk proxy.
///
/// Maps to `risk_monitor`.
pub struct VendorRiskEvaluatorSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for VendorRiskEvaluatorSuggestor {
    fn name(&self) -> &'static str {
        "risk_evaluator"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Signals)
            && !ctx.has(converge_pack::ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        evaluate_vendors(ctx, "risk", |vendor| {
            let years = vendor
                .get("years_in_business")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if years > 10 {
                1.0
            } else if years > 5 {
                0.7
            } else if years > 2 {
                0.4
            } else {
                0.1
            }
        })
    }
}

/// Scores vendors by delivery timeline.
///
/// Maps to `performance_reviewer` for the timeline dimension.
pub struct VendorTimelineEvaluatorSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for VendorTimelineEvaluatorSuggestor {
    fn name(&self) -> &'static str {
        "timeline_evaluator"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Signals)
            && !ctx.has(converge_pack::ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        evaluate_vendors(ctx, "timeline", |vendor| {
            let weeks = vendor
                .get("delivery_weeks")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(52);
            if weeks <= 4 {
                1.0
            } else if weeks <= 8 {
                0.8
            } else if weeks <= 12 {
                0.5
            } else {
                0.2
            }
        })
    }
}

/// Aggregates all vendor evaluation scores and emits a ranked recommendation.
///
/// Maps to `partner_sourcer`. High-risk approval remains a governance concern
/// enforced by Converge gates or a downstream HITL policy.
pub struct VendorConsensusSuggestor;

#[async_trait::async_trait]
impl converge_pack::Suggestor for VendorConsensusSuggestor {
    fn name(&self) -> &'static str {
        "consensus"
    }

    fn dependencies(&self) -> &[converge_pack::ContextKey] {
        &[converge_pack::ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn converge_pack::Context) -> bool {
        ctx.has(converge_pack::ContextKey::Evaluations)
            && !ctx.has(converge_pack::ContextKey::Proposals)
    }

    async fn execute(&self, ctx: &dyn converge_pack::Context) -> converge_pack::AgentEffect {
        let evaluations = ctx.get(converge_pack::ContextKey::Evaluations);
        let mut scores: std::collections::HashMap<String, (f64, u32)> =
            std::collections::HashMap::new();

        for eval in evaluations {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&eval.content) {
                let id = json
                    .get("vendor_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("?");
                let score = json
                    .get("score")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                let entry = scores.entry(id.to_string()).or_insert((0.0, 0));
                entry.0 += score;
                entry.1 += 1;
            }
        }

        let mut ranked: Vec<(String, f64)> = scores
            .into_iter()
            .map(|(id, (total, count))| (id, total / f64::from(count)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let proposals: Vec<converge_pack::ProposedFact> = ranked
            .iter()
            .enumerate()
            .map(|(i, (vendor_id, score))| {
                converge_pack::ProposedFact::new(
                    converge_pack::ContextKey::Proposals,
                    format!("recommendation:{}", i + 1),
                    serde_json::json!({
                        "vendor_id": vendor_id,
                        "rank": i + 1,
                        "score": score,
                        "recommendation": if i == 0 { "recommended" } else { "alternative" }
                    })
                    .to_string(),
                    self.name(),
                )
                .with_confidence(if i == 0 { 0.85 } else { 0.6 })
            })
            .collect();

        converge_pack::AgentEffect::with_proposals(proposals)
    }
}

fn evaluate_vendors<F>(
    ctx: &dyn converge_pack::Context,
    criterion: &str,
    scorer: F,
) -> converge_pack::AgentEffect
where
    F: Fn(&serde_json::Value) -> f64,
{
    let signals = ctx.get(converge_pack::ContextKey::Signals);
    let evaluations: Vec<converge_pack::ProposedFact> = signals
        .iter()
        .filter_map(|signal| {
            let vendor: serde_json::Value = serde_json::from_str(&signal.content).ok()?;
            let id = vendor.get("id").and_then(serde_json::Value::as_str)?;
            let score = scorer(&vendor);
            Some(
                converge_pack::ProposedFact::new(
                    converge_pack::ContextKey::Evaluations,
                    format!("{criterion}:{id}"),
                    serde_json::json!({
                        "vendor_id": id,
                        "criterion": criterion,
                        "score": score,
                    })
                    .to_string(),
                    format!("{criterion}_evaluator"),
                )
                .with_confidence(1.0),
            )
        })
        .collect();

    converge_pack::AgentEffect::with_proposals(evaluations)
}
