//! [`FormationDraft`] — the typed payload for proposed and shortlisted
//! formation drafts.
//!
//! Wire format: JSON serialized via serde, transported inside
//! [`converge_pack::TextPayload`]. The literal `kind` field is the
//! discriminator parsers check before accepting a fact as a draft.
//! Wire-level family stays `"converge.text"`; the discriminator is
//! explicit in the payload.

use organism_catalog::SuggestorDescriptorId;
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
#[serde(deny_unknown_fields)]
pub struct FormationDraft {
    /// Literal discriminator; must equal [`DRAFT_KIND`].
    pub kind: String,
    /// Stable identifier for this draft, assigned by the proposer.
    /// Used with [`Self::draft_batch_id`] by downstream Suggestors
    /// (critic verdicts, scorer shortlist filtering) as the routing
    /// key — verdicts join back to drafts by `(draft_batch_id,
    /// draft_id)`, not by reconstructed ordering or source/index
    /// pairs. Must be non-empty and unique within its batch;
    /// proposers are responsible for picking ids that won't collide
    /// inside a batch.
    pub draft_id: String,
    /// Identifier of the batch this draft belongs to. The critic
    /// emits a per-batch sentinel and the scorer waits for that
    /// sentinel before shortlisting drafts from that batch — so
    /// drafts that arrive in a later batch (after the first batch's
    /// verdicts and shortlist are already in context) can be
    /// processed cleanly without temporal contamination.
    ///
    /// `draft_batch_id` is **routing/audit identity only**. The
    /// compiler ([`organism_runtime::FormationCompiler::compile_draft_from_catalog`])
    /// still decides admissibility — batches are not an authority
    /// boundary and do not partition admissibility.
    ///
    /// Multiple drafts in the same batch share a `draft_batch_id`.
    /// Multiple proposers can run in one Formation by using distinct
    /// `draft_batch_id` values.
    pub draft_batch_id: String,
    /// The proposed roster, in the order the upstream proposer
    /// intends. Each id must resolve in the catalog at compile time
    /// (see [`crate::compile_draft`]).
    pub descriptor_ids: Vec<SuggestorDescriptorId>,
    /// One-sentence human-readable reason this draft was proposed.
    pub rationale: String,
    /// Name of the Suggestor (or other source) that proposed this
    /// draft. Used for audit, not for routing — routing is the
    /// `(draft_batch_id, draft_id)` pair.
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
    /// The draft_id was empty or whitespace only.
    #[error("draft must carry a non-empty draft_id")]
    EmptyDraftId,
    /// The draft_batch_id was empty or whitespace only.
    #[error("draft must carry a non-empty draft_batch_id")]
    EmptyDraftBatchId,
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
    /// `draft_id` is the stable routing key — see [`Self::draft_id`].
    /// `draft_batch_id` groups drafts for round-scoped critic and
    /// scorer gating — see [`Self::draft_batch_id`].
    #[must_use]
    pub fn new<I, D>(
        draft_id: impl Into<String>,
        draft_batch_id: impl Into<String>,
        descriptor_ids: I,
        rationale: impl Into<String>,
        source: impl Into<String>,
    ) -> Self
    where
        I: IntoIterator<Item = D>,
        D: Into<SuggestorDescriptorId>,
    {
        Self {
            kind: DRAFT_KIND.to_string(),
            draft_id: draft_id.into(),
            draft_batch_id: draft_batch_id.into(),
            descriptor_ids: descriptor_ids.into_iter().map(Into::into).collect(),
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
        if self.draft_id.as_str().trim().is_empty() {
            return Err(FormationDraftValidationError::EmptyDraftId);
        }
        if self.draft_batch_id.as_str().trim().is_empty() {
            return Err(FormationDraftValidationError::EmptyDraftBatchId);
        }
        if self.descriptor_ids.is_empty() {
            return Err(FormationDraftValidationError::EmptyDescriptorIds);
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in &self.descriptor_ids {
            if id.as_str().trim().is_empty() {
                return Err(FormationDraftValidationError::EmptyDescriptorId);
            }
            if !seen.insert(id.as_str()) {
                return Err(FormationDraftValidationError::DuplicateDescriptorId {
                    descriptor_id: id.to_string(),
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
    fn new_sets_discriminator_and_id() {
        let draft = FormationDraft::new(
            "draft-7",
            "batch-1",
            vec![SuggestorDescriptorId::from("a")],
            "why",
            "proposer",
        );
        assert!(draft.is_well_formed());
        assert_eq!(draft.kind, DRAFT_KIND);
        assert_eq!(draft.draft_id, "draft-7");
    }

    #[test]
    fn deserialized_with_wrong_kind_is_rejected_by_predicate() {
        let json = r#"{"kind":"something.else","draft_id":"d","draft_batch_id":"batch-1","descriptor_ids":["a"],"rationale":"r","source":"s"}"#;
        let parsed: FormationDraft = serde_json::from_str(json).unwrap();
        assert!(!parsed.is_well_formed());
        assert!(matches!(
            parsed.validate(),
            Err(FormationDraftValidationError::KindMismatch { .. })
        ));
    }

    #[test]
    fn empty_draft_id_is_rejected_by_predicate() {
        let draft = FormationDraft::new(
            " ",
            "batch-1",
            vec![SuggestorDescriptorId::from("a")],
            "why",
            "proposer",
        );
        assert!(matches!(
            draft.validate(),
            Err(FormationDraftValidationError::EmptyDraftId)
        ));
    }

    #[test]
    fn duplicate_descriptor_id_is_rejected_by_predicate() {
        let draft = FormationDraft::new(
            "d",
            "batch-1",
            vec![
                SuggestorDescriptorId::from("a"),
                SuggestorDescriptorId::from("a"),
            ],
            "why",
            "proposer",
        );
        assert!(!draft.is_well_formed());
        assert!(matches!(
            draft.validate(),
            Err(FormationDraftValidationError::DuplicateDescriptorId { ref descriptor_id })
                if descriptor_id == "a"
        ));
    }

    #[test]
    fn empty_fields_are_rejected_by_predicate() {
        let empty_roster = FormationDraft::new(
            "d",
            "batch-1",
            Vec::<SuggestorDescriptorId>::new(),
            "why",
            "proposer",
        );
        assert!(matches!(
            empty_roster.validate(),
            Err(FormationDraftValidationError::EmptyDescriptorIds)
        ));

        let empty_source = FormationDraft::new(
            "d",
            "batch-1",
            vec![SuggestorDescriptorId::from("a")],
            "why",
            " ",
        );
        assert!(matches!(
            empty_source.validate(),
            Err(FormationDraftValidationError::EmptySource)
        ));
    }

    #[test]
    fn round_trip_via_json() {
        let draft = FormationDraft::new(
            "d",
            "batch-1",
            vec![
                SuggestorDescriptorId::from("a"),
                SuggestorDescriptorId::from("b"),
            ],
            "why",
            "proposer",
        );
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
/// The join key from a verdict back to the draft it judged is the
/// `(draft_batch_id, draft_id)` pair. Critics must take both values
/// from the draft they read, not reconstruct them from ordering or
/// source labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftValidation {
    /// Literal discriminator; must equal [`DRAFT_VALIDATION_KIND`].
    pub kind: String,
    /// Stable identifier of the [`FormationDraft`] this verdict
    /// applies to. Copied verbatim from `FormationDraft.draft_id`.
    pub draft_id: String,
    /// Batch id of the [`FormationDraft`] this verdict applies to —
    /// copied verbatim from `FormationDraft.draft_batch_id`. Used by
    /// the scorer to enumerate which batches have completed
    /// validation and which are still pending. This is part of the
    /// authoritative join key with [`Self::draft_id`].
    pub draft_batch_id: String,
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
    /// The draft_id was empty or whitespace only.
    #[error("draft validation draft_id must not be empty")]
    EmptyDraftId,
    /// The draft_batch_id was empty or whitespace only.
    #[error("draft validation draft_batch_id must not be empty")]
    EmptyDraftBatchId,
    /// The reason text was empty or whitespace only.
    #[error("draft validation reason must not be empty")]
    EmptyReason,
    /// The critic label was empty or whitespace only.
    #[error("draft validation critic must not be empty")]
    EmptyCritic,
}

impl DraftValidation {
    /// Construct a verdict with the discriminator set correctly.
    /// `draft_id` and `draft_batch_id` must be the same values the
    /// draft carries — copied verbatim from the
    /// [`FormationDraft`].
    #[must_use]
    pub fn new(
        draft_id: impl Into<String>,
        draft_batch_id: impl Into<String>,
        verdict: DraftVerdict,
        reason: impl Into<String>,
        critic: impl Into<String>,
    ) -> Self {
        Self {
            kind: DRAFT_VALIDATION_KIND.to_string(),
            draft_id: draft_id.into(),
            draft_batch_id: draft_batch_id.into(),
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
        if self.draft_id.as_str().trim().is_empty() {
            return Err(DraftValidationPayloadError::EmptyDraftId);
        }
        if self.draft_batch_id.as_str().trim().is_empty() {
            return Err(DraftValidationPayloadError::EmptyDraftBatchId);
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

    /// True if this verdict applies to the supplied draft. The join
    /// key is the draft's `(draft_batch_id, draft_id)` pair — no
    /// reconstructed indices, no source matching.
    #[must_use]
    pub fn matches(&self, draft_batch_id: &str, draft_id: &str) -> bool {
        self.draft_batch_id == draft_batch_id && self.draft_id == draft_id
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn new_sets_discriminator_and_validates() {
        let v = DraftValidation::new(
            "draft-a",
            "batch-1",
            DraftVerdict::Pass,
            "looks fine",
            "critic-x",
        );
        assert!(v.is_well_formed());
        assert_eq!(v.kind, DRAFT_VALIDATION_KIND);
        assert_eq!(v.draft_id, "draft-a");
        assert_eq!(v.draft_batch_id, "batch-1");
    }

    #[test]
    fn wrong_kind_rejected_by_predicate() {
        let json = r#"{"kind":"x","draft_id":"d","draft_batch_id":"b","verdict":"pass","reason":"r","critic":"c"}"#;
        let parsed: DraftValidation = serde_json::from_str(json).unwrap();
        assert!(matches!(
            parsed.validate(),
            Err(DraftValidationPayloadError::KindMismatch { .. })
        ));
    }

    #[test]
    fn empty_draft_id_rejected() {
        let v = DraftValidation::new(" ", "batch-1", DraftVerdict::Pass, "r", "c");
        assert!(matches!(
            v.validate(),
            Err(DraftValidationPayloadError::EmptyDraftId)
        ));
    }

    #[test]
    fn empty_draft_batch_id_rejected() {
        let v = DraftValidation::new("d", " ", DraftVerdict::Pass, "r", "c");
        assert!(matches!(
            v.validate(),
            Err(DraftValidationPayloadError::EmptyDraftBatchId)
        ));
    }

    #[test]
    fn empty_fields_rejected() {
        let v = DraftValidation::new("d", "batch-1", DraftVerdict::Block, " ", "c");
        assert!(matches!(
            v.validate(),
            Err(DraftValidationPayloadError::EmptyReason)
        ));
    }

    #[test]
    fn matches_joins_by_draft_id() {
        let v = DraftValidation::new("draft-7", "batch-1", DraftVerdict::Pass, "r", "c");
        assert!(v.matches("batch-1", "draft-7"));
        assert!(!v.matches("batch-2", "draft-7"));
        assert!(!v.matches("batch-1", "draft-8"));
        assert!(!v.matches("", ""));
    }

    #[test]
    fn verdict_serializes_snake_case() {
        let json = serde_json::to_string(&DraftVerdict::Block).unwrap();
        assert_eq!(json, "\"block\"");
    }
}
