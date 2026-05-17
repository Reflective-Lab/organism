//! Organism planning provenance marker.

use converge_pack::ProvenanceSource;

/// Marker type identifying planning-layer Organism facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganismPlanning;

impl ProvenanceSource for OrganismPlanning {
    fn as_str(&self) -> &'static str {
        "organism-planning"
    }
}

/// Canonical provenance constant for planning-layer Organism facts.
pub const ORGANISM_PLANNING_PROVENANCE: OrganismPlanning = OrganismPlanning;
