//! [`FormationDraft`] — the typed payload for proposed and shortlisted
//! formation drafts.
//!
//! Wire format: JSON serialized via serde, transported inside
//! [`converge_pack::TextPayload`]. The literal `kind` field is the
//! discriminator parsers check before accepting a fact as a draft.
//! Wire-level family stays `"converge.text"`; the discriminator is
//! explicit in the payload.

use serde::{Deserialize, Serialize};

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

    /// Returns true if `kind` matches the expected discriminator.
    /// Parsers must check this before trusting the rest of the struct.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.kind == DRAFT_KIND
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
