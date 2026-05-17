//! Role stall detection — Suggestor that watches one ContextKey and emits a
//! `Diagnostic` recommendation when the role bound to that key is failing
//! to contribute while convergence is otherwise progressing.
//!
//! The stall detector is **observation, not action**. It does not swap
//! descriptors or restart the formation — that's host policy. Organism's
//! contribution is the signal: "the role bound to `<key>` produced nothing
//! after N rounds of progress elsewhere; consider an alternate."
//!
//! See `MILESTONES.md` Stage 3+ "In-loop re-selection" for design rationale.

use converge_pack::{
    AgentEffect, Context, ContextKey, ProposedFact, ProvenanceSource, Suggestor, TextPayload,
};

use crate::provenance::ORGANISM_RUNTIME_PROVENANCE;

fn proposed_text_fact(
    key: ContextKey,
    id: impl Into<converge_pack::ProposalId>,
    content: impl Into<String>,
) -> ProposedFact {
    ORGANISM_RUNTIME_PROVENANCE.proposed_fact(key, id, TextPayload::new(content))
}

/// Suggestor that watches one [`ContextKey`] and flags the role bound to it
/// as stalled when convergence is happening elsewhere.
///
/// Inputs: any keys (the detector counts facts globally to gauge "is the
/// run making progress at all?").
///
/// Output: zero or one `ContextKey::Diagnostic` fact with id
/// `stall:<role_label>` carrying the watched-key emptiness, total
/// progress facts seen, and a recommendation string.
///
/// Idempotent: emits at most one stall fact per role per run; the
/// `accepts` predicate stops firing after the first emission.
pub struct RoleStallSuggestor {
    watched_key: ContextKey,
    role_label: String,
    /// Minimum total facts elsewhere in context before emptiness in
    /// `watched_key` counts as a stall. Three is the default — meaningful
    /// progress has happened, but the watched role hasn't joined.
    min_progress: usize,
    /// Dependency list returned by `Suggestor::dependencies` — every output
    /// key except the watched one. Lets the engine schedule us whenever
    /// progress lands somewhere other than the role we're watching.
    deps: Vec<ContextKey>,
}

impl RoleStallSuggestor {
    /// Create a new stall detector. `role_label` is the human-readable
    /// label the host's recommendation logic will route on.
    #[must_use]
    pub fn new(watched_key: ContextKey, role_label: impl Into<String>) -> Self {
        let deps = OUTPUT_KEYS
            .iter()
            .copied()
            .filter(|k| *k != watched_key)
            .collect();
        Self {
            watched_key,
            role_label: role_label.into(),
            min_progress: 3,
            deps,
        }
    }

    /// Tune how many "elsewhere" facts must have landed before we count an
    /// empty watched key as a stall. Lower = more sensitive (fires earlier);
    /// higher = quieter.
    #[must_use]
    pub fn with_min_progress(mut self, min_progress: usize) -> Self {
        self.min_progress = min_progress;
        self
    }

    fn fact_id(&self) -> String {
        format!("{FACT_PREFIX}:{}", self.role_label)
    }

    fn already_emitted(&self, ctx: &dyn Context) -> bool {
        let target = self.fact_id();
        ctx.get(ContextKey::Diagnostic)
            .iter()
            .any(|f| f.id().as_str() == target)
    }

    fn progress_elsewhere(&self, ctx: &dyn Context) -> usize {
        // Count *computed* facts under every key except the watched key
        // and Seeds. Seeds are inputs, not output — they're always there
        // from the start, so they don't represent "the run is making
        // progress".
        const PROGRESS_KEYS: &[ContextKey] = &[
            ContextKey::Signals,
            ContextKey::Proposals,
            ContextKey::Evaluations,
            ContextKey::Strategies,
            ContextKey::Constraints,
            ContextKey::Hypotheses,
            ContextKey::Diagnostic,
            ContextKey::Votes,
            ContextKey::Disagreements,
            ContextKey::ConsensusOutcomes,
        ];
        PROGRESS_KEYS
            .iter()
            .filter(|k| **k != self.watched_key)
            .map(|k| ctx.get(*k).len())
            .sum()
    }
}

const FACT_PREFIX: &str = "stall";

/// Keys other Suggestors typically write to. The stall detector depends
/// on every output key except its watched one — so the engine wakes it up
/// whenever progress lands somewhere it can compare against.
const OUTPUT_KEYS: &[ContextKey] = &[
    ContextKey::Signals,
    ContextKey::Strategies,
    ContextKey::Proposals,
    ContextKey::Evaluations,
    ContextKey::Constraints,
    ContextKey::Hypotheses,
    ContextKey::Diagnostic,
];

#[async_trait::async_trait]
#[allow(clippy::unnecessary_literal_bound)]
impl Suggestor for RoleStallSuggestor {
    fn name(&self) -> &'static str {
        "role-stall"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &self.deps
    }

    fn provenance(&self) -> &'static str {
        ORGANISM_RUNTIME_PROVENANCE.as_str()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if self.already_emitted(ctx) {
            return false;
        }
        // Watched key is non-empty? Role is contributing — no stall.
        if !ctx.get(self.watched_key).is_empty() {
            return false;
        }
        // Watched key is empty AND progress is happening elsewhere.
        self.progress_elsewhere(ctx) >= self.min_progress
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let progress = self.progress_elsewhere(ctx);
        let payload = serde_json::json!({
            "agent": "role-stall",
            "role": self.role_label,
            "watched_key": format!("{:?}", self.watched_key),
            "progress_elsewhere": progress,
            "recommendation": format!(
                "consider an alternate descriptor for role `{}`; nothing under {:?} after {} facts elsewhere",
                self.role_label, self.watched_key, progress,
            ),
            "severity": "stall",
        });
        AgentEffect::with_proposal(proposed_text_fact(
            ContextKey::Diagnostic,
            self.fact_id(),
            payload.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation::Formation;

    /// Productive partner: emits a Strategy fact when it sees a Seed.
    /// Pairs with a stall detector watching Evaluations to demonstrate "one
    /// role producing, the other not".
    struct ProductiveStrategist;

    #[async_trait::async_trait]
    #[allow(clippy::unnecessary_literal_bound)]
    impl Suggestor for ProductiveStrategist {
        fn name(&self) -> &'static str {
            "productive-strategist"
        }

        fn dependencies(&self) -> &[ContextKey] {
            &[ContextKey::Seeds]
        }

        fn provenance(&self) -> &'static str {
            ORGANISM_RUNTIME_PROVENANCE.as_str()
        }

        fn accepts(&self, ctx: &dyn Context) -> bool {
            ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Strategies)
        }

        async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
            AgentEffect::with_proposal(proposed_text_fact(
                ContextKey::Strategies,
                "strat-1".to_string(),
                "{\"plan\": \"draft\"}".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn fires_when_watched_key_empty_and_others_progressing() {
        let result = Formation::new("stall-test")
            .agent(ProductiveStrategist)
            .agent(
                RoleStallSuggestor::new(ContextKey::Evaluations, "evaluator").with_min_progress(1),
            )
            .seed(ContextKey::Seeds, "s1", "seed", "test")
            .run()
            .await
            .expect("formation runs");

        let diagnostic = result.converge_result.context.get(ContextKey::Diagnostic);
        let stall = diagnostic
            .iter()
            .find(|f| f.id().as_str() == "stall:evaluator")
            .expect("stall fact emitted for the missing evaluator role");
        let payload: serde_json::Value =
            serde_json::from_str(stall.text().unwrap_or_default()).expect("payload is JSON");
        assert_eq!(payload["role"], "evaluator");
        assert_eq!(payload["severity"], "stall");
        assert!(payload["progress_elsewhere"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn quiet_when_watched_key_is_active() {
        // Watch Strategies — the productive partner writes there, so no stall.
        let result = Formation::new("no-stall")
            .agent(ProductiveStrategist)
            .agent(
                RoleStallSuggestor::new(ContextKey::Strategies, "strategist").with_min_progress(1),
            )
            .seed(ContextKey::Seeds, "s1", "seed", "test")
            .run()
            .await
            .expect("formation runs");

        let diagnostic = result.converge_result.context.get(ContextKey::Diagnostic);
        assert!(
            diagnostic.iter().all(|f| !f.id().starts_with("stall:")),
            "stall should NOT fire when the watched role is producing"
        );
    }

    #[tokio::test]
    async fn quiet_until_min_progress_threshold() {
        // High threshold; the productive partner contributes one fact, less
        // than the threshold. Stall stays silent.
        let result = Formation::new("threshold")
            .agent(ProductiveStrategist)
            .agent(
                RoleStallSuggestor::new(ContextKey::Evaluations, "evaluator").with_min_progress(99),
            )
            .seed(ContextKey::Seeds, "s1", "seed", "test")
            .run()
            .await
            .expect("formation runs");

        let diagnostic = result.converge_result.context.get(ContextKey::Diagnostic);
        assert!(
            diagnostic.iter().all(|f| !f.id().starts_with("stall:")),
            "stall should NOT fire below min_progress threshold"
        );
    }
}
