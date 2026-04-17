//! Diligence-to-decision blueprint.
//!
//! Packs: DueDiligence → Legal → Knowledge
//!
//! ## Hypothesis lifecycle wiring
//!
//! Add `organism_planning::suggestor::HypothesisTrackerSuggestor::new("dd")`
//! to the engine suggestor list. After the run, emit `HypothesisResolved`
//! events from `tracker.resolved()`. See `organism-planning` docs.

use crate::pack::{InvariantClass, InvariantMeta};

pub const CROSS_PACK_INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "synthesis_before_decision",
        class: InvariantClass::Acceptance,
        description: "DD synthesis must exist before legal review begins",
    },
    InvariantMeta {
        name: "contradictions_require_human",
        class: InvariantClass::Semantic,
        description: "Flagged contradictions escalate to HITL before convergence",
    },
    InvariantMeta {
        name: "findings_feed_knowledge",
        class: InvariantClass::Acceptance,
        description: "Confirmed facts must promote to the knowledge base",
    },
];

pub const PACKS: &[&str] = &["due_diligence", "legal", "knowledge"];
