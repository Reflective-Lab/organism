//! Read [`FormationDraft`] facts out of a Converge context.
//!
//! Strict-parser semantics: facts whose payload is not a
//! [`converge_pack::TextPayload`], whose text is not valid JSON, or
//! whose JSON has the wrong `kind` discriminator, are silently
//! skipped — they are not drafts and should not pretend to be. This
//! lets a design Formation interleave draft proposals with unrelated
//! facts on the same `ContextKey` without contamination.

use converge_kernel::{Context, ContextKey};
use converge_pack::TextPayload;
use thiserror::Error;

use crate::payload::FormationDraft;

/// Why a single fact failed to parse as a [`FormationDraft`]. Returned
/// from [`extract_drafts_strict`] for diagnostic use; the default
/// [`extract_drafts`] silently skips invalid facts.
#[derive(Debug, Error)]
pub enum DraftParseError {
    /// The fact's payload was not a [`TextPayload`].
    #[error("fact payload is not TextPayload")]
    PayloadNotText,
    /// The TextPayload content failed JSON parsing.
    #[error("text payload is not valid JSON: {0}")]
    Json(String),
    /// JSON parsed but the `kind` discriminator did not match
    /// [`crate::DRAFT_KIND`].
    #[error("kind mismatch: expected '{expected}', got '{actual}'")]
    KindMismatch {
        expected: &'static str,
        actual: String,
    },
}

/// Extract every well-formed [`FormationDraft`] fact from `context`
/// at `context_key`. Silently skips facts that are not drafts
/// (non-text payload, malformed JSON, wrong `kind`). Use
/// [`extract_drafts_strict`] if you need per-fact diagnostics.
#[must_use]
pub fn extract_drafts(context: &dyn Context, context_key: ContextKey) -> Vec<FormationDraft> {
    extract_drafts_strict(context, context_key)
        .into_iter()
        .filter_map(Result::ok)
        .collect()
}

/// Strict variant of [`extract_drafts`] — returns one entry per fact
/// at `context_key`, each either the parsed draft or the parse error
/// for that fact. Order matches the underlying fact order.
#[must_use]
pub fn extract_drafts_strict(
    context: &dyn Context,
    context_key: ContextKey,
) -> Vec<Result<FormationDraft, DraftParseError>> {
    context
        .get(context_key)
        .iter()
        .map(|fact| {
            let text = fact
                .payload::<TextPayload>()
                .ok_or(DraftParseError::PayloadNotText)?;
            let parsed: FormationDraft = serde_json::from_str(text.as_str())
                .map_err(|err| DraftParseError::Json(err.to_string()))?;
            if !parsed.is_well_formed() {
                return Err(DraftParseError::KindMismatch {
                    expected: crate::payload::DRAFT_KIND,
                    actual: parsed.kind,
                });
            }
            Ok(parsed)
        })
        .collect()
}
