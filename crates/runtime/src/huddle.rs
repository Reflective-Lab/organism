//! Huddle primitives — platform-level deliberation patterns.
//!
//! Suggestors here consume governance facts ([`Vote`], [`Disagreement`]) and
//! produce [`ConsensusOutcome`] facts. They power any flow that needs
//! collective sign-off: research huddles, vendor-selection panels, approval
//! gates, multi-agent reviews.
//!
//! This module is the upstream home for patterns Wolfgang previously kept in
//! its `deep_research_runtime`. Domain packs (Wolfgang, Monterro, etc.) compose
//! these primitives through `Formation` rather than reinventing them.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use converge_pack::{
    AgentEffect, ConsensusOutcome, ConsensusRule, Context, ContextKey, ProposedFact, Suggestor,
    Vote, VoteTopicId,
};

/// Fact-id and key conventions for round-based deliberation.
///
/// Wolfgang's research huddles, Monterro's diligence loops, and any other
/// pack that wants round-by-round turn-taking shares this naming so upstream
/// suggestors can read and write the same facts.
#[derive(Debug, Clone, Copy)]
pub struct RoundConventions {
    /// Where round-start signals live. Default [`ContextKey::Signals`].
    pub round_signal_key: ContextKey,
    /// Fact-id prefix for round-start signals. Default `"round:start:"`.
    pub round_signal_prefix: &'static str,
    /// Where "this round may continue" markers live. Default
    /// [`ContextKey::Constraints`].
    pub continue_key: ContextKey,
    /// Fact-id prefix for continue markers. Default `"round:continue:"`.
    pub continue_prefix: &'static str,
}

impl RoundConventions {
    #[must_use]
    pub const fn default_const() -> Self {
        Self {
            round_signal_key: ContextKey::Signals,
            round_signal_prefix: "round:start:",
            continue_key: ContextKey::Constraints,
            continue_prefix: "round:continue:",
        }
    }

    fn round_signal_id(&self, round: u8) -> String {
        format!("{}{round}", self.round_signal_prefix)
    }

    fn continue_id(&self, round: u8) -> String {
        format!("{}{round}", self.continue_prefix)
    }
}

impl Default for RoundConventions {
    fn default() -> Self {
        Self::default_const()
    }
}

/// Boxed terminal-state predicate: returns true to halt round emission.
pub type TerminalPredicate = Box<dyn Fn(&dyn Context) -> bool + Send + Sync>;

fn never_terminal() -> TerminalPredicate {
    Box::new(|_ctx| false)
}

fn has_fact(ctx: &dyn Context, key: ContextKey, id: &str) -> bool {
    ctx.get(key).iter().any(|fact| fact.id.as_str() == id)
}

/// Drives round-by-round deliberation by emitting `round:start:N` signals.
///
/// Round 1 fires when no round has started yet. Round N+1 fires when the
/// previous round has been marked continue (a fact under
/// [`RoundConventions::continue_key`] with id `round:continue:N`). Stops when
/// the configured terminal predicate returns true or `max_rounds` is reached.
///
/// Domain packs supply the terminal predicate to express research-specific
/// completion markers (e.g. Wolfgang's `research:complete` /
/// `research:max_rounds_reached` facts). The platform stays agnostic.
pub struct RoundStarter {
    max_rounds: u8,
    conventions: RoundConventions,
    is_terminal: TerminalPredicate,
}

impl RoundStarter {
    #[must_use]
    pub fn new(max_rounds: u8) -> Self {
        Self {
            max_rounds,
            conventions: RoundConventions::default(),
            is_terminal: never_terminal(),
        }
    }

    #[must_use]
    pub fn with_conventions(mut self, conventions: RoundConventions) -> Self {
        self.conventions = conventions;
        self
    }

    /// Provide a domain-specific terminal-state predicate. Returns `true` to
    /// stop round emission (e.g. when a research-complete fact is present).
    #[must_use]
    pub fn with_terminal_predicate<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&dyn Context) -> bool + Send + Sync + 'static,
    {
        self.is_terminal = Box::new(predicate);
        self
    }

    fn next_round_to_emit(&self, ctx: &dyn Context) -> Option<u8> {
        if !has_fact(
            ctx,
            self.conventions.round_signal_key,
            &self.conventions.round_signal_id(1),
        ) {
            return Some(1);
        }
        for round in 1..self.max_rounds {
            if has_fact(
                ctx,
                self.conventions.continue_key,
                &self.conventions.continue_id(round),
            ) && !has_fact(
                ctx,
                self.conventions.round_signal_key,
                &self.conventions.round_signal_id(round + 1),
            ) {
                return Some(round + 1);
            }
        }
        None
    }
}

#[async_trait::async_trait]
impl Suggestor for RoundStarter {
    fn name(&self) -> &'static str {
        "organism-round-starter"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if (self.is_terminal)(ctx) {
            return false;
        }
        self.next_round_to_emit(ctx).is_some()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let Some(round) = self.next_round_to_emit(ctx) else {
            return AgentEffect::empty();
        };
        AgentEffect::with_proposal(ProposedFact::new(
            self.conventions.round_signal_key,
            self.conventions.round_signal_id(round),
            format!("start round {round}"),
            self.name(),
        ))
    }
}

/// Tallies [`Vote`] facts under [`ContextKey::Votes`] against a
/// [`ConsensusRule`] and emits [`ConsensusOutcome`] facts under
/// [`ContextKey::ConsensusOutcomes`].
///
/// Vote payloads are read as JSON-serialized [`Vote`] structs from each fact's
/// content. Outcomes are emitted at most once per topic — once a topic has an
/// outcome, additional votes on that topic are ignored. This matches the
/// round-based deliberation pattern: each round has its own topic id, so
/// re-tallying happens by issuing a new topic, not by amending an old one.
pub struct ConsensusEvaluator {
    rule: ConsensusRule,
    total_voters: usize,
    decided_topics: Mutex<HashSet<VoteTopicId>>,
}

impl ConsensusEvaluator {
    #[must_use]
    pub fn new(rule: ConsensusRule, total_voters: usize) -> Self {
        Self {
            rule,
            total_voters,
            decided_topics: Mutex::new(HashSet::new()),
        }
    }

    #[must_use]
    pub const fn rule(&self) -> ConsensusRule {
        self.rule
    }

    #[must_use]
    pub const fn total_voters(&self) -> usize {
        self.total_voters
    }
}

#[async_trait::async_trait]
impl Suggestor for ConsensusEvaluator {
    fn name(&self) -> &'static str {
        "organism-consensus-evaluator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Votes]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Votes) {
            return false;
        }
        let decided = self.decided_topics.lock().unwrap();
        ctx.get(ContextKey::Votes)
            .iter()
            .filter_map(|fact| serde_json::from_str::<Vote>(&fact.content).ok())
            .any(|vote| !decided.contains(&vote.topic))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut decided = self.decided_topics.lock().unwrap();

        let mut by_topic: HashMap<VoteTopicId, Vec<Vote>> = HashMap::new();
        for fact in ctx.get(ContextKey::Votes) {
            let Ok(vote) = serde_json::from_str::<Vote>(&fact.content) else {
                continue;
            };
            if decided.contains(&vote.topic) {
                continue;
            }
            by_topic.entry(vote.topic.clone()).or_default().push(vote);
        }

        let mut topics: Vec<VoteTopicId> = by_topic.keys().cloned().collect();
        topics.sort();

        let mut proposals = Vec::with_capacity(topics.len());
        for topic in topics {
            let votes = by_topic.remove(&topic).unwrap_or_default();
            let outcome =
                ConsensusOutcome::evaluate(topic.clone(), self.rule, &votes, self.total_voters);
            let Ok(content) = serde_json::to_string(&outcome) else {
                continue;
            };
            proposals.push(ProposedFact::new(
                ContextKey::ConsensusOutcomes,
                format!("outcome:{}", topic.as_str()),
                content,
                self.name(),
            ));
            decided.insert(topic);
        }

        AgentEffect::with_proposals(proposals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation::Formation;
    use converge_pack::{ActorId, VoteDecision};

    fn vote(topic: &str, voter: &str, decision: VoteDecision) -> Vote {
        Vote {
            topic: VoteTopicId::new(topic),
            voter: ActorId::new(voter),
            decision,
            reason: None,
        }
    }

    fn formation_with_votes(
        label: &str,
        rule: ConsensusRule,
        total_voters: usize,
        votes: &[Vote],
    ) -> Formation {
        let mut formation =
            Formation::new(label).agent(ConsensusEvaluator::new(rule, total_voters));
        for (i, v) in votes.iter().enumerate() {
            let content = serde_json::to_string(v).unwrap();
            formation = formation.seed(
                ContextKey::Votes,
                format!("vote-{i}"),
                content,
                "test-author",
            );
        }
        formation
    }

    #[tokio::test]
    async fn emits_outcome_per_topic_under_consensus_outcomes_key() {
        let votes = [
            vote("done-r1", "alice", VoteDecision::Yes),
            vote("done-r1", "bob", VoteDecision::Yes),
            vote("done-r1", "carol", VoteDecision::No),
        ];
        let result = formation_with_votes("majority-pass", ConsensusRule::Majority, 3, &votes)
            .run()
            .await
            .expect("formation should converge");

        let outcomes = result
            .converge_result
            .context
            .get(ContextKey::ConsensusOutcomes);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].id.as_str(), "outcome:done-r1");

        let outcome: ConsensusOutcome = serde_json::from_str(&outcomes[0].content).unwrap();
        assert_eq!(outcome.yes_votes, 2);
        assert_eq!(outcome.no_votes, 1);
        assert_eq!(outcome.total_voters, 3);
        assert!(outcome.passes);
    }

    #[tokio::test]
    async fn evaluates_each_topic_independently() {
        let votes = [
            vote("a", "alice", VoteDecision::Yes),
            vote("a", "bob", VoteDecision::Yes),
            vote("b", "alice", VoteDecision::No),
            vote("b", "bob", VoteDecision::No),
        ];
        let result = formation_with_votes("split-topics", ConsensusRule::Majority, 2, &votes)
            .run()
            .await
            .expect("formation should converge");

        let outcomes = result
            .converge_result
            .context
            .get(ContextKey::ConsensusOutcomes);
        assert_eq!(outcomes.len(), 2);

        let mut decisions: std::collections::HashMap<String, ConsensusOutcome> =
            std::collections::HashMap::new();
        for fact in outcomes {
            decisions.insert(
                fact.id.as_str().to_string(),
                serde_json::from_str(&fact.content).unwrap(),
            );
        }
        assert!(decisions["outcome:a"].passes);
        assert!(!decisions["outcome:b"].passes);
    }

    #[tokio::test]
    async fn unanimous_rule_blocks_when_any_voter_dissents() {
        let votes = [
            vote("ship", "a", VoteDecision::Yes),
            vote("ship", "b", VoteDecision::No),
        ];
        let result = formation_with_votes("unanimous-fail", ConsensusRule::Unanimous, 2, &votes)
            .run()
            .await
            .expect("formation should converge");

        let outcomes = result
            .converge_result
            .context
            .get(ContextKey::ConsensusOutcomes);
        assert_eq!(outcomes.len(), 1);
        let outcome: ConsensusOutcome = serde_json::from_str(&outcomes[0].content).unwrap();
        assert!(!outcome.passes);
    }

    #[tokio::test]
    async fn does_not_emit_when_no_votes_seeded() {
        let result = Formation::new("no-votes")
            .agent(ConsensusEvaluator::new(ConsensusRule::Majority, 1))
            .run()
            .await
            .expect("formation should converge");

        assert!(
            !result
                .converge_result
                .context
                .has(ContextKey::ConsensusOutcomes)
        );
    }

    // ── RoundStarter ──────────────────────────────────────────────

    #[tokio::test]
    async fn round_starter_emits_round_one_when_no_round_has_started() {
        let result = Formation::new("round-1")
            .agent(RoundStarter::new(3))
            .run()
            .await
            .expect("formation should converge");

        let signals = result.converge_result.context.get(ContextKey::Signals);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].id.as_str(), "round:start:1");
    }

    #[tokio::test]
    async fn round_starter_advances_when_continue_marker_lands() {
        let result = Formation::new("round-2")
            .agent(RoundStarter::new(3))
            .seed(
                ContextKey::Constraints,
                "round:continue:1",
                "round 1 voted to continue",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        let mut signal_ids: Vec<&str> = result
            .converge_result
            .context
            .get(ContextKey::Signals)
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        signal_ids.sort_unstable();
        assert_eq!(signal_ids, vec!["round:start:1", "round:start:2"]);
    }

    #[tokio::test]
    async fn round_starter_stops_at_max_rounds() {
        let mut formation = Formation::new("max-cap").agent(RoundStarter::new(2));
        for round in 1..=2 {
            formation = formation.seed(
                ContextKey::Constraints,
                format!("round:continue:{round}"),
                format!("continue {round}"),
                "test-author",
            );
        }
        let result = formation.run().await.expect("formation should converge");

        let signals = result.converge_result.context.get(ContextKey::Signals);
        assert_eq!(signals.len(), 2);
        assert!(!signals.iter().any(|f| f.id.as_str() == "round:start:3"));
    }

    #[tokio::test]
    async fn round_starter_respects_terminal_predicate() {
        const TERMINAL_ID: &str = "research:complete";
        let result = Formation::new("terminal-block")
            .agent(RoundStarter::new(3).with_terminal_predicate(|ctx| {
                ctx.get(ContextKey::Strategies)
                    .iter()
                    .any(|f| f.id.as_str() == TERMINAL_ID)
            }))
            .seed(
                ContextKey::Strategies,
                TERMINAL_ID,
                "research is done",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        assert!(!result.converge_result.context.has(ContextKey::Signals));
    }

    #[tokio::test]
    async fn round_starter_honors_custom_conventions() {
        let conventions = RoundConventions {
            round_signal_key: ContextKey::Hypotheses,
            round_signal_prefix: "phase:",
            continue_key: ContextKey::Strategies,
            continue_prefix: "phase:next:",
        };
        let result = Formation::new("custom-conv")
            .agent(RoundStarter::new(3).with_conventions(conventions))
            .seed(
                ContextKey::Strategies,
                "phase:next:1",
                "advance",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        let mut ids: Vec<&str> = result
            .converge_result
            .context
            .get(ContextKey::Hypotheses)
            .iter()
            .map(|f| f.id.as_str())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, vec!["phase:1", "phase:2"]);
    }
}
