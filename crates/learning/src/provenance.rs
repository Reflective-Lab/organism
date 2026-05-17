//! Organism learning provenance marker.

use converge_pack::ProvenanceSource;

/// Marker type identifying learning-layer Organism facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganismLearning;

impl ProvenanceSource for OrganismLearning {
    fn as_str(&self) -> &'static str {
        "organism-learning"
    }
}

/// Canonical provenance constant for learning-layer Organism facts.
pub const ORGANISM_LEARNING_PROVENANCE: OrganismLearning = OrganismLearning;
