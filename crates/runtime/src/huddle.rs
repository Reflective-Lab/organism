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
    AgentEffect, ConsensusOutcome, ConsensusRule, Context, ContextKey, Disagreement, Fact,
    ProposedFact, Suggestor, Vote, VoteTopicId,
};
use serde::{Deserialize, Serialize};

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
    /// Where per-round contributor notes live. Default
    /// [`ContextKey::Hypotheses`]. Note ids are expected to end with `:N` for
    /// round N (matching Wolfgang's `note:{participant}:{round}` shape).
    pub note_key: ContextKey,
    /// Where round syntheses live. Default [`ContextKey::Strategies`].
    pub synthesis_key: ContextKey,
    /// Fact-id prefix for syntheses. Default `"synthesis:"`.
    pub synthesis_prefix: &'static str,
}

impl RoundConventions {
    #[must_use]
    pub const fn default_const() -> Self {
        Self {
            round_signal_key: ContextKey::Signals,
            round_signal_prefix: "round:start:",
            continue_key: ContextKey::Constraints,
            continue_prefix: "round:continue:",
            note_key: ContextKey::Hypotheses,
            synthesis_key: ContextKey::Strategies,
            synthesis_prefix: "synthesis:",
        }
    }

    fn round_signal_id(&self, round: u8) -> String {
        format!("{}{round}", self.round_signal_prefix)
    }

    fn continue_id(&self, round: u8) -> String {
        format!("{}{round}", self.continue_prefix)
    }

    fn synthesis_id(&self, round: u8) -> String {
        format!("{}{round}", self.synthesis_prefix)
    }

    fn note_belongs_to_round(fact_id: &str, round: u8) -> bool {
        fact_id.ends_with(&format!(":{round}"))
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

/// Produces a synthesis string from a slice of round notes.
///
/// Implementations own the content production — calling LLMs, applying
/// templates, running rule-based summarizers — while [`RoundSynthesizer`]
/// owns the orchestration (which round to synthesize, when notes are
/// complete, where to write the result).
#[async_trait::async_trait]
pub trait SynthesisProducer: Send + Sync {
    /// Synthesize the given round's notes into a single content payload.
    ///
    /// `notes` are facts from [`RoundConventions::note_key`] whose ids end
    /// with `:N` for round `N`. Return the content the engine should publish
    /// under the synthesis fact, or an error message that the orchestrator
    /// will route to [`ContextKey::Diagnostic`].
    async fn synthesize(&self, round: u8, notes: &[Fact]) -> Result<String, String>;
}

/// Drives round-by-round synthesis.
///
/// Watches [`RoundConventions::round_signal_key`] for started rounds, counts
/// notes for each round under [`RoundConventions::note_key`], and once a
/// round has at least `expected_note_count` notes (and no synthesis yet),
/// invokes the [`SynthesisProducer`] and emits a synthesis fact under
/// [`RoundConventions::synthesis_key`].
///
/// Errors from the producer are routed to [`ContextKey::Diagnostic`] with a
/// `runtime:error:synthesis:{round}` id and never block convergence on the
/// happy path of other rounds.
pub struct RoundSynthesizer<P: SynthesisProducer> {
    expected_note_count: usize,
    conventions: RoundConventions,
    producer: P,
}

impl<P: SynthesisProducer> RoundSynthesizer<P> {
    #[must_use]
    pub fn new(expected_note_count: usize, producer: P) -> Self {
        Self {
            expected_note_count,
            conventions: RoundConventions::default(),
            producer,
        }
    }

    #[must_use]
    pub fn with_conventions(mut self, conventions: RoundConventions) -> Self {
        self.conventions = conventions;
        self
    }

    fn started_rounds(&self, ctx: &dyn Context) -> Vec<u8> {
        ctx.get(self.conventions.round_signal_key)
            .iter()
            .filter_map(|fact| {
                fact.id
                    .as_str()
                    .strip_prefix(self.conventions.round_signal_prefix)
                    .and_then(|n| n.parse::<u8>().ok())
            })
            .collect()
    }

    fn count_notes_for_round(&self, ctx: &dyn Context, round: u8) -> usize {
        ctx.get(self.conventions.note_key)
            .iter()
            .filter(|fact| RoundConventions::note_belongs_to_round(fact.id.as_str(), round))
            .count()
    }

    fn notes_for_round<'a>(&self, ctx: &'a dyn Context, round: u8) -> Vec<&'a Fact> {
        ctx.get(self.conventions.note_key)
            .iter()
            .filter(|fact| RoundConventions::note_belongs_to_round(fact.id.as_str(), round))
            .collect()
    }

    fn next_round_needing_synthesis(&self, ctx: &dyn Context) -> Option<u8> {
        self.started_rounds(ctx).into_iter().find(|round| {
            !has_fact(
                ctx,
                self.conventions.synthesis_key,
                &self.conventions.synthesis_id(*round),
            ) && self.count_notes_for_round(ctx, *round) >= self.expected_note_count
        })
    }
}

#[async_trait::async_trait]
impl<P: SynthesisProducer> Suggestor for RoundSynthesizer<P> {
    fn name(&self) -> &'static str {
        "organism-round-synthesizer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        self.next_round_needing_synthesis(ctx).is_some()
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let Some(round) = self.next_round_needing_synthesis(ctx) else {
            return AgentEffect::empty();
        };

        let notes: Vec<Fact> = self
            .notes_for_round(ctx, round)
            .into_iter()
            .cloned()
            .collect();

        match self.producer.synthesize(round, &notes).await {
            Ok(content) => AgentEffect::with_proposal(ProposedFact::new(
                self.conventions.synthesis_key,
                self.conventions.synthesis_id(round),
                content,
                self.name(),
            )),
            Err(err) => AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Diagnostic,
                format!("runtime:error:synthesis:{round}"),
                err,
                self.name(),
            )),
        }
    }
}

/// Aggregated dissent payload emitted by [`DisagreementMapper`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisagreementMap {
    pub topic: VoteTopicId,
    pub entries: Vec<Disagreement>,
}

/// Aggregates [`Disagreement`] facts under [`ContextKey::Disagreements`] into
/// per-topic [`DisagreementMap`] payloads.
///
/// Output goes under a configurable key (default [`ContextKey::Diagnostic`])
/// with id `disagreement_map:{topic}`. Maps are emitted at most once per
/// topic — the same once-per-topic discipline as [`ConsensusEvaluator`].
pub struct DisagreementMapper {
    output_key: ContextKey,
    mapped_topics: Mutex<HashSet<VoteTopicId>>,
}

impl DisagreementMapper {
    #[must_use]
    pub fn new() -> Self {
        Self {
            output_key: ContextKey::Diagnostic,
            mapped_topics: Mutex::new(HashSet::new()),
        }
    }

    #[must_use]
    pub fn with_output_key(mut self, key: ContextKey) -> Self {
        self.output_key = key;
        self
    }

    #[must_use]
    pub const fn output_key(&self) -> ContextKey {
        self.output_key
    }

    fn map_id(topic: &VoteTopicId) -> String {
        format!("disagreement_map:{}", topic.as_str())
    }
}

impl Default for DisagreementMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Suggestor for DisagreementMapper {
    fn name(&self) -> &'static str {
        "organism-disagreement-mapper"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Disagreements]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        if !ctx.has(ContextKey::Disagreements) {
            return false;
        }
        let mapped = self.mapped_topics.lock().unwrap();
        ctx.get(ContextKey::Disagreements)
            .iter()
            .filter_map(|fact| serde_json::from_str::<Disagreement>(&fact.content).ok())
            .any(|d| !mapped.contains(&d.topic))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut mapped = self.mapped_topics.lock().unwrap();

        let mut by_topic: HashMap<VoteTopicId, Vec<Disagreement>> = HashMap::new();
        for fact in ctx.get(ContextKey::Disagreements) {
            let Ok(d) = serde_json::from_str::<Disagreement>(&fact.content) else {
                continue;
            };
            if mapped.contains(&d.topic) {
                continue;
            }
            by_topic.entry(d.topic.clone()).or_default().push(d);
        }

        let mut topics: Vec<VoteTopicId> = by_topic.keys().cloned().collect();
        topics.sort();

        let mut proposals = Vec::with_capacity(topics.len());
        for topic in topics {
            let entries = by_topic.remove(&topic).unwrap_or_default();
            let map = DisagreementMap {
                topic: topic.clone(),
                entries,
            };
            let Ok(content) = serde_json::to_string(&map) else {
                continue;
            };
            proposals.push(ProposedFact::new(
                self.output_key,
                Self::map_id(&topic),
                content,
                self.name(),
            ));
            mapped.insert(topic);
        }

        AgentEffect::with_proposals(proposals)
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
            note_key: ContextKey::Hypotheses,
            synthesis_key: ContextKey::Strategies,
            synthesis_prefix: "phase:synthesis:",
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

    // ── RoundSynthesizer ──────────────────────────────────────────

    struct StaticProducer(&'static str);

    #[async_trait::async_trait]
    impl SynthesisProducer for StaticProducer {
        async fn synthesize(&self, _round: u8, _notes: &[Fact]) -> Result<String, String> {
            Ok(self.0.to_string())
        }
    }

    struct CountingProducer;

    #[async_trait::async_trait]
    impl SynthesisProducer for CountingProducer {
        async fn synthesize(&self, round: u8, notes: &[Fact]) -> Result<String, String> {
            Ok(format!("round {round} from {} notes", notes.len()))
        }
    }

    struct FailingProducer(&'static str);

    #[async_trait::async_trait]
    impl SynthesisProducer for FailingProducer {
        async fn synthesize(&self, _round: u8, _notes: &[Fact]) -> Result<String, String> {
            Err(self.0.to_string())
        }
    }

    fn formation_with_round_one_started(label: &str) -> Formation {
        Formation::new(label).seed(
            ContextKey::Signals,
            "round:start:1",
            "start round 1",
            "test-author",
        )
    }

    #[tokio::test]
    async fn round_synthesizer_emits_when_notes_complete() {
        let result = formation_with_round_one_started("synth-complete")
            .agent(RoundSynthesizer::new(2, CountingProducer))
            .seed(
                ContextKey::Hypotheses,
                "note:alice:1",
                "alice note",
                "test-author",
            )
            .seed(
                ContextKey::Hypotheses,
                "note:bob:1",
                "bob note",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        let strategies = result.converge_result.context.get(ContextKey::Strategies);
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].id.as_str(), "synthesis:1");
        assert_eq!(strategies[0].content, "round 1 from 2 notes");
    }

    #[tokio::test]
    async fn round_synthesizer_waits_for_complete_note_count() {
        let result = formation_with_round_one_started("synth-incomplete")
            .agent(RoundSynthesizer::new(
                3,
                StaticProducer("should-not-appear"),
            ))
            .seed(
                ContextKey::Hypotheses,
                "note:alice:1",
                "alice note",
                "test-author",
            )
            .seed(
                ContextKey::Hypotheses,
                "note:bob:1",
                "bob note",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        assert!(!result.converge_result.context.has(ContextKey::Strategies));
    }

    #[tokio::test]
    async fn round_synthesizer_routes_producer_errors_to_diagnostic() {
        let result = formation_with_round_one_started("synth-err")
            .agent(RoundSynthesizer::new(
                1,
                FailingProducer("upstream timeout"),
            ))
            .seed(
                ContextKey::Hypotheses,
                "note:alice:1",
                "alice note",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        assert!(!result.converge_result.context.has(ContextKey::Strategies));
        let diagnostic = result.converge_result.context.get(ContextKey::Diagnostic);
        assert_eq!(diagnostic.len(), 1);
        assert_eq!(diagnostic[0].id.as_str(), "runtime:error:synthesis:1");
        assert_eq!(diagnostic[0].content, "upstream timeout");
    }

    #[tokio::test]
    async fn round_synthesizer_only_synthesizes_started_rounds() {
        let result = formation_with_round_one_started("synth-pending-round-2")
            .agent(RoundSynthesizer::new(1, StaticProducer("done")))
            .seed(
                ContextKey::Hypotheses,
                "note:alice:1",
                "round 1 note",
                "test-author",
            )
            .seed(
                ContextKey::Hypotheses,
                "note:alice:2",
                "round 2 note (not yet started)",
                "test-author",
            )
            .run()
            .await
            .expect("formation should converge");

        let strategies = result.converge_result.context.get(ContextKey::Strategies);
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].id.as_str(), "synthesis:1");
    }

    // ── DisagreementMapper ────────────────────────────────────────

    fn disagreement(topic: &str, dissenter: &str, reason: &str) -> Disagreement {
        Disagreement {
            topic: VoteTopicId::new(topic),
            dissenter: ActorId::new(dissenter),
            reason: reason.to_string(),
        }
    }

    fn seed_disagreement(formation: Formation, slot: usize, d: &Disagreement) -> Formation {
        let content = serde_json::to_string(d).unwrap();
        formation.seed(
            ContextKey::Disagreements,
            format!("disagreement-{slot}"),
            content,
            "test-author",
        )
    }

    #[tokio::test]
    async fn disagreement_mapper_aggregates_per_topic() {
        let alice_on_a = disagreement("topic-a", "alice", "too risky");
        let bob_on_a = disagreement("topic-a", "bob", "missing context");
        let carol_on_b = disagreement("topic-b", "carol", "missing data");

        let mut formation = Formation::new("dmap").agent(DisagreementMapper::new());
        formation = seed_disagreement(formation, 0, &alice_on_a);
        formation = seed_disagreement(formation, 1, &bob_on_a);
        formation = seed_disagreement(formation, 2, &carol_on_b);

        let result = formation.run().await.expect("formation should converge");

        let maps = result.converge_result.context.get(ContextKey::Diagnostic);
        assert_eq!(maps.len(), 2);

        let mut by_id: std::collections::HashMap<String, DisagreementMap> =
            std::collections::HashMap::new();
        for fact in maps {
            let parsed: DisagreementMap = serde_json::from_str(&fact.content).unwrap();
            by_id.insert(fact.id.as_str().to_string(), parsed);
        }
        let map_a = &by_id["disagreement_map:topic-a"];
        assert_eq!(map_a.entries.len(), 2);
        let map_b = &by_id["disagreement_map:topic-b"];
        assert_eq!(map_b.entries.len(), 1);
        assert_eq!(map_b.entries[0].dissenter.as_str(), "carol");
    }

    #[tokio::test]
    async fn disagreement_mapper_does_nothing_without_disagreements() {
        let result = Formation::new("dmap-empty")
            .agent(DisagreementMapper::new())
            .run()
            .await
            .expect("formation should converge");

        assert!(!result.converge_result.context.has(ContextKey::Diagnostic));
    }

    #[tokio::test]
    async fn disagreement_mapper_honors_custom_output_key() {
        let d = disagreement("topic-x", "alice", "too rushed");
        let mut formation = Formation::new("dmap-custom")
            .agent(DisagreementMapper::new().with_output_key(ContextKey::Strategies));
        formation = seed_disagreement(formation, 0, &d);

        let result = formation.run().await.expect("formation should converge");

        assert!(!result.converge_result.context.has(ContextKey::Diagnostic));
        let strategies = result.converge_result.context.get(ContextKey::Strategies);
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0].id.as_str(), "disagreement_map:topic-x");
    }
}
