//! Adversarial vocabulary.
//!
//! Types for institutionalized disagreement: challenges, findings, signals.
//! Adversarial agents are Suggestors — they participate in the convergence
//! loop alongside planners, simulators, and policy gates.
//!
//! The debate cycle is natural convergence: planning proposes → adversaries
//! challenge (via `ContextKey::Constraints`) → planning revises → repeat.
//! Converge's fixed-point detection handles termination.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Challenge ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: Uuid,
    pub kind: SkepticismKind,
    pub target_plan: Uuid,
    pub description: String,
    pub severity: Severity,
    pub evidence: Vec<String>,
    pub suggestion: Option<String>,
}

impl Challenge {
    pub fn new(
        kind: SkepticismKind,
        target_plan: Uuid,
        description: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            target_plan,
            description: description.into(),
            severity,
            evidence: Vec::new(),
            suggestion: None,
        }
    }

    pub fn is_blocking(&self) -> bool {
        self.severity == Severity::Blocker
    }
}

// ── Skepticism Taxonomy ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkepticismKind {
    AssumptionBreaking,
    ConstraintChecking,
    CausalSkepticism,
    EconomicSkepticism,
    OperationalSkepticism,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Advisory,
    Warning,
    Blocker,
}

// ── Finding (simplified challenge for reporting) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub agent: String,
    pub severity: Severity,
    pub message: String,
}

// ── Adversarial Signal (training data for learning system) ─────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialSignal {
    pub kind: SkepticismKind,
    pub failed_assumption: String,
    pub context: serde_json::Value,
    pub revision_summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn plan_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    #[test]
    fn challenge_new_sets_defaults() {
        let c = Challenge::new(
            SkepticismKind::CausalSkepticism,
            plan_id(),
            "bad assumption",
            Severity::Warning,
        );
        assert_eq!(c.kind, SkepticismKind::CausalSkepticism);
        assert_eq!(c.target_plan, plan_id());
        assert_eq!(c.description, "bad assumption");
        assert_eq!(c.severity, Severity::Warning);
        assert!(c.evidence.is_empty());
        assert!(c.suggestion.is_none());
    }

    #[test]
    fn challenge_new_generates_unique_ids() {
        let a = Challenge::new(
            SkepticismKind::AssumptionBreaking,
            plan_id(),
            "",
            Severity::Advisory,
        );
        let b = Challenge::new(
            SkepticismKind::AssumptionBreaking,
            plan_id(),
            "",
            Severity::Advisory,
        );
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn is_blocking_only_for_blocker() {
        let blocker = Challenge::new(
            SkepticismKind::ConstraintChecking,
            plan_id(),
            "stop",
            Severity::Blocker,
        );
        let warning = Challenge::new(
            SkepticismKind::ConstraintChecking,
            plan_id(),
            "maybe",
            Severity::Warning,
        );
        let advisory = Challenge::new(
            SkepticismKind::ConstraintChecking,
            plan_id(),
            "fyi",
            Severity::Advisory,
        );
        assert!(blocker.is_blocking());
        assert!(!warning.is_blocking());
        assert!(!advisory.is_blocking());
    }

    #[test]
    fn challenge_new_accepts_string_and_str() {
        let from_str = Challenge::new(
            SkepticismKind::EconomicSkepticism,
            plan_id(),
            "lit",
            Severity::Advisory,
        );
        let from_string = Challenge::new(
            SkepticismKind::EconomicSkepticism,
            plan_id(),
            String::from("owned"),
            Severity::Advisory,
        );
        assert_eq!(from_str.description, "lit");
        assert_eq!(from_string.description, "owned");
    }

    #[test]
    fn challenge_new_empty_description() {
        let c = Challenge::new(
            SkepticismKind::OperationalSkepticism,
            plan_id(),
            "",
            Severity::Advisory,
        );
        assert_eq!(c.description, "");
    }

    #[test]
    fn skepticism_kind_all_variants_distinct() {
        let variants = [
            SkepticismKind::AssumptionBreaking,
            SkepticismKind::ConstraintChecking,
            SkepticismKind::CausalSkepticism,
            SkepticismKind::EconomicSkepticism,
            SkepticismKind::OperationalSkepticism,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn severity_all_variants_distinct() {
        let variants = [Severity::Advisory, Severity::Warning, Severity::Blocker];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn challenge_serde_roundtrip() {
        let mut c = Challenge::new(
            SkepticismKind::EconomicSkepticism,
            plan_id(),
            "too expensive",
            Severity::Blocker,
        );
        c.evidence = vec!["cost +40%".into()];
        c.suggestion = Some("reduce scope".into());

        let json = serde_json::to_string(&c).unwrap();
        let back: Challenge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, c.id);
        assert_eq!(back.kind, c.kind);
        assert_eq!(back.description, c.description);
        assert_eq!(back.severity, c.severity);
        assert_eq!(back.evidence, c.evidence);
        assert_eq!(back.suggestion, c.suggestion);
    }

    #[test]
    fn finding_serde_roundtrip() {
        let f = Finding {
            agent: "economic-skeptic".into(),
            severity: Severity::Warning,
            message: "budget overrun".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.agent, f.agent);
        assert_eq!(back.message, f.message);
    }

    #[test]
    fn adversarial_signal_serde_roundtrip() {
        let s = AdversarialSignal {
            kind: SkepticismKind::CausalSkepticism,
            failed_assumption: "X causes Y".into(),
            context: serde_json::json!({"key": "value"}),
            revision_summary: Some("added control".into()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: AdversarialSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, s.kind);
        assert_eq!(back.failed_assumption, s.failed_assumption);
        assert_eq!(back.context, s.context);
        assert_eq!(back.revision_summary, s.revision_summary);
    }

    #[test]
    fn adversarial_signal_none_revision() {
        let s = AdversarialSignal {
            kind: SkepticismKind::AssumptionBreaking,
            failed_assumption: "assumption".into(),
            context: serde_json::json!(null),
            revision_summary: None,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: AdversarialSignal = serde_json::from_str(&json).unwrap();
        assert!(back.revision_summary.is_none());
    }

    #[test]
    fn skepticism_kind_serde_snake_case() {
        let json = serde_json::to_string(&SkepticismKind::AssumptionBreaking).unwrap();
        assert_eq!(json, "\"assumption_breaking\"");
        let json = serde_json::to_string(&SkepticismKind::CausalSkepticism).unwrap();
        assert_eq!(json, "\"causal_skepticism\"");
    }

    #[test]
    fn severity_serde_snake_case() {
        let json = serde_json::to_string(&Severity::Blocker).unwrap();
        assert_eq!(json, "\"blocker\"");
        let json = serde_json::to_string(&Severity::Advisory).unwrap();
        assert_eq!(json, "\"advisory\"");
    }

    proptest! {
        #[test]
        fn challenge_never_panics_on_arbitrary_description(desc in ".*") {
            let c = Challenge::new(
                SkepticismKind::OperationalSkepticism,
                plan_id(),
                desc.clone(),
                Severity::Advisory,
            );
            prop_assert_eq!(c.description, desc);
        }

        #[test]
        fn challenge_blocking_iff_blocker(sev in prop_oneof![
            Just(Severity::Advisory),
            Just(Severity::Warning),
            Just(Severity::Blocker),
        ]) {
            let c = Challenge::new(SkepticismKind::AssumptionBreaking, plan_id(), "x", sev);
            prop_assert_eq!(c.is_blocking(), sev == Severity::Blocker);
        }
    }
}
