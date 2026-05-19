//! Formation Dynamics — the deliberation that decides which formation runs.
//!
//! ## Invariant
//!
//! There is one model: everything is a Suggestor, every collaboration
//! is a Formation, every Formation runs through Converge. The "design
//! huddle" that picks a work formation is itself a Formation, composed
//! of normal Suggestors. This crate adds the missing pieces needed for
//! that loop:
//!
//! - The [`FormationDraft`] payload contract (typed at the Organism
//!   boundary, JSON-in-TextPayload on the wire).
//! - [`CatalogProposerSuggestor`] — proposes draft facts under
//!   `ContextKey::Strategies`.
//! - [`BeautyContestSuggestor`] — proposes a top-N shortlist under
//!   `ContextKey::Proposals`.
//! - [`extract_drafts`] — reads draft facts from a Converge context.
//! - [`compile_draft`] — thin wrapper over
//!   [`organism_runtime::FormationCompiler::compile_draft_from_catalog`],
//!   the exact-roster validator.
//!
//! ## What this crate is NOT
//!
//! - No new traits beyond [`converge_pack::Suggestor`]. There is no
//!   `FormationDesignHuddle` trait or `BeautyContest` trait — those
//!   would be a side-car workflow engine alongside Converge.
//! - No new [`converge_kernel::ContextKey`] — drafts live under
//!   existing `Strategies` (candidates) and `Proposals` (shortlist),
//!   distinguished by a literal `kind` field in their JSON payload.
//! - No new [`converge_pack::FactPayload`] variant — drafts ride
//!   inside [`converge_pack::TextPayload`] with a strict JSON schema.
//!   The wire-level family stays `"converge.text"`; discrimination is
//!   explicit in the payload.
//! - No orchestrator on [`organism_runtime::Runtime`] that drives the
//!   loop end-to-end. Runtime stays a coordinator; hosts compose:
//!   run the design Formation, call [`extract_drafts`], call
//!   [`compile_draft`], run the validated work Formation.
//!
//! ## Language
//!
//! Suggestors *propose* — Converge admits and may promote. The
//! proposer Suggestor proposes draft facts; the scorer Suggestor
//! proposes a shortlist. The shortlist becomes promoted only if
//! Converge accepts the proposals.

mod batch;
mod compile;
mod critic;
mod exclusion;
mod extract;
mod payload;
mod preflight;
mod proposer;
mod provenance;
mod scorer;

pub use compile::compile_draft;
pub use critic::{
    CRITIC_PASS_COMPLETE_MARKER, DraftValidatorCriticSuggestor, critic_pass_complete_marker,
};
pub use exclusion::{BlockedMinusPassedPolicy, RoundExclusionPolicy};
pub use extract::{
    DraftParseError, completed_batches, extract_draft_validations, extract_drafts,
    extract_drafts_for_batch, latest_completed_batch,
};
pub use payload::{
    DRAFT_KIND, DRAFT_VALIDATION_KIND, DraftBatchId, DraftId, DraftValidation,
    DraftValidationPayloadError, DraftVerdict, FormationDraft, FormationDraftValidationError,
};
pub use preflight::{PreflightError, preflight_design_formation};
pub use proposer::{
    CatalogProposerSuggestor, PROPOSER_EXCLUSIONS_PREFIX, proposer_exclusions_marker,
};
pub use scorer::{
    BeautyContestSuggestor, SCORER_BATCH_COMPLETE_PREFIX, scorer_batch_complete_marker,
};
