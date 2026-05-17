//! [`FormationDraft`] — the typed payload for proposed and shortlisted
//! formation drafts.
//!
//! Wire format: JSON serialized via serde, transported inside
//! [`converge_pack::TextPayload`]. The literal `kind` field is the
//! discriminator parsers check before accepting a fact as a draft.
//! Wire-level family stays `"converge.text"`; the discriminator is
//! explicit in the payload.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The strict discriminator. Every [`FormationDraft`] carries this
/// exact value in its `kind` field. Parsers reject any fact whose
/// `kind` does not match.
pub const DRAFT_KIND: &str = "organism.dynamics.formation-draft";

/// A proposed formation draft — an ordered roster of descriptor ids
/// the upstream deliberation Formation believes can satisfy a work
/// template, plus a rationale and a source label for audit.
///
/// The `kind` field is a strict literal discriminator. When
/// constructing via [`FormationDraft::new`] it is always set to
/// [`DRAFT_KIND`]; when parsing from JSON, [`Self::is_well_formed`]
/// must hold before the value is trusted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormationDraft {
    /// Literal discriminator; must equal [`DRAFT_KIND`].
    pub kind: String,
    /// The proposed roster, in the order the upstream proposer
    /// intends. Each id must resolve in the catalog at compile time
    /// (see [`crate::compile_draft`]).
    pub descriptor_ids: Vec<String>,
    /// One-sentence human-readable reason this draft was proposed.
    pub rationale: String,
    /// Name of the Suggestor (or other source) that proposed this
    /// draft. Used for audit, not for routing.
    pub source: String,
}

/// Why a [`FormationDraft`] is not trustworthy enough to feed into
/// draft extraction, scoring, or exact compilation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FormationDraftValidationError {
    /// The literal discriminator did not match [`DRAFT_KIND`].
    #[error("kind mismatch: expected '{expected}', got '{actual}'")]
    KindMismatch {
        expected: &'static str,
        actual: String,
    },
    /// The draft had no descriptor ids.
    #[error("draft must contain at least one descriptor id")]
    EmptyDescriptorIds,
    /// One descriptor id was empty or whitespace only.
    #[error("draft contains an empty descriptor id")]
    EmptyDescriptorId,
    /// A descriptor id appeared more than once.
    #[error("draft references descriptor '{descriptor_id}' more than once")]
    DuplicateDescriptorId { descriptor_id: String },
    /// The source label was empty or whitespace only.
    #[error("draft source must not be empty")]
    EmptySource,
    /// The rationale was empty or whitespace only.
    #[error("draft rationale must not be empty")]
    EmptyRationale,
}

impl FormationDraft {
    /// Construct a draft with the discriminator set correctly.
    #[must_use]
    pub fn new(
        descriptor_ids: Vec<String>,
        rationale: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            kind: DRAFT_KIND.to_string(),
            descriptor_ids,
            rationale: rationale.into(),
            source: source.into(),
        }
    }

    /// Validate the discriminator and basic roster shape.
    ///
    /// This does not prove the descriptor ids exist in a catalog; that is
    /// [`organism_runtime::FormationCompiler::compile_draft_from_catalog`]'s
    /// job. It does prevent malformed drafts from being extracted or
    /// scored as candidates.
    pub fn validate(&self) -> Result<(), FormationDraftValidationError> {
        if self.kind != DRAFT_KIND {
            return Err(FormationDraftValidationError::KindMismatch {
                expected: DRAFT_KIND,
                actual: self.kind.clone(),
            });
        }
        if self.descriptor_ids.is_empty() {
            return Err(FormationDraftValidationError::EmptyDescriptorIds);
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in &self.descriptor_ids {
            if id.trim().is_empty() {
                return Err(FormationDraftValidationError::EmptyDescriptorId);
            }
            if !seen.insert(id.as_str()) {
                return Err(FormationDraftValidationError::DuplicateDescriptorId {
                    descriptor_id: id.clone(),
                });
            }
        }
        if self.source.trim().is_empty() {
            return Err(FormationDraftValidationError::EmptySource);
        }
        if self.rationale.trim().is_empty() {
            return Err(FormationDraftValidationError::EmptyRationale);
        }
        Ok(())
    }

    /// Returns true if the discriminator and basic roster shape are valid.
    /// Parsers must check this before trusting the rest of the struct.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.validate().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_discriminator() {
        let draft = FormationDraft::new(vec!["a".into()], "why", "proposer");
        assert!(draft.is_well_formed());
        assert_eq!(draft.kind, DRAFT_KIND);
    }

    #[test]
    fn deserialized_with_wrong_kind_is_rejected_by_predicate() {
        let json =
            r#"{"kind":"something.else","descriptor_ids":["a"],"rationale":"r","source":"s"}"#;
        let parsed: FormationDraft = serde_json::from_str(json).unwrap();
        assert!(!parsed.is_well_formed());
        assert!(matches!(
            parsed.validate(),
            Err(FormationDraftValidationError::KindMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_descriptor_id_is_rejected_by_predicate() {
        let draft = FormationDraft::new(vec!["a".into(), "a".into()], "why", "proposer");
        assert!(!draft.is_well_formed());
        assert!(matches!(
            draft.validate(),
            Err(FormationDraftValidationError::DuplicateDescriptorId { ref descriptor_id })
                if descriptor_id == "a"
        ));
    }

    #[test]
    fn empty_fields_are_rejected_by_predicate() {
        let empty_roster = FormationDraft::new(Vec::new(), "why", "proposer");
        assert!(matches!(
            empty_roster.validate(),
            Err(FormationDraftValidationError::EmptyDescriptorIds)
        ));

        let empty_source = FormationDraft::new(vec!["a".into()], "why", " ");
        assert!(matches!(
            empty_source.validate(),
            Err(FormationDraftValidationError::EmptySource)
        ));
    }

    #[test]
    fn round_trip_via_json() {
        let draft = FormationDraft::new(vec!["a".into(), "b".into()], "why", "proposer");
        let json = serde_json::to_string(&draft).unwrap();
        let parsed: FormationDraft = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, draft);
        assert!(parsed.is_well_formed());
    }
}

// ---------------------------------------------------------------------------
// DraftValidation — typed verdict payload from a critic Suggestor
// ---------------------------------------------------------------------------

/// Strict discriminator for [`DraftValidation`] facts.
pub const DRAFT_VALIDATION_KIND: &str = "organism.dynamics.draft-validation";

/// Critic verdict on a single [`FormationDraft`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftVerdict {
    /// The draft is admissible — passes the critic's check.
    Pass,
    /// The draft is rejected — must not be selected as the work plan.
    Block,
}

/// A critic's typed verdict on a specific [`FormationDraft`]. Proposed
/// by a Suggestor (e.g. the catalog-aware draft validator) under
/// `ContextKey::Evaluations` (`Pass`) or `ContextKey::Constraints`
/// (`Block`); Converge admits and may promote. Wire format is
/// JSON-in-TextPayload with the [`DRAFT_VALIDATION_KIND`] literal
/// discriminator — same shape as [`FormationDraft`].
///
/// The join key from a verdict back to the draft it judged is the pair
/// `(draft_source, draft_index)`. The proposer that emitted the draft
/// is the source of truth for those values — critics must take them
/// from the draft they read, not invent them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftValidation {
    /// Literal discriminator; must equal [`DRAFT_VALIDATION_KIND`].
    pub kind: String,
    /// The `source` field of the [`FormationDraft`] this verdict
    /// applies to.
    pub draft_source: String,
    /// The position of the draft within its proposer's emission. Used
    /// with `draft_source` to join a verdict back to a draft.
    pub draft_index: usize,
    /// The verdict itself.
    pub verdict: DraftVerdict,
    /// Human-readable explanation. Required and non-empty.
    pub reason: String,
    /// The name of the critic Suggestor that emitted this verdict
    /// (audit only).
    pub critic: String,
}

/// Why a [`DraftValidation`] fact failed validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DraftValidationPayloadError {
    /// The literal discriminator did not match
    /// [`DRAFT_VALIDATION_KIND`].
    #[error("kind mismatch: expected '{expected}', got '{actual}'")]
    KindMismatch {
        expected: &'static str,
        actual: String,
    },
    /// The reason text was empty or whitespace only.
    #[error("draft validation reason must not be empty")]
    EmptyReason,
    /// The critic label was empty or whitespace only.
    #[error("draft validation critic must not be empty")]
    EmptyCritic,
    /// The draft_source was empty or whitespace only.
    #[error("draft validation draft_source must not be empty")]
    EmptyDraftSource,
}

impl DraftValidation {
    /// Construct a verdict with the discriminator set correctly.
    #[must_use]
    pub fn new(
        draft_source: impl Into<String>,
        draft_index: usize,
        verdict: DraftVerdict,
        reason: impl Into<String>,
        critic: impl Into<String>,
    ) -> Self {
        Self {
            kind: DRAFT_VALIDATION_KIND.to_string(),
            draft_source: draft_source.into(),
            draft_index,
            verdict,
            reason: reason.into(),
            critic: critic.into(),
        }
    }

    /// Strict-shape validation. Use before trusting a parsed verdict.
    pub fn validate(&self) -> Result<(), DraftValidationPayloadError> {
        if self.kind != DRAFT_VALIDATION_KIND {
            return Err(DraftValidationPayloadError::KindMismatch {
                expected: DRAFT_VALIDATION_KIND,
                actual: self.kind.clone(),
            });
        }
        if self.draft_source.trim().is_empty() {
            return Err(DraftValidationPayloadError::EmptyDraftSource);
        }
        if self.reason.trim().is_empty() {
            return Err(DraftValidationPayloadError::EmptyReason);
        }
        if self.critic.trim().is_empty() {
            return Err(DraftValidationPayloadError::EmptyCritic);
        }
        Ok(())
    }

    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.validate().is_ok()
    }

    /// True if this verdict applies to the supplied draft (matches
    /// both `source` and the draft's index within its proposer's
    /// emission).
    #[must_use]
    pub fn matches(&self, draft_source: &str, draft_index: usize) -> bool {
        self.draft_source == draft_source && self.draft_index == draft_index
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn new_sets_discriminator_and_validates() {
        let v = DraftValidation::new(
            "proposer-a",
            0,
            DraftVerdict::Pass,
            "looks fine",
            "critic-x",
        );
        assert!(v.is_well_formed());
        assert_eq!(v.kind, DRAFT_VALIDATION_KIND);
    }

    #[test]
    fn wrong_kind_rejected_by_predicate() {
        let json = r#"{"kind":"x","draft_source":"p","draft_index":0,"verdict":"pass","reason":"r","critic":"c"}"#;
        let parsed: DraftValidation = serde_json::from_str(json).unwrap();
        assert!(matches!(
            parsed.validate(),
            Err(DraftValidationPayloadError::KindMismatch { .. })
        ));
    }

    #[test]
    fn empty_fields_rejected() {
        let v = DraftValidation::new("p", 0, DraftVerdict::Block, " ", "c");
        assert!(matches!(
            v.validate(),
            Err(DraftValidationPayloadError::EmptyReason)
        ));
    }

    #[test]
    fn matches_pairs_source_and_index() {
        let v = DraftValidation::new("p", 2, DraftVerdict::Pass, "r", "c");
        assert!(v.matches("p", 2));
        assert!(!v.matches("p", 3));
        assert!(!v.matches("other", 2));
    }

    #[test]
    fn verdict_serializes_snake_case() {
        let json = serde_json::to_string(&DraftVerdict::Block).unwrap();
        assert_eq!(json, "\"block\"");
    }
}
