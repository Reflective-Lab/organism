//! Organism runtime provenance marker.

use converge_pack::ProvenanceSource;

/// Marker type identifying runtime-layer Organism facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganismRuntime;

impl ProvenanceSource for OrganismRuntime {
    fn as_str(&self) -> &'static str {
        "organism-runtime"
    }
}

/// Canonical provenance constant for runtime-layer Organism facts.
pub const ORGANISM_RUNTIME_PROVENANCE: OrganismRuntime = OrganismRuntime;
