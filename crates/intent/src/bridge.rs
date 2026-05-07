//! Truth → IntentPacket bridge.
//!
//! Compiles a structured Truth-shaped input into a runtime [`IntentPacket`].
//! Replaces the hand-rolled construction path that Helms currently uses
//! through `helms/truth-catalog/src/organism.rs`.
//!
//! Organism does not parse `.truths` source — that lives upstream (e.g. in
//! `axiom-truth`). Consumers parse a Truth document with whatever toolchain
//! they prefer, populate a [`TruthInput`] (or build one from constants), and
//! call [`compile_truth`]. The fields on [`TruthInput`] mirror the canonical
//! Truth Document governance shape so the consumer adapter is field-by-field
//! trivial.
//!
//! See `kb/Concepts/Intent Resolution.md` for the resolver ladder this binding
//! feeds, and `kb/Concepts/Bidirectional ExperienceStore.md` for the user-side
//! events that influence resolution downstream.

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::{ExpiryAction, ForbiddenAction, IntentPacket, Reversibility};

/// Errors produced when compiling a [`TruthInput`] into an [`IntentPacket`].
#[derive(Debug, Clone, thiserror::Error)]
pub enum BridgeError {
    /// The `intent` block is missing or has no outcome/goal text. An
    /// IntentPacket needs a non-empty outcome to drive resolution.
    #[error("truth input has no intent outcome or goal")]
    MissingOutcome,

    /// The `authority.expires` field was present but could not be parsed as
    /// an RFC-3339 timestamp or `YYYY-MM-DD` date.
    #[error("could not parse authority.expires '{value}': {message}")]
    ExpiryParse { value: String, message: String },
}

/// Default expiry window applied when the input doesn't specify one. Intents
/// without an explicit deadline get one day; the runtime can re-issue the
/// IntentPacket if the work outlives that window.
const DEFAULT_EXPIRY_HOURS: i64 = 24;

// ── Truth input shape ──────────────────────────────────────────────
//
// The fields mirror the canonical Truth Document governance shape used by
// `axiom-truth` so adapters are trivial. Organism does NOT depend on
// `axiom-truth`; consumers parse Truth source with their own toolchain and
// hand the result here.

/// Structured input the bridge compiles into an [`IntentPacket`].
///
/// All blocks are optional — only the `intent` block (with a non-empty
/// `outcome` or `goal`) is required for compilation to succeed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruthInput {
    pub intent: Option<IntentBlock>,
    pub authority: Option<AuthorityBlock>,
    pub constraint: Option<ConstraintBlock>,
    pub evidence: Option<EvidenceBlock>,
    pub exception: Option<ExceptionBlock>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentBlock {
    pub outcome: Option<String>,
    pub goal: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityBlock {
    pub actor: Option<String>,
    pub may: Vec<String>,
    pub must_not: Vec<String>,
    pub requires_approval: Vec<String>,
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintBlock {
    pub budget: Vec<String>,
    pub cost_limit: Vec<String>,
    pub must_not: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBlock {
    pub requires: Vec<String>,
    pub provenance: Vec<String>,
    pub audit: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExceptionBlock {
    pub escalates_to: Vec<String>,
    pub requires: Vec<String>,
}

// ── Bridge ─────────────────────────────────────────────────────────

/// Compile a [`TruthInput`] into an [`IntentPacket`].
///
/// Field mapping (input block → IntentPacket field):
/// - `intent.outcome` (or `intent.goal` as fallback) → `outcome`
/// - `authority.may` → `authority`
/// - `authority.must_not` ⊕ `constraint.must_not` → `forbidden`
///   (deduplicated; authority entries get an `authority` reason, constraint
///   entries get a `constraint` reason)
/// - `authority.requires_approval` → folded into `constraints` as
///   `"requires_approval: <action>"` lines
/// - `authority.expires` → `expires` (RFC-3339; falls back to `YYYY-MM-DD`
///   interpreted as midnight UTC)
/// - `constraint.budget` ⊕ `constraint.cost_limit` → `constraints`
/// - `exception.escalates_to` ⊕ `exception.requires` → `expiry_action`
///   (presence flips the default `Halt` to `Escalate`)
/// - Reversibility defaults to `Reversible`. Inputs can override via a
///   constraint of the form `"reversibility: irreversible"` (case-insensitive).
///
/// # Errors
///
/// Returns [`BridgeError::MissingOutcome`] if neither outcome nor goal is set,
/// and [`BridgeError::ExpiryParse`] if `authority.expires` is malformed.
pub fn compile_truth(input: &TruthInput) -> Result<IntentPacket, BridgeError> {
    let outcome = extract_outcome(input.intent.as_ref())?;
    let expires = extract_expiry(input.authority.as_ref())?;
    let authority = extract_authority(input.authority.as_ref());
    let forbidden = extract_forbidden(input.authority.as_ref(), input.constraint.as_ref());
    let constraints = extract_constraints(input.authority.as_ref(), input.constraint.as_ref());
    let reversibility = extract_reversibility(&constraints);
    let expiry_action = extract_expiry_action(input.exception.as_ref());

    let packet = IntentPacket::new(outcome, expires)
        .with_authority(authority)
        .with_reversibility(reversibility)
        .with_expiry_action(expiry_action);

    Ok(IntentPacket {
        constraints,
        forbidden,
        ..packet
    })
}

fn extract_outcome(intent: Option<&IntentBlock>) -> Result<String, BridgeError> {
    let block = intent.ok_or(BridgeError::MissingOutcome)?;
    block
        .outcome
        .as_ref()
        .or(block.goal.as_ref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or(BridgeError::MissingOutcome)
}

fn extract_expiry(authority: Option<&AuthorityBlock>) -> Result<DateTime<Utc>, BridgeError> {
    let Some(value) = authority.and_then(|a| a.expires.as_ref()) else {
        return Ok(Utc::now() + Duration::hours(DEFAULT_EXPIRY_HOURS));
    };
    let trimmed = value.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Ergonomic fallback: "YYYY-MM-DD" is interpreted as midnight UTC so
    // authors can omit the time component.
    if let Some(dt) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|naive| Utc.from_local_datetime(&naive).single())
    {
        return Ok(dt);
    }
    Err(BridgeError::ExpiryParse {
        value: value.clone(),
        message: "expected RFC-3339 timestamp or YYYY-MM-DD date".into(),
    })
}

fn extract_authority(authority: Option<&AuthorityBlock>) -> Vec<String> {
    let Some(block) = authority else {
        return Vec::new();
    };
    let mut entries: Vec<String> = block.may.iter().map(|s| s.trim().to_string()).collect();
    if let Some(actor) = block.actor.as_ref() {
        let actor = actor.trim();
        if !actor.is_empty() {
            entries.insert(0, format!("actor: {actor}"));
        }
    }
    entries
}

fn extract_forbidden(
    authority: Option<&AuthorityBlock>,
    constraint: Option<&ConstraintBlock>,
) -> Vec<ForbiddenAction> {
    let mut forbidden: Vec<ForbiddenAction> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    if let Some(auth) = authority {
        for action in &auth.must_not {
            let action = action.trim().to_string();
            if !action.is_empty() && seen.insert(action.clone()) {
                forbidden.push(ForbiddenAction {
                    action,
                    reason: "authority".into(),
                });
            }
        }
    }

    if let Some(con) = constraint {
        for action in &con.must_not {
            let action = action.trim().to_string();
            if !action.is_empty() && seen.insert(action.clone()) {
                forbidden.push(ForbiddenAction {
                    action,
                    reason: "constraint".into(),
                });
            }
        }
    }

    forbidden
}

fn extract_constraints(
    authority: Option<&AuthorityBlock>,
    constraint: Option<&ConstraintBlock>,
) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    if let Some(con) = constraint {
        entries.extend(con.budget.iter().map(|b| format!("budget: {}", b.trim())));
        entries.extend(
            con.cost_limit
                .iter()
                .map(|c| format!("cost_limit: {}", c.trim())),
        );
    }
    if let Some(auth) = authority {
        entries.extend(
            auth.requires_approval
                .iter()
                .map(|a| format!("requires_approval: {}", a.trim())),
        );
    }
    entries
}

fn extract_reversibility(constraints: &[String]) -> Reversibility {
    for c in constraints {
        let lower = c.to_lowercase();
        if lower.contains("reversibility:") {
            if lower.contains("irreversible") {
                return Reversibility::Irreversible;
            }
            if lower.contains("partial") {
                return Reversibility::Partial;
            }
        }
    }
    Reversibility::Reversible
}

fn extract_expiry_action(exception: Option<&ExceptionBlock>) -> ExpiryAction {
    match exception {
        Some(block) if !block.escalates_to.is_empty() || !block.requires.is_empty() => {
            ExpiryAction::Escalate
        }
        _ => ExpiryAction::Halt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        intent: Option<IntentBlock>,
        authority: Option<AuthorityBlock>,
        constraint: Option<ConstraintBlock>,
        exception: Option<ExceptionBlock>,
    ) -> TruthInput {
        TruthInput {
            intent,
            authority,
            constraint,
            evidence: None,
            exception,
        }
    }

    #[test]
    fn missing_intent_block_rejected() {
        let i = input(None, None, None, None);
        assert!(matches!(
            compile_truth(&i),
            Err(BridgeError::MissingOutcome)
        ));
    }

    #[test]
    fn intent_with_only_whitespace_outcome_rejected() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("   ".into()),
                goal: None,
            }),
            None,
            None,
            None,
        );
        assert!(matches!(
            compile_truth(&i),
            Err(BridgeError::MissingOutcome)
        ));
    }

    #[test]
    fn outcome_taken_from_intent_outcome_field() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("qualify inbound leads".into()),
                goal: None,
            }),
            None,
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.outcome, "qualify inbound leads");
    }

    #[test]
    fn outcome_falls_back_to_goal() {
        let i = input(
            Some(IntentBlock {
                outcome: None,
                goal: Some("qualify inbound leads".into()),
            }),
            None,
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.outcome, "qualify inbound leads");
    }

    #[test]
    fn authority_actor_prefixes_authority_list() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: Some("revops_team".into()),
                may: vec!["approve_lead".into(), "request_demo".into()],
                must_not: vec![],
                requires_approval: vec![],
                expires: None,
            }),
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(
            packet.authority,
            vec!["actor: revops_team", "approve_lead", "request_demo"]
        );
    }

    #[test]
    fn forbidden_collects_authority_and_constraint_must_not() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: None,
                may: vec![],
                must_not: vec!["delete_account".into()],
                requires_approval: vec![],
                expires: None,
            }),
            Some(ConstraintBlock {
                budget: vec![],
                cost_limit: vec![],
                must_not: vec!["spend_over_500".into()],
            }),
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.forbidden.len(), 2);
        assert_eq!(packet.forbidden[0].action, "delete_account");
        assert_eq!(packet.forbidden[0].reason, "authority");
        assert_eq!(packet.forbidden[1].action, "spend_over_500");
        assert_eq!(packet.forbidden[1].reason, "constraint");
    }

    #[test]
    fn forbidden_deduplicates_same_action_across_blocks() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: None,
                may: vec![],
                must_not: vec!["delete_account".into()],
                requires_approval: vec![],
                expires: None,
            }),
            Some(ConstraintBlock {
                budget: vec![],
                cost_limit: vec![],
                must_not: vec!["delete_account".into()],
            }),
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.forbidden.len(), 1);
        assert_eq!(packet.forbidden[0].reason, "authority");
    }

    #[test]
    fn constraints_carry_budget_cost_and_approval_lines() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: None,
                may: vec![],
                must_not: vec![],
                requires_approval: vec!["spend_over_1000".into()],
                expires: None,
            }),
            Some(ConstraintBlock {
                budget: vec!["$500".into()],
                cost_limit: vec!["$100/lead".into()],
                must_not: vec![],
            }),
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert!(packet.constraints.contains(&"budget: $500".to_string()));
        assert!(
            packet
                .constraints
                .contains(&"cost_limit: $100/lead".to_string())
        );
        assert!(
            packet
                .constraints
                .contains(&"requires_approval: spend_over_1000".to_string())
        );
    }

    #[test]
    fn expiry_parses_rfc3339() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: None,
                may: vec![],
                must_not: vec![],
                requires_approval: vec![],
                expires: Some("2027-01-15T12:00:00Z".into()),
            }),
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.expires.to_rfc3339(), "2027-01-15T12:00:00+00:00");
    }

    #[test]
    fn expiry_parses_yyyy_mm_dd_as_midnight_utc() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: None,
                may: vec![],
                must_not: vec![],
                requires_approval: vec![],
                expires: Some("2027-01-15".into()),
            }),
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.expires.to_rfc3339(), "2027-01-15T00:00:00+00:00");
    }

    #[test]
    fn malformed_expiry_rejected() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            Some(AuthorityBlock {
                actor: None,
                may: vec![],
                must_not: vec![],
                requires_approval: vec![],
                expires: Some("not-a-date".into()),
            }),
            None,
            None,
        );
        assert!(matches!(
            compile_truth(&i),
            Err(BridgeError::ExpiryParse { .. })
        ));
    }

    #[test]
    fn missing_expiry_uses_default_window() {
        let before = Utc::now();
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            None,
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        let after = Utc::now();
        let expected_min = before + Duration::hours(DEFAULT_EXPIRY_HOURS);
        let expected_max = after + Duration::hours(DEFAULT_EXPIRY_HOURS);
        assert!(packet.expires >= expected_min && packet.expires <= expected_max);
    }

    #[test]
    fn reversibility_irreversible_when_constraint_says_so() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            None,
            Some(ConstraintBlock {
                budget: vec!["reversibility: irreversible".into()],
                cost_limit: vec![],
                must_not: vec![],
            }),
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.reversibility, Reversibility::Irreversible);
    }

    #[test]
    fn reversibility_defaults_to_reversible() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            None,
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.reversibility, Reversibility::Reversible);
    }

    #[test]
    fn exception_block_flips_expiry_action_to_escalate() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            None,
            None,
            Some(ExceptionBlock {
                escalates_to: vec!["legal".into()],
                requires: vec![],
            }),
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.expiry_action, ExpiryAction::Escalate);
    }

    #[test]
    fn no_exception_block_keeps_default_halt() {
        let i = input(
            Some(IntentBlock {
                outcome: Some("ship".into()),
                goal: None,
            }),
            None,
            None,
            None,
        );
        let packet = compile_truth(&i).expect("compiles");
        assert_eq!(packet.expiry_action, ExpiryAction::Halt);
    }
}
