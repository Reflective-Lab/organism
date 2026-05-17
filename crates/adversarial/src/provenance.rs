//! Organism adversarial provenance marker.

use converge_pack::ProvenanceSource;

/// Marker type identifying adversarial-layer Organism facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganismAdversarial;

impl ProvenanceSource for OrganismAdversarial {
    fn as_str(&self) -> &'static str {
        "organism-adversarial"
    }
}

/// Canonical provenance constant for adversarial-layer Organism facts.
pub const ORGANISM_ADVERSARIAL_PROVENANCE: OrganismAdversarial = OrganismAdversarial;
