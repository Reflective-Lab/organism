//! Huddle invocation — the request that asks for a huddle.
//!
//! `Huddle` (in `huddle.rs`) is the *executor*: it runs a set of `Reasoner`s
//! against an `IntentPacket`. `HuddleInvocation` is the *invocation packet*:
//! a structured "convene a huddle on subject X because of Y, at urgency Z".
//!
//! The envelope is domain-agnostic. Triggers are free-form strings so each
//! consumer can encode its own taxonomy without touching this crate. Strongly
//! typed domain triggers can be JSON-serialised into `domain_context`, which
//! mirrors the `AdversarialSignal::context` pattern in `organism-adversarial`.
//!
//! Classification rules — "is this brief contested? sensitive? high-risk?" —
//! depend on domain inputs (claim records, drafts, trust labels, …) and so
//! stay in the consuming crate. Organism only owns the envelope.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddleInvocationKind {
    /// Subject carries internal contradiction or contested verification.
    Contested,
    /// Subject involves sensitive content that demands editorial care.
    Sensitive,
    /// Subject has unresolved gaps or open obligations that elevate risk.
    HighRisk,
    /// Subject was produced or materially shaped by AI assistance.
    AiAssisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuddleUrgency {
    Routine,
    Elevated,
    Urgent,
}

/// Domain-agnostic huddle invocation envelope.
///
/// `subject_id` identifies whatever the huddle is about (a story, a plan,
/// a decision, …). `triggers` is a free-form list of human-readable reason
/// labels. `domain_context` carries optional structured domain data (e.g.
/// strongly typed trigger enums serialised to JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HuddleInvocation {
    pub id: String,
    pub subject_id: String,
    pub kind: HuddleInvocationKind,
    pub urgency: HuddleUrgency,
    pub triggers: Vec<String>,
    pub rationale: String,
    pub correlation_id: String,
    pub reviewer: String,
    #[serde(default, skip_serializing_if = "is_null")]
    pub domain_context: serde_json::Value,
}

fn is_null(v: &serde_json::Value) -> bool {
    v.is_null()
}

impl HuddleInvocation {
    pub fn new(
        subject_id: impl Into<String>,
        kind: HuddleInvocationKind,
        urgency: HuddleUrgency,
        correlation_id: impl Into<String>,
    ) -> Self {
        let subject = subject_id.into();
        Self {
            id: format!("huddle:{subject}"),
            subject_id: subject,
            kind,
            urgency,
            triggers: Vec::new(),
            rationale: String::new(),
            correlation_id: correlation_id.into(),
            reviewer: String::new(),
            domain_context: serde_json::Value::Null,
        }
    }

    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    #[must_use]
    pub fn with_triggers(mut self, triggers: Vec<String>) -> Self {
        self.triggers = triggers;
        self
    }

    #[must_use]
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = rationale.into();
        self
    }

    #[must_use]
    pub fn with_reviewer(mut self, reviewer: impl Into<String>) -> Self {
        self.reviewer = reviewer.into();
        self
    }

    #[must_use]
    pub fn with_domain_context(mut self, context: serde_json::Value) -> Self {
        self.domain_context = context;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_default_id_and_empty_fields() {
        let inv = HuddleInvocation::new(
            "story-001",
            HuddleInvocationKind::Contested,
            HuddleUrgency::Elevated,
            "corr-1",
        );
        assert_eq!(inv.id, "huddle:story-001");
        assert_eq!(inv.subject_id, "story-001");
        assert_eq!(inv.kind, HuddleInvocationKind::Contested);
        assert_eq!(inv.urgency, HuddleUrgency::Elevated);
        assert_eq!(inv.correlation_id, "corr-1");
        assert!(inv.triggers.is_empty());
        assert!(inv.rationale.is_empty());
        assert!(inv.reviewer.is_empty());
        assert!(inv.domain_context.is_null());
    }

    #[test]
    fn builder_chain_sets_fields() {
        let inv = HuddleInvocation::new(
            "story-001",
            HuddleInvocationKind::Sensitive,
            HuddleUrgency::Urgent,
            "corr-1",
        )
        .with_id("huddle:custom")
        .with_triggers(vec!["sensitive-subject".into(), "policy/x".into()])
        .with_rationale("sensitive keyword in title")
        .with_reviewer("organism-huddle")
        .with_domain_context(serde_json::json!({"tag": "newspaper"}));
        assert_eq!(inv.id, "huddle:custom");
        assert_eq!(inv.triggers.len(), 2);
        assert_eq!(inv.rationale, "sensitive keyword in title");
        assert_eq!(inv.reviewer, "organism-huddle");
        assert_eq!(inv.domain_context["tag"], "newspaper");
    }

    #[test]
    fn serde_round_trip_uses_snake_case() {
        let inv = HuddleInvocation::new(
            "story-001",
            HuddleInvocationKind::AiAssisted,
            HuddleUrgency::Routine,
            "corr-1",
        );
        let json = serde_json::to_string(&inv).expect("serialize");
        assert!(json.contains("\"kind\":\"ai_assisted\""), "got {json}");
        assert!(json.contains("\"urgency\":\"routine\""), "got {json}");
        let back: HuddleInvocation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, inv);
    }

    #[test]
    fn null_domain_context_is_omitted_from_serialization() {
        let inv = HuddleInvocation::new(
            "story-001",
            HuddleInvocationKind::HighRisk,
            HuddleUrgency::Elevated,
            "corr-1",
        );
        let json = serde_json::to_string(&inv).expect("serialize");
        assert!(!json.contains("domain_context"), "got {json}");
    }

    #[test]
    fn populated_domain_context_round_trips() {
        let inv = HuddleInvocation::new(
            "story-001",
            HuddleInvocationKind::HighRisk,
            HuddleUrgency::Elevated,
            "corr-1",
        )
        .with_domain_context(serde_json::json!({"adversarial_triggers": ["foo", "bar"]}));
        let json = serde_json::to_string(&inv).expect("serialize");
        let back: HuddleInvocation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.domain_context["adversarial_triggers"][0], "foo");
        assert_eq!(back, inv);
    }

    #[test]
    fn all_kind_variants_serialize() {
        for kind in [
            HuddleInvocationKind::Contested,
            HuddleInvocationKind::Sensitive,
            HuddleInvocationKind::HighRisk,
            HuddleInvocationKind::AiAssisted,
        ] {
            let json = serde_json::to_string(&kind).expect("serialize");
            let back: HuddleInvocationKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn all_urgency_variants_serialize() {
        for urgency in [
            HuddleUrgency::Routine,
            HuddleUrgency::Elevated,
            HuddleUrgency::Urgent,
        ] {
            let json = serde_json::to_string(&urgency).expect("serialize");
            let back: HuddleUrgency = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(urgency, back);
        }
    }
}
