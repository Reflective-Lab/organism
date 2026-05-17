//! Descriptor catalog and lookup primitives for Organism's selection layer.
//!
//! This crate is descriptor-first. It owns serializable, side-effect-free
//! metadata about Suggestors and Providers and the lookup primitives that
//! search over them. It deliberately does NOT own factories, live
//! `Arc<dyn Suggestor>` values, or any executable instantiation — those
//! remain host-wired in `organism-runtime`.
//!
//! The split mirrors the existing `SuggestorDescriptorCatalog` /
//! `ExecutableSuggestorCatalog` pattern from `organism-runtime`: descriptors
//! describe candidates; the executable side maps compiled IDs to factories.
//!
//! Catalog metadata reuses Converge vocabulary directly:
//! - [`converge_kernel::formation::SuggestorRole`] /
//!   [`converge_kernel::formation::SuggestorCapability`] /
//!   [`converge_kernel::formation::ProfileSnapshot`] for profile shape
//! - [`converge_pack::FactFamilyId`] for what Suggestors produce
//! - [`converge_kernel::ContextKey`] for what they read
//! - [`converge_provider::BackendRequirements`] for provider matching
//!
//! Beyond Converge's existing surface, this crate adds an additive sidecar
//! ([`DiscoveryMetadata`] wrapped by [`CatalogSuggestorDescriptor`]) carrying
//! the descriptor's natural-language summary, use-when guidance, example
//! task phrasings, the fact families it produces, and one or more
//! [`LoopContribution`]s describing how it participates in deliberation.

mod descriptor;
mod discovery;
mod lookup;
mod registry;

pub use descriptor::{
    DataContract, GovernanceClass, ProviderDescriptor, ProviderDescriptorCatalog, ReplayMode,
    SuggestorDescriptor, SuggestorDescriptorCatalog,
};
pub use discovery::{CatalogSuggestorDescriptor, DiscoveryMetadata, LoopContribution};
pub use lookup::{CatalogLookup, ChatBackendLookup, KeywordLookup, LookupError, Suggestion};
pub use registry::DiscoveryCatalog;
