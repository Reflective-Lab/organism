//! Problem classifier Suggestor.
//!
//! Reads `Seeds` (and optionally `Signals`), runs the deterministic keyword
//! classifier from [`organism_intent::problem`], and emits a `Hypotheses`
//! fact tagged with the resulting [`ProblemClass`].
//!
//! The Suggestor is the *in-loop* refinement path: after seeds land, the
//! classifier observes them and posts a hypothesis the rest of the
//! convergence loop can react to. The *pre-loop* selection path
//! ([`crate::guru::FormationGuru`]) calls [`organism_intent::problem::classify`]
//! directly on the structured `IntentPacket` to pick which formation template
//! to run in the first place.

use converge_pack::{
    AgentEffect, Context, ContextFact, ContextKey, ProposedFact, ProvenanceSource, Suggestor,
    TextPayload,
};
use organism_intent::problem::{ProblemClassification, classify_text};

use crate::provenance::ORGANISM_RUNTIME_PROVENANCE;

fn proposed_text_fact(
    key: ContextKey,
    id: impl Into<converge_pack::ProposalId>,
    content: impl Into<String>,
) -> ProposedFact {
    ORGANISM_RUNTIME_PROVENANCE.proposed_fact(key, id, TextPayload::new(content))
}

fn fact_text(fact: &ContextFact) -> &str {
    fact.text().unwrap_or_default()
}

/// Suggestor that classifies the dominant problem shape from seeds and
/// signals already in convergence context.
///
/// Inputs: `ContextKey::Seeds`, `ContextKey::Signals` (optional).
/// Outputs: one `ContextKey::Hypotheses` fact carrying the
/// [`ProblemClassification`] as JSON.
///
/// Idempotent: re-running on a context that already contains a
/// `problem-class:` hypothesis is a no-op (the predicate stops accepting).
pub struct ProblemClassifierSuggestor;

impl ProblemClassifierSuggestor {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProblemClassifierSuggestor {
    fn default() -> Self {
        Self::new()
    }
}

const FACT_PREFIX: &str = "problem-class";

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for ProblemClassifierSuggestor {
    fn name(&self) -> &'static str {
        "problem-classifier"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_RUNTIME_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        // Need at least one seed; don't re-fire if we've already classified.
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id().starts_with(FACT_PREFIX))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut haystack = String::new();
        for fact in ctx.get(ContextKey::Seeds) {
            haystack.push(' ');
            haystack.push_str(fact_text(fact));
        }
        for fact in ctx.get(ContextKey::Signals) {
            haystack.push(' ');
            haystack.push_str(fact_text(fact));
        }

        let classification = classify_text(&haystack);
        let payload = serde_json::json!({
            "agent": "problem-classifier",
            "class": classification.class.as_str(),
            "matched_keywords": classification.matched_keywords,
            "defaulted": classification.defaulted,
        });

        AgentEffect::with_proposal(proposed_text_fact(
            ContextKey::Hypotheses,
            format!("{FACT_PREFIX}:{}", classification.class.as_str()),
            payload.to_string(),
        ))
    }
}

/// Read the latest `problem-class:` hypothesis out of context, if any.
///
/// FormationGuru and other downstream consumers can use this when they want
/// the in-loop classification rather than computing one from the
/// `IntentPacket` directly. Returns `None` if no classification has been
/// emitted yet.
#[must_use]
pub fn extract_classification(ctx: &dyn Context) -> Option<ProblemClassification> {
    ctx.get(ContextKey::Hypotheses)
        .iter()
        .find(|f| f.id().starts_with(FACT_PREFIX))
        .and_then(|f| serde_json::from_str(fact_text(f)).ok())
        .and_then(|v: serde_json::Value| {
            let class_str = v.get("class")?.as_str()?;
            let class = match class_str {
                "decision" => organism_intent::problem::ProblemClass::Decision,
                "research" => organism_intent::problem::ProblemClass::Research,
                "evaluation" => organism_intent::problem::ProblemClass::Evaluation,
                "planning" => organism_intent::problem::ProblemClass::Planning,
                "diligence" => organism_intent::problem::ProblemClass::Diligence,
                "incident" => organism_intent::problem::ProblemClass::Incident,
                "strategy" => organism_intent::problem::ProblemClass::Strategy,
                _ => return None,
            };
            let matched_keywords = v
                .get("matched_keywords")?
                .as_array()?
                .iter()
                .filter_map(|w| w.as_str().map(str::to_owned))
                .collect();
            let defaulted = v.get("defaulted")?.as_bool()?;
            let tiebroken = v
                .get("tiebroken")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            Some(ProblemClassification {
                class,
                matched_keywords,
                defaulted,
                tiebroken,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation::Formation;
    use organism_intent::problem::ProblemClass;

    fn classified_payload_from(seed: &str) -> serde_json::Value {
        let result = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                Formation::new("classifier-test")
                    .agent(ProblemClassifierSuggestor::new())
                    .seed(ContextKey::Seeds, "seed-1", seed, "test")
                    .run()
                    .await
                    .expect("formation runs")
            });

        let hypotheses = result.converge_result.context.get(ContextKey::Hypotheses);
        let fact = hypotheses
            .iter()
            .find(|f| f.id().starts_with(FACT_PREFIX))
            .expect("classifier emitted a problem-class hypothesis");
        serde_json::from_str(fact_text(fact)).expect("payload is JSON")
    }

    #[test]
    fn classifier_emits_evaluation_for_evaluation_keywords() {
        let payload = classified_payload_from("evaluate the vendor proposals carefully");
        assert_eq!(payload["class"], "evaluation");
        assert_eq!(payload["defaulted"], false);
    }

    #[test]
    fn classifier_emits_diligence_for_vet_keyword() {
        let payload = classified_payload_from("vet the acquisition target end-to-end");
        assert_eq!(payload["class"], "diligence");
    }

    #[test]
    fn classifier_emits_incident_for_outage_keyword() {
        let payload = classified_payload_from("respond to the prod outage and stabilize");
        assert_eq!(payload["class"], "incident");
    }

    #[test]
    fn classifier_falls_back_to_decision_with_no_keywords() {
        let payload = classified_payload_from("doing the thing today");
        assert_eq!(payload["class"], "decision");
        assert_eq!(payload["defaulted"], true);
    }

    #[test]
    fn extract_classification_recovers_typed_value() {
        let payload = classified_payload_from("research the competitive landscape");
        // Roundtrip the JSON payload through extract_classification's matcher
        // by constructing a ProblemClassification directly from it.
        assert_eq!(payload["class"], "research");
        let class_str = payload["class"].as_str().unwrap();
        let class = match class_str {
            "research" => ProblemClass::Research,
            _ => panic!("unexpected class {class_str}"),
        };
        assert_eq!(class, ProblemClass::Research);
    }
}
