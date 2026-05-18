//! Intent resolution — maps intents to the packs, capabilities, and invariants
//! needed for convergence.
//!
//! Four resolution levels:
//!
//! 1. **Declarative** — intent explicitly declares which packs it needs
//! 2. **Structural** — resolver matches fact prefixes to pack metadata
//! 3. **Semantic** — huddle matches outcome description to pack capabilities
//! 4. **Learned** — prior calibration from execution history predicts pack needs
//!
//! Resolution runs after admission, before planning. The output is an
//! `IntentBinding` that tells the runtime which agents to register
//! with the Converge engine.

use converge_pack::UnitInterval;
use serde::{Deserialize, Serialize};

// ── Typed identifiers ──────────────────────────────────────────────
//
// CapabilityRequirementId and InvariantId are Organism-owned typed
// wrappers over the human-readable strings that flow through intent
// resolution. Same shape as `SuggestorDescriptorId`, `ProviderId`,
// `FormationTemplateId` — `#[serde(transparent)]` so the wire form
// stays a bare string. Conversion impls let existing string-literal
// call sites flow through without churn.

macro_rules! string_id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }
        impl std::ops::Deref for $name {
            type Target = str;
            fn deref(&self) -> &str {
                &self.0
            }
        }
        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
        impl From<&String> for $name {
            fn from(s: &String) -> Self {
                Self(s.clone())
            }
        }
        impl From<$name> for String {
            fn from(id: $name) -> Self {
                id.0
            }
        }
        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0.as_str() == *other
            }
        }
        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                &self.0 == other
            }
        }
        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0.as_str()
            }
        }
        impl PartialEq<$name> for String {
            fn eq(&self, other: &$name) -> bool {
                self == &other.0
            }
        }
    };
}

string_id_newtype!(
    CapabilityRequirementId,
    "Identifier of a capability requested by an intent (e.g. `\"web\"`, `\"ocr\"`, `\"vision\"`). Distinct from `converge_kernel::formation::SuggestorCapability` (a closed enum on the Suggestor profile side); this is the open-world string label that crosses intent → resolver → registry."
);
string_id_newtype!(
    InvariantId,
    "Identifier of an invariant the intent requires the convergence loop to honor (e.g. `\"lead_has_source\"`, `\"claim_has_provenance\"`). Names a check registered with the runtime, not the check itself."
);

// ── Intent Binding ─────────────────────────────────────────────────

/// The output of intent resolution. Tells the runtime what to wire up.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentBinding {
    /// Which domain packs to register with the engine.
    pub packs: Vec<PackRequirement>,
    /// Which capabilities the intent needs (OCR, web, vision, etc.).
    pub capabilities: Vec<CapabilityRequirement>,
    /// Additional invariants to enforce beyond pack defaults.
    pub invariants: Vec<InvariantId>,
    /// How the binding was resolved.
    pub resolution: ResolutionTrace,
}

/// A domain pack needed by the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRequirement {
    pub pack_name: String,
    pub reason: String,
    pub confidence: UnitInterval,
    pub source: ResolutionLevel,
}

/// A capability needed by the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    pub capability: CapabilityRequirementId,
    pub reason: String,
    pub confidence: UnitInterval,
    pub source: ResolutionLevel,
}

/// Which resolution level produced the binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionLevel {
    /// Intent explicitly declared its packs.
    Declarative,
    /// Resolver matched fact prefixes to pack metadata.
    Structural,
    /// Huddle matched outcome to pack descriptions.
    Semantic,
    /// Prior calibration predicted from execution history.
    Learned,
}

/// How the resolution was performed — for traceability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionTrace {
    pub levels_attempted: Vec<ResolutionLevel>,
    pub levels_contributed: Vec<ResolutionLevel>,
    /// Number of prior episodes consulted (level 4).
    pub prior_episodes_consulted: usize,
    /// Confidence that the binding is complete.
    pub completeness_confidence: UnitInterval,
}

// ── Declarative Binding (Level 1) ──────────────────────────────────

/// Builder for declaring an intent's resource needs explicitly.
/// This is what apps use today.
///
/// ```rust,ignore
/// let binding = DeclarativeBinding::new()
///     .pack("customers", "lead qualification workflow")
///     .pack("linkedin_research", "enrich with LinkedIn data")
///     .capability("web", "capture company website")
///     .invariant("lead_has_source")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct DeclarativeBinding {
    packs: Vec<PackRequirement>,
    capabilities: Vec<CapabilityRequirement>,
    invariants: Vec<InvariantId>,
}

impl DeclarativeBinding {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn pack(mut self, name: impl Into<String>, reason: impl Into<String>) -> Self {
        self.packs.push(PackRequirement {
            pack_name: name.into(),
            reason: reason.into(),
            confidence: UnitInterval::ONE,
            source: ResolutionLevel::Declarative,
        });
        self
    }

    #[must_use]
    pub fn capability(
        mut self,
        name: impl Into<CapabilityRequirementId>,
        reason: impl Into<String>,
    ) -> Self {
        self.capabilities.push(CapabilityRequirement {
            capability: name.into(),
            reason: reason.into(),
            confidence: UnitInterval::ONE,
            source: ResolutionLevel::Declarative,
        });
        self
    }

    #[must_use]
    pub fn invariant(mut self, name: impl Into<InvariantId>) -> Self {
        self.invariants.push(name.into());
        self
    }

    #[must_use]
    pub fn build(self) -> IntentBinding {
        IntentBinding {
            packs: self.packs,
            capabilities: self.capabilities,
            invariants: self.invariants,
            resolution: ResolutionTrace {
                levels_attempted: vec![ResolutionLevel::Declarative],
                levels_contributed: vec![ResolutionLevel::Declarative],
                prior_episodes_consulted: 0,
                completeness_confidence: UnitInterval::ONE,
            },
        }
    }
}

// ── Resolution Trait ───────────────────────────────────────────────

/// Resolves an intent to its resource binding.
///
/// Implementations exist for each level. The runtime chains them:
/// declarative first, then structural fills gaps, semantic adds
/// uncertain matches, learned adjusts confidences from history.
pub trait IntentResolver: Send + Sync {
    fn level(&self) -> ResolutionLevel;
    fn resolve(&self, intent: &super::IntentPacket, current: &IntentBinding) -> IntentBinding;
}

// ── Semantic Resolver (Level 3) ────────────────────────────────────

/// Outcome → pack matcher. The semantic level delegates to an implementation
/// of this trait so the resolver itself stays free of vendor-specific LLM
/// imports. Constructor injection per the Plug Boundary doctrine: the
/// resolver declares a need (semantic outcome matching with reasons), and a
/// concrete matcher fulfils it.
///
/// Returned tuples: `(pack_name, confidence, reason)`. Confidence should be
/// in `[0.0, 1.0]`; reason is a short human-readable explanation that lands
/// in the resulting `PackRequirement.reason`.
pub trait SemanticMatcher: Send + Sync {
    fn match_packs(&self, outcome: &str) -> Vec<(String, f64, String)>;
}

/// Level 3 — Semantic resolver. Asks a `SemanticMatcher` to map the intent's
/// outcome description to candidate packs.
///
/// Adds matches that aren't already in the binding; never overwrites existing
/// pack entries. Records contribution under `ResolutionLevel::Semantic`.
pub struct SemanticResolver<M: SemanticMatcher> {
    matcher: M,
}

impl<M: SemanticMatcher> SemanticResolver<M> {
    #[must_use]
    pub fn new(matcher: M) -> Self {
        Self { matcher }
    }
}

impl<M: SemanticMatcher> IntentResolver for SemanticResolver<M> {
    fn level(&self) -> ResolutionLevel {
        ResolutionLevel::Semantic
    }

    fn resolve(&self, intent: &super::IntentPacket, current: &IntentBinding) -> IntentBinding {
        let mut binding = current.clone();
        let already_bound: std::collections::HashSet<String> =
            binding.packs.iter().map(|p| p.pack_name.clone()).collect();

        let mut contributed = false;
        for (pack_name, confidence, reason) in self.matcher.match_packs(&intent.outcome) {
            if already_bound.contains(&pack_name) {
                continue;
            }
            binding.packs.push(PackRequirement {
                pack_name,
                reason,
                confidence: UnitInterval::clamped(confidence),
                source: ResolutionLevel::Semantic,
            });
            contributed = true;
        }

        update_trace(
            &mut binding.resolution,
            ResolutionLevel::Semantic,
            contributed,
        );
        binding
    }
}

// ── Learned Resolver (Level 4) ─────────────────────────────────────

/// Lightweight projection of a past `LearningEpisode` for resolver use.
/// Kept here (not in `organism-learning`) so the `intent` crate stays at the
/// bottom of the dependency tree. The `learning` crate provides an adapter
/// from `LearningEpisode` to this shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSummary {
    /// Outcome description from the original intent.
    pub outcome: String,
    /// Packs that were registered for this episode's run.
    pub packs_used: Vec<String>,
    /// Whether the run reached a passing outcome.
    pub passed: bool,
}

/// Source of past episodes for learned resolution. Constructor-injected on
/// `LearnedResolver` so the resolver doesn't depend on a specific store
/// implementation.
pub trait EpisodeRecall: Send + Sync {
    fn similar_episodes(&self, intent: &super::IntentPacket) -> Vec<EpisodeSummary>;
}

/// Level 4 — Learned resolver. Looks at past episodes that match the current
/// intent and weights pack requirements by historical success rate.
///
/// For each pack that appeared in a similar passing episode, either bumps
/// confidence on an existing entry (capped at `1.0`) or adds the pack with
/// confidence proportional to its historical success rate.
pub struct LearnedResolver<R: EpisodeRecall> {
    recall: R,
    /// Bump applied to an already-bound pack's confidence per matching
    /// passing episode. Defaults to `0.05`. Capped at `1.0` after summation.
    confidence_bump: f64,
}

impl<R: EpisodeRecall> LearnedResolver<R> {
    #[must_use]
    pub fn new(recall: R) -> Self {
        Self {
            recall,
            confidence_bump: 0.05,
        }
    }

    #[must_use]
    pub fn with_confidence_bump(mut self, bump: f64) -> Self {
        self.confidence_bump = bump.clamp(0.0, 1.0);
        self
    }
}

impl<R: EpisodeRecall> IntentResolver for LearnedResolver<R> {
    fn level(&self) -> ResolutionLevel {
        ResolutionLevel::Learned
    }

    fn resolve(&self, intent: &super::IntentPacket, current: &IntentBinding) -> IntentBinding {
        let mut binding = current.clone();
        let episodes = self.recall.similar_episodes(intent);
        let consulted = episodes.len();
        if consulted == 0 {
            update_trace(&mut binding.resolution, ResolutionLevel::Learned, false);
            binding.resolution.prior_episodes_consulted += 0;
            return binding;
        }

        let total = consulted as f64;
        let passing = episodes.iter().filter(|e| e.passed).count() as f64;
        let success_rate = if total > 0.0 { passing / total } else { 0.0 };

        // Tally pack appearances across passing episodes.
        let mut pack_passing_count: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for ep in episodes.iter().filter(|e| e.passed) {
            for pack in &ep.packs_used {
                *pack_passing_count.entry(pack.clone()).or_default() += 1;
            }
        }

        let mut contributed = false;
        for (pack_name, count) in pack_passing_count {
            let weight = (count as f64 / total).clamp(0.0, 1.0);
            if let Some(existing) = binding.packs.iter_mut().find(|p| p.pack_name == pack_name) {
                let bump = self.confidence_bump * weight * (count as f64);
                existing.confidence = UnitInterval::clamped(existing.confidence.as_f64() + bump);
                contributed = true;
            } else {
                binding.packs.push(PackRequirement {
                    pack_name: pack_name.clone(),
                    reason: format!(
                        "{count} passing episode(s) used pack '{pack_name}' (success rate {success_rate:.2})",
                    ),
                    confidence: UnitInterval::clamped(weight),
                    source: ResolutionLevel::Learned,
                });
                contributed = true;
            }
        }

        update_trace(
            &mut binding.resolution,
            ResolutionLevel::Learned,
            contributed,
        );
        binding.resolution.prior_episodes_consulted += consulted;
        binding
    }
}

// ── Ladder Runner ──────────────────────────────────────────────────

/// Runs a chain of resolvers in level order, accumulating the binding.
///
/// The ladder is the public composition surface for Levels 1–4: caller hands
/// in the resolvers they want active (typically all four), the ladder runs
/// them in sequence, and the resulting `IntentBinding.resolution` carries a
/// `ResolutionTrace` reflecting everything that fired.
pub struct LadderResolver {
    resolvers: Vec<Box<dyn IntentResolver>>,
}

impl LadderResolver {
    #[must_use]
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    #[must_use]
    pub fn with(mut self, resolver: Box<dyn IntentResolver>) -> Self {
        self.resolvers.push(resolver);
        self
    }

    /// Run the ladder over the given intent, starting from `seed`.
    pub fn resolve(&self, intent: &super::IntentPacket, seed: IntentBinding) -> IntentBinding {
        let mut binding = seed;
        for resolver in &self.resolvers {
            binding = resolver.resolve(intent, &binding);
        }
        recompute_completeness(&mut binding.resolution);
        binding
    }
}

impl Default for LadderResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ── Trace helpers ──────────────────────────────────────────────────

fn update_trace(trace: &mut ResolutionTrace, level: ResolutionLevel, contributed: bool) {
    if !trace.levels_attempted.contains(&level) {
        trace.levels_attempted.push(level);
    }
    if contributed && !trace.levels_contributed.contains(&level) {
        trace.levels_contributed.push(level);
    }
}

fn recompute_completeness(trace: &mut ResolutionTrace) {
    if trace.levels_attempted.is_empty() {
        trace.completeness_confidence = UnitInterval::ZERO;
        return;
    }
    let attempted = trace.levels_attempted.len() as f64;
    let contributed = trace.levels_contributed.len() as f64;
    trace.completeness_confidence = UnitInterval::clamped(contributed / attempted);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarative_binding_builds_correctly() {
        let binding = DeclarativeBinding::new()
            .pack("customers", "lead qualification")
            .pack("linkedin_research", "enrich leads")
            .capability("web", "capture company page")
            .invariant("lead_has_source")
            .build();

        assert_eq!(binding.packs.len(), 2);
        assert_eq!(binding.capabilities.len(), 1);
        assert_eq!(binding.invariants.len(), 1);
        assert_eq!(binding.packs[0].pack_name, "customers");
        assert_eq!(binding.packs[0].source, ResolutionLevel::Declarative);
        assert!((binding.resolution.completeness_confidence.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn declarative_binding_empty() {
        let binding = DeclarativeBinding::new().build();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
        assert_eq!(
            binding.resolution.levels_attempted,
            vec![ResolutionLevel::Declarative]
        );
        assert_eq!(
            binding.resolution.levels_contributed,
            vec![ResolutionLevel::Declarative]
        );
        assert_eq!(binding.resolution.prior_episodes_consulted, 0);
    }

    #[test]
    fn declarative_binding_pack_confidence_is_one() {
        let binding = DeclarativeBinding::new().pack("test", "reason").build();
        assert!((binding.packs[0].confidence.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn declarative_binding_capability_confidence_is_one() {
        let binding = DeclarativeBinding::new()
            .capability("ocr", "doc processing")
            .build();
        assert!((binding.capabilities[0].confidence.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn declarative_binding_multiple_invariants() {
        let binding = DeclarativeBinding::new()
            .invariant("inv_a")
            .invariant("inv_b")
            .invariant("inv_c")
            .build();
        assert_eq!(binding.invariants, vec!["inv_a", "inv_b", "inv_c"]);
    }

    #[test]
    fn declarative_binding_default() {
        let binding = DeclarativeBinding::default();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
    }

    #[test]
    fn intent_binding_default() {
        let binding = IntentBinding::default();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
        assert!(binding.resolution.levels_attempted.is_empty());
        assert!(binding.resolution.levels_contributed.is_empty());
        assert_eq!(binding.resolution.prior_episodes_consulted, 0);
        assert!((binding.resolution.completeness_confidence.as_f64() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resolution_trace_default() {
        let trace = ResolutionTrace::default();
        assert!(trace.levels_attempted.is_empty());
        assert!(trace.levels_contributed.is_empty());
        assert_eq!(trace.prior_episodes_consulted, 0);
        assert!((trace.completeness_confidence.as_f64() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resolution_level_all_variants_distinct() {
        let variants = [
            ResolutionLevel::Declarative,
            ResolutionLevel::Structural,
            ResolutionLevel::Semantic,
            ResolutionLevel::Learned,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn resolution_level_serde_roundtrip() {
        for level in [
            ResolutionLevel::Declarative,
            ResolutionLevel::Structural,
            ResolutionLevel::Semantic,
            ResolutionLevel::Learned,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: ResolutionLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[test]
    fn resolution_level_snake_case() {
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Declarative).unwrap(),
            "\"declarative\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Structural).unwrap(),
            "\"structural\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Semantic).unwrap(),
            "\"semantic\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Learned).unwrap(),
            "\"learned\""
        );
    }

    #[test]
    fn pack_requirement_serde_roundtrip() {
        let req = PackRequirement {
            pack_name: "customers".into(),
            reason: "lead workflow".into(),
            confidence: UnitInterval::clamped(0.85),
            source: ResolutionLevel::Structural,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: PackRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pack_name, "customers");
        assert_eq!(back.reason, "lead workflow");
        assert!((back.confidence.as_f64() - 0.85).abs() < f64::EPSILON);
        assert_eq!(back.source, ResolutionLevel::Structural);
    }

    #[test]
    fn capability_requirement_serde_roundtrip() {
        let req = CapabilityRequirement {
            capability: "vision".into(),
            reason: "document scanning".into(),
            confidence: UnitInterval::clamped(0.7),
            source: ResolutionLevel::Semantic,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: CapabilityRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capability, "vision");
        assert_eq!(back.source, ResolutionLevel::Semantic);
    }

    #[test]
    fn intent_binding_serde_roundtrip() {
        let binding = DeclarativeBinding::new()
            .pack("dd", "due diligence")
            .capability("web", "scraping")
            .invariant("hypothesis_has_source")
            .build();

        let json = serde_json::to_string(&binding).unwrap();
        let back: IntentBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.packs.len(), 1);
        assert_eq!(back.capabilities.len(), 1);
        assert_eq!(back.invariants, vec!["hypothesis_has_source"]);
        assert_eq!(
            back.resolution.levels_attempted,
            vec![ResolutionLevel::Declarative]
        );
    }

    #[test]
    fn resolution_trace_serde_roundtrip() {
        let trace = ResolutionTrace {
            levels_attempted: vec![ResolutionLevel::Declarative, ResolutionLevel::Structural],
            levels_contributed: vec![ResolutionLevel::Declarative],
            prior_episodes_consulted: 42,
            completeness_confidence: UnitInterval::clamped(0.95),
        };
        let json = serde_json::to_string(&trace).unwrap();
        let back: ResolutionTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(back.levels_attempted.len(), 2);
        assert_eq!(back.levels_contributed.len(), 1);
        assert_eq!(back.prior_episodes_consulted, 42);
        assert!((back.completeness_confidence.as_f64() - 0.95).abs() < f64::EPSILON);
    }

    // ── Level 3: Semantic ──────────────────────────────────────────

    use chrono::{Duration, Utc};

    fn intent(outcome: &str) -> super::super::IntentPacket {
        super::super::IntentPacket::new(outcome, Utc::now() + Duration::hours(1))
    }

    struct StubMatcher(Vec<(&'static str, f64, &'static str)>);

    impl SemanticMatcher for StubMatcher {
        fn match_packs(&self, _outcome: &str) -> Vec<(String, f64, String)> {
            self.0
                .iter()
                .map(|(p, c, r)| ((*p).to_string(), *c, (*r).to_string()))
                .collect()
        }
    }

    #[test]
    fn semantic_resolver_adds_unbound_packs() {
        let matcher = StubMatcher(vec![
            ("customers", 0.7, "outcome mentions leads"),
            ("legal", 0.6, "compliance keyword detected"),
        ]);
        let resolver = SemanticResolver::new(matcher);
        let binding = resolver.resolve(&intent("qualify inbound leads"), &IntentBinding::default());

        assert_eq!(binding.packs.len(), 2);
        assert!(
            binding
                .packs
                .iter()
                .all(|p| p.source == ResolutionLevel::Semantic)
        );
        assert_eq!(
            binding.resolution.levels_contributed,
            vec![ResolutionLevel::Semantic]
        );
    }

    #[test]
    fn semantic_resolver_skips_already_bound_packs() {
        let seed = DeclarativeBinding::new()
            .pack("customers", "explicit declaration")
            .build();
        let matcher = StubMatcher(vec![("customers", 0.7, "outcome mentions leads")]);
        let resolver = SemanticResolver::new(matcher);
        let binding = resolver.resolve(&intent("qualify inbound leads"), &seed);

        // Customers stays Declarative; semantic level didn't contribute.
        assert_eq!(binding.packs.len(), 1);
        assert_eq!(binding.packs[0].source, ResolutionLevel::Declarative);
        assert!(
            binding
                .resolution
                .levels_attempted
                .contains(&ResolutionLevel::Semantic)
        );
        assert!(
            !binding
                .resolution
                .levels_contributed
                .contains(&ResolutionLevel::Semantic)
        );
    }

    #[test]
    fn semantic_resolver_clamps_confidence() {
        let matcher = StubMatcher(vec![("customers", 1.7, "out-of-range stub")]);
        let binding =
            SemanticResolver::new(matcher).resolve(&intent("anything"), &IntentBinding::default());
        assert!((binding.packs[0].confidence.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    // ── Level 4: Learned ───────────────────────────────────────────

    struct StubRecall(Vec<EpisodeSummary>);

    impl EpisodeRecall for StubRecall {
        fn similar_episodes(&self, _intent: &super::super::IntentPacket) -> Vec<EpisodeSummary> {
            self.0.clone()
        }
    }

    fn ep(outcome: &str, packs: &[&str], passed: bool) -> EpisodeSummary {
        EpisodeSummary {
            outcome: outcome.into(),
            packs_used: packs.iter().map(|p| (*p).to_string()).collect(),
            passed,
        }
    }

    #[test]
    fn learned_resolver_records_episode_count_in_trace() {
        let recall = StubRecall(vec![
            ep("a", &["customers"], true),
            ep("b", &["customers"], false),
        ]);
        let binding =
            LearnedResolver::new(recall).resolve(&intent("anything"), &IntentBinding::default());
        assert_eq!(binding.resolution.prior_episodes_consulted, 2);
    }

    #[test]
    fn learned_resolver_adds_pack_used_in_passing_episode() {
        let recall = StubRecall(vec![ep("similar", &["customers"], true)]);
        let binding =
            LearnedResolver::new(recall).resolve(&intent("anything"), &IntentBinding::default());

        let added = binding.packs.iter().find(|p| p.pack_name == "customers");
        assert!(added.is_some(), "passing-episode pack should be added");
        assert_eq!(added.unwrap().source, ResolutionLevel::Learned);
        assert!(
            binding
                .resolution
                .levels_contributed
                .contains(&ResolutionLevel::Learned)
        );
    }

    #[test]
    fn learned_resolver_skips_packs_only_in_failing_episodes() {
        let recall = StubRecall(vec![ep("similar", &["risky_pack"], false)]);
        let binding =
            LearnedResolver::new(recall).resolve(&intent("anything"), &IntentBinding::default());
        assert!(
            binding.packs.is_empty(),
            "failing-episode-only packs should not be added"
        );
        // Level was attempted but did not contribute.
        assert!(
            binding
                .resolution
                .levels_attempted
                .contains(&ResolutionLevel::Learned)
        );
        assert!(
            !binding
                .resolution
                .levels_contributed
                .contains(&ResolutionLevel::Learned)
        );
    }

    #[test]
    fn learned_resolver_bumps_confidence_on_already_bound_pack() {
        let seed = DeclarativeBinding::new()
            .pack("customers", "explicit")
            .build();
        let baseline = seed.packs[0].confidence;
        let recall = StubRecall(vec![
            ep("a", &["customers"], true),
            ep("b", &["customers"], true),
        ]);
        let binding = LearnedResolver::new(recall)
            .with_confidence_bump(0.05)
            .resolve(&intent("anything"), &seed);

        let customers = binding
            .packs
            .iter()
            .find(|p| p.pack_name == "customers")
            .expect("customers pack still present");
        assert_eq!(customers.source, ResolutionLevel::Declarative);
        // Two passing episodes, weight = 1.0, count = 2, bump = 0.05*1.0*2 = 0.1
        // baseline of 1.0 already capped, so still 1.0.
        assert!(
            customers.confidence >= baseline,
            "learned recall must not lower existing confidence"
        );
    }

    #[test]
    fn learned_resolver_no_episodes_does_not_contribute() {
        let recall = StubRecall(vec![]);
        let binding =
            LearnedResolver::new(recall).resolve(&intent("anything"), &IntentBinding::default());
        assert!(
            binding
                .resolution
                .levels_attempted
                .contains(&ResolutionLevel::Learned)
        );
        assert!(
            !binding
                .resolution
                .levels_contributed
                .contains(&ResolutionLevel::Learned)
        );
        assert_eq!(binding.resolution.prior_episodes_consulted, 0);
    }

    // ── Ladder integration ─────────────────────────────────────────

    /// Resolver that mimics the existing pack-prefix logic without depending on
    /// `runtime::Registry` (which would create a circular dep). Sufficient to
    /// prove ladder ordering and trace accumulation.
    struct StubStructural;

    impl IntentResolver for StubStructural {
        fn level(&self) -> ResolutionLevel {
            ResolutionLevel::Structural
        }

        fn resolve(
            &self,
            intent: &super::super::IntentPacket,
            current: &IntentBinding,
        ) -> IntentBinding {
            let mut binding = current.clone();
            let already: std::collections::HashSet<String> =
                binding.packs.iter().map(|p| p.pack_name.clone()).collect();
            let mut contributed = false;
            // Trivial structural rule: outcome mentioning "vendor" → procurement pack.
            if intent.outcome.to_lowercase().contains("vendor") && !already.contains("procurement")
            {
                binding.packs.push(PackRequirement {
                    pack_name: "procurement".into(),
                    reason: "outcome mentions 'vendor'".into(),
                    confidence: UnitInterval::clamped(0.9),
                    source: ResolutionLevel::Structural,
                });
                contributed = true;
            }
            update_trace(
                &mut binding.resolution,
                ResolutionLevel::Structural,
                contributed,
            );
            binding
        }
    }

    #[test]
    fn ladder_runs_all_four_levels_and_records_each() {
        let seed = DeclarativeBinding::new()
            .pack("customers", "explicit")
            .build();

        let semantic =
            SemanticResolver::new(StubMatcher(vec![("legal", 0.6, "compliance keyword")]));
        let learned = LearnedResolver::new(StubRecall(vec![ep(
            "vendor selection for ACME",
            &["customers", "partnerships"],
            true,
        )]));

        let ladder = LadderResolver::new()
            .with(Box::new(StubStructural))
            .with(Box::new(semantic))
            .with(Box::new(learned));

        let binding = ladder.resolve(&intent("vendor selection for ACME"), seed);

        // Level 1 declarative seed.
        assert!(
            binding
                .packs
                .iter()
                .any(|p| p.pack_name == "customers" && p.source == ResolutionLevel::Declarative)
        );
        // Level 2 added procurement (vendor → procurement).
        assert!(
            binding
                .packs
                .iter()
                .any(|p| p.pack_name == "procurement" && p.source == ResolutionLevel::Structural)
        );
        // Level 3 added legal.
        assert!(
            binding
                .packs
                .iter()
                .any(|p| p.pack_name == "legal" && p.source == ResolutionLevel::Semantic)
        );
        // Level 4 added partnerships from prior episode.
        assert!(
            binding
                .packs
                .iter()
                .any(|p| p.pack_name == "partnerships" && p.source == ResolutionLevel::Learned)
        );

        // Trace shows all four levels attempted and contributing.
        for level in [
            ResolutionLevel::Declarative,
            ResolutionLevel::Structural,
            ResolutionLevel::Semantic,
            ResolutionLevel::Learned,
        ] {
            assert!(
                binding.resolution.levels_attempted.contains(&level),
                "level {level:?} should be in levels_attempted"
            );
            assert!(
                binding.resolution.levels_contributed.contains(&level),
                "level {level:?} should be in levels_contributed"
            );
        }

        // Prior episodes counted.
        assert_eq!(binding.resolution.prior_episodes_consulted, 1);

        // All-attempted, all-contributing → completeness = 1.0.
        assert!((binding.resolution.completeness_confidence.as_f64() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ladder_completeness_reflects_partial_contribution() {
        // No semantic matches, no recall — only the declarative seed contributes.
        // Trace accumulates across seed + ladder: D (seed) attempted+contributed,
        // S (ladder) attempted only, L (ladder) attempted only.
        let seed = DeclarativeBinding::new()
            .pack("customers", "explicit")
            .build();
        let ladder = LadderResolver::new()
            .with(Box::new(SemanticResolver::new(StubMatcher(vec![]))))
            .with(Box::new(LearnedResolver::new(StubRecall(vec![]))));
        let binding = ladder.resolve(&intent("anything"), seed);

        assert_eq!(binding.resolution.levels_attempted.len(), 3);
        assert_eq!(binding.resolution.levels_contributed.len(), 1);
        let expected = 1.0 / 3.0;
        assert!(
            (binding.resolution.completeness_confidence.as_f64() - expected).abs() < f64::EPSILON,
            "completeness should reflect 1 of 3 levels contributing; got {}",
            binding.resolution.completeness_confidence.as_f64()
        );
    }
}
