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
}
