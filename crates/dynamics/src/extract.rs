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

use crate::batch::decode_batch_id;
use crate::payload::{DraftValidation, FormationDraft, FormationDraftValidationError};
use crate::scorer::SCORER_BATCH_COMPLETE_PREFIX;

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
    /// JSON parsed as a draft but failed the strict draft validator.
    #[error("invalid draft: {0}")]
    InvalidDraft(#[from] FormationDraftValidationError),
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
            parsed.validate()?;
            Ok(parsed)
        })
        .collect()
}

/// Extract drafts at `context_key` that belong to a specific
/// `draft_batch_id`. Convenience wrapper over [`extract_drafts`] for
/// callers that need to drive a compile handoff off one batch only —
/// "round 2's drafts," "the synthesizer-selected batch's drafts" —
/// without picking among facts from other batches.
#[must_use]
pub fn extract_drafts_for_batch(
    context: &dyn Context,
    context_key: ContextKey,
    draft_batch_id: &str,
) -> Vec<FormationDraft> {
    extract_drafts(context, context_key)
        .into_iter()
        .filter(|d| d.draft_batch_id == draft_batch_id)
        .collect()
}

/// Every `draft_batch_id` for which the
/// [`crate::scorer::BeautyContestSuggestor`] has emitted a scorer
/// completion sentinel under [`ContextKey::Diagnostic`], in fact
/// iteration order — earliest completion first. Use as the audit of
/// which rounds have actually finished, not as a "shortlist exists"
/// check.
#[must_use]
pub fn completed_batches(context: &dyn Context) -> Vec<String> {
    context
        .get(ContextKey::Diagnostic)
        .iter()
        .filter_map(|fact| {
            fact.id()
                .as_str()
                .strip_prefix(SCORER_BATCH_COMPLETE_PREFIX)
                .and_then(|rest| rest.strip_prefix('-'))
                .and_then(decode_batch_id)
        })
        .collect()
}

/// The most recently completed draft batch — the last entry in
/// [`completed_batches`]. Returns `None` if no batch has finished
/// scoring yet.
///
/// "Latest" here means "last to emit its scorer sentinel," not "highest
/// round number" — caller code that wants explicit round selection
/// should match on `draft_batch_id` directly via
/// [`extract_drafts_for_batch`] instead of relying on this helper.
#[must_use]
pub fn latest_completed_batch(context: &dyn Context) -> Option<String> {
    completed_batches(context).pop()
}

/// Extract every well-formed [`DraftValidation`] fact from `context`
/// at `context_key`. Same strict-parser semantics as
/// [`extract_drafts`]: non-text payload, malformed JSON, wrong
/// `kind`, or shape-invalid verdicts are silently skipped. Use to
/// read critic verdicts (e.g. from `ContextKey::Evaluations` for
/// passes or `ContextKey::Constraints` for blocks).
#[must_use]
pub fn extract_draft_validations(
    context: &dyn Context,
    context_key: ContextKey,
) -> Vec<DraftValidation> {
    context
        .get(context_key)
        .iter()
        .filter_map(|fact| {
            let text = fact.payload::<TextPayload>()?;
            let parsed: DraftValidation = serde_json::from_str(text.as_str()).ok()?;
            parsed.validate().ok()?;
            Some(parsed)
        })
        .collect()
}
