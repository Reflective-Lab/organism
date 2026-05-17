//! Provenance marker for facts proposed by organism-dynamics
//! Suggestors. Follows the 3.9 contract: a unit struct implementing
//! [`ProvenanceSource`] with a stable static identifier.

use converge_pack::ProvenanceSource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganismDynamics;

impl ProvenanceSource for OrganismDynamics {
    fn as_str(&self) -> &'static str {
        "organism-dynamics"
    }
}

pub const ORGANISM_DYNAMICS_PROVENANCE: OrganismDynamics = OrganismDynamics;
