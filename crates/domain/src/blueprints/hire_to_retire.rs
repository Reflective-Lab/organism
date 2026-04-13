//! Hire-to-retire blueprint.
//!
//! Packs: Legal → People → Trust (converge-domain) → Money (converge-domain)
//!
//! Cross-pack invariants:
//! - ip_assignment_before_payment: No payment until IP signed
//! - identity_before_access: Identity must be provisioned first
//! - termination_revokes_access: Termination immediately revokes
//! - all_actions_audited: Complete audit trail
