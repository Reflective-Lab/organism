//! [`preflight_design_formation`] — opt-in shape validation before a
//! design Formation is instantiated.
//!
//! Hosts that care about charter-shaped guarantees (minimum members,
//! expected roles, required round synthesis, etc.) call this helper
//! before building their design Formation. It is a thin wrapper over
//! [`organism_runtime::CollaborationRunner::new`], which performs the
//! actual validation. No new authority — no new loop, no overlay on
//! Converge. Just a place to surface charter-shape errors before they
//! become "the Formation ran but the team was wrong."
//!
//! The same Formation is composable with or without preflight; hosts
//! that don't need the check simply skip it.

use thiserror::Error;

use organism_planning::{CollaborationCharter, TeamFormation};
use organism_runtime::{CollaborationParticipant, CollaborationRunner, CollaborationRunnerError};

/// Errors surfaced by [`preflight_design_formation`]. Wraps the
/// underlying [`CollaborationRunnerError`] without adding any new
/// failure modes — preflight only exposes what `CollaborationRunner::new`
/// already validates.
#[derive(Debug, Error)]
pub enum PreflightError {
    #[error("design-formation preflight failed: {0}")]
    Charter(#[from] CollaborationRunnerError),
}

/// Validate a proposed design-Formation team against `charter`. On
/// success, returns the [`CollaborationRunner`] so callers can hold
/// the validated handle alongside the runnable Formation they then
/// build. On failure, returns the underlying validation error
/// verbatim — preflight does not add policy of its own.
///
/// Calling this is opt-in. The same Formation can be instantiated
/// without preflight; the check exists to surface "this team won't
/// satisfy our charter" errors *before* a run starts, not to gate
/// runtime execution.
pub fn preflight_design_formation<P: CollaborationParticipant>(
    team: TeamFormation,
    charter: CollaborationCharter,
    participants: Vec<P>,
) -> Result<CollaborationRunner<P>, PreflightError> {
    Ok(CollaborationRunner::new(team, charter, participants)?)
}
