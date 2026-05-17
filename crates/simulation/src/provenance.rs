//! Organism simulation provenance marker.

use converge_pack::ProvenanceSource;

/// Marker type identifying simulation-layer Organism facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganismSimulation;

impl ProvenanceSource for OrganismSimulation {
    fn as_str(&self) -> &'static str {
        "organism-simulation"
    }
}

/// Canonical provenance constant for simulation-layer Organism facts.
pub const ORGANISM_SIMULATION_PROVENANCE: OrganismSimulation = OrganismSimulation;
