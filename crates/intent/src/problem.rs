//! Problem classification.
//!
//! Coarse taxonomy of organizational problems. The classifier takes an
//! [`IntentPacket`](crate::IntentPacket) and tags it with one of seven classes,
//! which the Formation Guru uses to narrow down candidate formation templates.
//!
//! The deterministic classifier here matches keywords in the intent's outcome
//! and known entities; it is intentionally cheap and biased toward returning
//! something reasonable. Ambiguous intents fall back to [`ProblemClass::Decision`]
//! since most multi-Suggestor work in Organism is decision-shaped. An LLM
//! tiebreaker is the natural follow-on for genuinely ambiguous cases — out of
//! scope here.

use serde::{Deserialize, Serialize};

use crate::IntentPacket;

/// Coarse taxonomy of organizational problems Organism handles.
///
/// The seven classes cover most business problem shapes. They are *not*
/// mutually exclusive in practice — a "vendor selection" intent is both a
/// Decision and a Diligence problem — but the classifier picks a single
/// dominant class so the Formation Guru can route to one template per run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemClass {
    /// Pick one option from a finite candidate set ("approve this expense",
    /// "select this vendor", "choose between two strategies").
    Decision,
    /// Open-ended fact-finding ("research the competitive landscape",
    /// "investigate why churn rose", "explore options").
    Research,
    /// Score / rank / compare against criteria ("evaluate vendor proposals",
    /// "rate candidate performance").
    Evaluation,
    /// Forward-looking sequencing ("plan the Q3 launch", "schedule the
    /// migration", "design the rollout").
    Planning,
    /// Adversarial fact-gathering with a verdict ("vet this acquisition
    /// target", "audit the contract", "verify these claims").
    Diligence,
    /// Time-pressured stabilization ("fix the prod outage", "respond to the
    /// incident", "resolve the breach").
    Incident,
    /// Long-horizon framing ("set our 3-year strategy", "define the vision",
    /// "frame the market position").
    Strategy,
}

impl ProblemClass {
    /// Stable string name used in fact payloads and log lines.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Research => "research",
            Self::Evaluation => "evaluation",
            Self::Planning => "planning",
            Self::Diligence => "diligence",
            Self::Incident => "incident",
            Self::Strategy => "strategy",
        }
    }
}

impl std::fmt::Display for ProblemClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Result of classifying an intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemClassification {
    pub class: ProblemClass,
    /// The trigger words that fired to produce this class. Empty if the
    /// classifier fell back to the default. Useful for traces and
    /// explanations.
    pub matched_keywords: Vec<String>,
    /// True if no keyword matched and the classifier defaulted (without a
    /// tiebreaker resolving the ambiguity).
    pub defaulted: bool,
    /// True if a [`ClassifierTiebreaker`] was consulted to resolve an
    /// ambiguous keyword pass. A consumer that audits selections may prefer
    /// to surface tiebroken classifications differently from
    /// keyword-driven ones (lower confidence, may flip on retry).
    #[serde(default)]
    pub tiebroken: bool,
}

/// Plug-Boundary trait for LLM-backed (or otherwise external) tiebreakers
/// the keyword classifier can defer to on ambiguous intents. Implementations
/// live in host code (e.g. axiom or the application) so organism stays free
/// of vendor adapter imports — same shape as the SemanticMatcher trait at
/// resolution-ladder Level 3.
#[async_trait::async_trait]
pub trait ClassifierTiebreaker: Send + Sync {
    /// Given the text the keyword classifier could not classify, commit to
    /// one of the seven [`ProblemClass`] values. Implementations are
    /// expected to always pick a class — but errors are advisory; the
    /// caller falls back to the deterministic default on any failure.
    ///
    /// # Errors
    ///
    /// `TiebreakerError::Unavailable` — backend offline / no quota / etc.
    /// `TiebreakerError::Other` — anything else.
    async fn break_tie(&self, text: &str) -> Result<ProblemClass, TiebreakerError>;
}

/// Why a tiebreaker couldn't decide.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TiebreakerError {
    #[error("classifier tiebreaker unavailable")]
    Unavailable,
    #[error("classifier tiebreaker failed: {0}")]
    Other(String),
}

/// Deterministic keyword-based classifier for an [`IntentPacket`].
///
/// Looks at the intent's `outcome` text plus any context-key prefixes the
/// caller exposes via the intent's `constraints` and `forbidden` lists and
/// returns the dominant [`ProblemClass`].
///
/// Ranking is by class-specific keyword count: the class whose keywords match
/// the most distinct words wins. Ties break in the order:
/// `Incident → Diligence → Evaluation → Decision → Research → Planning →
/// Strategy`. Incidents win ties because misclassifying a stabilization
/// problem as anything else is the most expensive error.
#[must_use]
pub fn classify(intent: &IntentPacket) -> ProblemClassification {
    classify_text(&build_haystack(intent))
}

/// Classify a free-form text blob (e.g. concatenated Seed contents pulled
/// from convergence context). Used by `ProblemClassifierSuggestor`, which sees
/// `ProposedFact` content strings rather than typed `IntentPacket`s.
#[must_use]
pub fn classify_text(haystack: &str) -> ProblemClassification {
    let words = tokenize(haystack);

    let mut hits: Vec<(ProblemClass, Vec<String>)> = Vec::new();
    for class in ALL_CLASSES {
        let keywords = class_keywords(class);
        let matched: Vec<String> = words
            .iter()
            .filter(|w| keywords.iter().any(|k| word_matches(w, k)))
            .cloned()
            .collect();
        if !matched.is_empty() {
            hits.push((class, matched));
        }
    }

    if hits.is_empty() {
        return ProblemClassification {
            class: ProblemClass::Decision,
            matched_keywords: Vec::new(),
            defaulted: true,
            tiebroken: false,
        };
    }

    // Highest match count wins; ties go to the class earliest in TIE_ORDER.
    hits.sort_by(|a, b| {
        let by_count = b.1.len().cmp(&a.1.len());
        if by_count.is_eq() {
            tie_rank(a.0).cmp(&tie_rank(b.0))
        } else {
            by_count
        }
    });

    let (class, matched) = hits.into_iter().next().expect("non-empty");
    ProblemClassification {
        class,
        matched_keywords: matched,
        defaulted: false,
        tiebroken: false,
    }
}

/// Classify with optional LLM tiebreaker fallback. The deterministic keyword
/// pass runs first; only if it defaults (no keyword matched) does the
/// tiebreaker get consulted. On tiebreaker error, the result falls back to
/// the deterministic default — degraded but never absent.
///
/// Hosts use this when ambiguous truths arrive often enough to justify the
/// LLM round-trip cost. For deterministic-only classification (no LLM
/// dependency, instant), call [`classify`] directly.
pub async fn classify_with_tiebreaker<T: ClassifierTiebreaker + ?Sized>(
    intent: &IntentPacket,
    tiebreaker: &T,
) -> ProblemClassification {
    classify_text_with_tiebreaker(&build_haystack(intent), tiebreaker).await
}

/// Free-text variant of [`classify_with_tiebreaker`]. Used by Suggestors
/// that read seed text out of context.
pub async fn classify_text_with_tiebreaker<T: ClassifierTiebreaker + ?Sized>(
    haystack: &str,
    tiebreaker: &T,
) -> ProblemClassification {
    let initial = classify_text(haystack);
    if !initial.defaulted {
        return initial;
    }
    match tiebreaker.break_tie(haystack).await {
        Ok(class) => ProblemClassification {
            class,
            matched_keywords: vec![format!("tiebreaker:{class}")],
            defaulted: false,
            tiebroken: true,
        },
        Err(_) => initial, // degraded: keep the default
    }
}

const ALL_CLASSES: [ProblemClass; 7] = [
    ProblemClass::Decision,
    ProblemClass::Research,
    ProblemClass::Evaluation,
    ProblemClass::Planning,
    ProblemClass::Diligence,
    ProblemClass::Incident,
    ProblemClass::Strategy,
];

const TIE_ORDER: [ProblemClass; 7] = [
    ProblemClass::Incident,
    ProblemClass::Diligence,
    ProblemClass::Evaluation,
    ProblemClass::Decision,
    ProblemClass::Research,
    ProblemClass::Planning,
    ProblemClass::Strategy,
];

fn tie_rank(class: ProblemClass) -> usize {
    TIE_ORDER
        .iter()
        .position(|c| *c == class)
        .unwrap_or(usize::MAX)
}

fn class_keywords(class: ProblemClass) -> &'static [&'static str] {
    match class {
        ProblemClass::Decision => &[
            "decide",
            "decision",
            "select",
            "selection",
            "choose",
            "choice",
            "pick",
            "approve",
            "approval",
            "reject",
            "rejection",
        ],
        ProblemClass::Research => &[
            "research",
            "investigate",
            "investigation",
            "explore",
            "exploration",
            "discover",
            "find",
            "study",
            "learn",
            "survey",
        ],
        ProblemClass::Evaluation => &[
            "evaluate",
            "evaluation",
            "assess",
            "assessment",
            "score",
            "rank",
            "rating",
            "rate",
            "compare",
            "comparison",
            "benchmark",
            "review",
        ],
        ProblemClass::Planning => &[
            "plan",
            "planning",
            "schedule",
            "scheduling",
            "design",
            "prepare",
            "organize",
            "structure",
            "roadmap-execution",
            "rollout",
            "sequence",
        ],
        ProblemClass::Diligence => &[
            "diligence",
            "due-diligence",
            "vet",
            "audit",
            "verify",
            "verification",
            "validate",
            "validation",
            "qualify",
            "qualification",
            "background-check",
        ],
        ProblemClass::Incident => &[
            "incident",
            "outage",
            "issue",
            "bug",
            "fix",
            "resolve",
            "emergency",
            "urgent",
            "stabilize",
            "remediate",
            "rollback",
            "respond",
        ],
        ProblemClass::Strategy => &[
            "strategy",
            "strategic",
            "vision",
            "roadmap",
            "long-term",
            "direction",
            "positioning",
            "market-position",
            "framing",
        ],
    }
}

fn build_haystack(intent: &IntentPacket) -> String {
    let mut buf = intent.outcome.clone();
    for c in &intent.constraints {
        buf.push(' ');
        buf.push_str(c);
    }
    for f in &intent.forbidden {
        buf.push(' ');
        buf.push_str(&f.action);
    }
    if let Some(s) = intent.context.as_str() {
        buf.push(' ');
        buf.push_str(s);
    }
    buf
}

fn tokenize(haystack: &str) -> Vec<String> {
    haystack
        .to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

fn word_matches(word: &str, keyword: &str) -> bool {
    word == keyword || word.starts_with(keyword) || keyword.starts_with(word) && word.len() >= 4
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn intent(outcome: &str) -> IntentPacket {
        IntentPacket::new(outcome, Utc::now() + Duration::hours(1))
    }

    #[test]
    fn decision_keyword_matches() {
        let i = intent("decide which vendor to approve");
        let r = classify(&i);
        assert_eq!(r.class, ProblemClass::Decision);
        assert!(!r.defaulted);
    }

    #[test]
    fn research_keyword_matches() {
        let i = intent("research the competitive landscape for Q3");
        assert_eq!(classify(&i).class, ProblemClass::Research);
    }

    #[test]
    fn evaluation_keyword_matches() {
        let i = intent("evaluate vendor proposals against the rubric");
        assert_eq!(classify(&i).class, ProblemClass::Evaluation);
    }

    #[test]
    fn planning_keyword_matches() {
        let i = intent("plan the Q3 launch sequence");
        assert_eq!(classify(&i).class, ProblemClass::Planning);
    }

    #[test]
    fn diligence_keyword_matches() {
        let i = intent("vet the acquisition target end-to-end");
        assert_eq!(classify(&i).class, ProblemClass::Diligence);
    }

    #[test]
    fn incident_keyword_matches() {
        let i = intent("respond to the prod outage and stabilize");
        assert_eq!(classify(&i).class, ProblemClass::Incident);
    }

    #[test]
    fn strategy_keyword_matches() {
        let i = intent("set our three-year strategic direction");
        assert_eq!(classify(&i).class, ProblemClass::Strategy);
    }

    #[test]
    fn empty_outcome_defaults_to_decision() {
        let i = intent("doing the thing");
        let r = classify(&i);
        assert_eq!(r.class, ProblemClass::Decision);
        assert!(r.defaulted);
        assert!(r.matched_keywords.is_empty());
    }

    #[test]
    fn incident_wins_tie_against_decision() {
        let i = intent("decide how to respond to the outage");
        // both Decision (decide) and Incident (respond, outage) match;
        // Incident has more matches AND wins ties anyway.
        assert_eq!(classify(&i).class, ProblemClass::Incident);
    }

    #[test]
    fn diligence_wins_over_research_when_keywords_co_occur() {
        let i = intent("vet and research the new partner");
        let r = classify(&i);
        // Both classes match; with equal counts, Diligence wins by tie rank.
        // (Misclassifying a vetting workflow as Research would skip the
        // adversarial verdict, which is the point of Diligence templates.)
        assert_eq!(r.class, ProblemClass::Diligence);
    }

    #[test]
    fn matched_keywords_recorded() {
        let i = intent("evaluate and rank the vendor proposals");
        let r = classify(&i);
        assert_eq!(r.class, ProblemClass::Evaluation);
        assert!(r.matched_keywords.iter().any(|w| w == "evaluate"));
        assert!(r.matched_keywords.iter().any(|w| w == "rank"));
    }

    #[test]
    fn constraints_and_forbidden_contribute_to_classification() {
        let mut i = intent("ship the thing");
        i.constraints = vec!["audit trail required".into()];
        // "audit" is a Diligence keyword; "ship" alone wouldn't match anything.
        assert_eq!(classify(&i).class, ProblemClass::Diligence);
    }

    #[test]
    fn problem_class_serde_snake_case() {
        let s = serde_json::to_string(&ProblemClass::Diligence).unwrap();
        assert_eq!(s, "\"diligence\"");
        let back: ProblemClass = serde_json::from_str("\"incident\"").unwrap();
        assert_eq!(back, ProblemClass::Incident);
    }

    #[test]
    fn problem_class_display_matches_as_str() {
        for class in ALL_CLASSES {
            assert_eq!(class.to_string(), class.as_str());
        }
    }

    // ── Tiebreaker tests ─────────────────────────────────────────────

    struct StubTiebreaker {
        class: ProblemClass,
    }

    #[async_trait::async_trait]
    impl ClassifierTiebreaker for StubTiebreaker {
        async fn break_tie(&self, _text: &str) -> Result<ProblemClass, TiebreakerError> {
            Ok(self.class)
        }
    }

    struct UnavailableTiebreaker;

    #[async_trait::async_trait]
    impl ClassifierTiebreaker for UnavailableTiebreaker {
        async fn break_tie(&self, _text: &str) -> Result<ProblemClass, TiebreakerError> {
            Err(TiebreakerError::Unavailable)
        }
    }

    #[tokio::test]
    async fn tiebreaker_invoked_only_when_keyword_pass_defaulted() {
        let tb = StubTiebreaker {
            class: ProblemClass::Strategy,
        };
        // Clear keyword match — tiebreaker NOT consulted.
        let i = intent("evaluate the proposal carefully");
        let r = classify_with_tiebreaker(&i, &tb).await;
        assert_eq!(r.class, ProblemClass::Evaluation);
        assert!(!r.tiebroken);
    }

    #[tokio::test]
    async fn tiebreaker_resolves_ambiguous_classification() {
        let tb = StubTiebreaker {
            class: ProblemClass::Strategy,
        };
        let i = intent("doing the thing today");
        let r = classify_with_tiebreaker(&i, &tb).await;
        assert_eq!(r.class, ProblemClass::Strategy);
        assert!(!r.defaulted);
        assert!(r.tiebroken);
        assert!(
            r.matched_keywords
                .iter()
                .any(|k| k.starts_with("tiebreaker:"))
        );
    }

    #[tokio::test]
    async fn tiebreaker_failure_falls_back_to_default() {
        let tb = UnavailableTiebreaker;
        let i = intent("doing the thing today");
        let r = classify_with_tiebreaker(&i, &tb).await;
        assert_eq!(r.class, ProblemClass::Decision);
        assert!(r.defaulted);
        assert!(!r.tiebroken);
    }
}
