//! Internal helpers shared by the family modules.

use converge_kernel::ContextKey;
use converge_kernel::formation::{ProfileSnapshot, SuggestorCapability, SuggestorRole};
use converge_pack::FactFamilyId;
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{
    CatalogSuggestorDescriptor, DiscoveryMetadata, LoopContribution, SuggestorDescriptor,
};

/// Compact spec for authoring mosaic-family descriptors. Each family
/// module turns one [`EntrySpec`] into a [`CatalogSuggestorDescriptor`]
/// via [`entry`].
pub(crate) struct EntrySpec<'a> {
    pub id: &'a str,
    pub role: SuggestorRole,
    pub capabilities: Vec<SuggestorCapability>,
    pub output_keys: Vec<ContextKey>,
    pub reads: Vec<ContextKey>,
    pub domain_tags: Vec<&'a str>,
    pub cost: CostClass,
    pub latency: LatencyClass,
    pub summary: &'a str,
    pub use_when: &'a str,
    pub examples: Vec<&'a str>,
    pub loop_contributions: Vec<LoopContribution>,
    pub produces: Vec<&'a str>,
}

pub(crate) fn entry(spec: EntrySpec<'_>) -> CatalogSuggestorDescriptor {
    let profile = ProfileSnapshot {
        name: spec.id.to_string(),
        role: spec.role,
        output_keys: spec.output_keys,
        cost_hint: spec.cost,
        latency_hint: spec.latency,
        capabilities: spec.capabilities,
        confidence_min: 0.7,
        confidence_max: 0.95,
    };
    let mut descriptor = SuggestorDescriptor::new(spec.id, profile);
    for key in spec.reads {
        descriptor = descriptor.with_read(key);
    }
    for tag in spec.domain_tags {
        descriptor = descriptor.with_domain_tag(tag);
    }

    let mut discovery = DiscoveryMetadata::new(spec.summary, spec.use_when);
    for ex in spec.examples {
        discovery = discovery.with_example(ex);
    }
    for contribution in spec.loop_contributions {
        discovery = discovery.with_loop_contribution(contribution);
    }
    for family in spec.produces {
        discovery = discovery.with_produces(FactFamilyId::new(family));
    }

    CatalogSuggestorDescriptor::new(descriptor, discovery)
}
