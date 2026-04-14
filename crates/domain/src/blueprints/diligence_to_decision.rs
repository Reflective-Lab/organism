//! Diligence-to-decision blueprint.
//!
//! Packs: DueDiligence → Legal → Knowledge
//!
//! Cross-pack invariants:
//! - synthesis_before_decision: DD synthesis must exist before legal review
//! - contradictions_require_human: Flagged contradictions escalate to HITL
//! - findings_feed_knowledge: Confirmed facts promote to the knowledge base
