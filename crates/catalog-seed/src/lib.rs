//! Seed descriptor bundle for the discovery catalog.
//!
//! Three trees of curated [`CatalogSuggestorDescriptor`]s, each authored
//! against a specific source release:
//!
//! - [`converge`] — public Suggestors shipped by Converge crates.
//! - [`organism`] — production Suggestors shipped by Organism crates.
//! - [`mosaic`] — Suggestors shipped by mosaic-extensions crates.
//!
//! ## Descriptor-only
//!
//! This crate describes Suggestors that live elsewhere. It deliberately
//! does **not** depend on any Suggestor-implementing crate — descriptors
//! carry text metadata, stable identifiers, and protocol-level types
//! ([`ContextKey`], [`FactFamilyId`], [`SuggestorRole`],
//! [`SuggestorCapability`]) only. Hosts that want to actually
//! instantiate one of these Suggestors register the factory separately
//! in their `ExecutableSuggestorCatalog` and link the relevant source
//! crate.
//!
//! ## Why descriptor-only, single crate
//!
//! - Compile cost stays tiny — the seed builds in seconds.
//! - Ships ahead of (or behind) any source crate's cadence.
//! - One consumption point: hosts depend on `organism-catalog-seed` and
//!   get the full menu, while still picking exactly which Suggestors
//!   their executable catalog actually instantiates.
//!
//! ## Per-source visibility
//!
//! Each tree exposes its own `descriptors()` and `pinned_to()`. Crate
//! level [`all`] returns the union; per-source convenience constructors
//! ([`converge_only`], [`organism_only`], [`mosaic_only`]) return a
//! single tree, and [`pinned_to`] returns the concatenated version
//! manifest.
//!
//! [`ContextKey`]: converge_kernel::ContextKey
//! [`FactFamilyId`]: converge_pack::FactFamilyId
//! [`SuggestorRole`]: converge_kernel::formation::SuggestorRole
//! [`SuggestorCapability`]: converge_kernel::formation::SuggestorCapability
//! [`CatalogSuggestorDescriptor`]: organism_catalog::CatalogSuggestorDescriptor

pub mod converge;
pub mod mosaic;
pub mod organism;

mod support;
pub(crate) use support::{EntrySpec, entry};

use organism_catalog::DiscoveryCatalog;

/// Returns a [`DiscoveryCatalog`] pre-seeded with every tree's
/// descriptors. The order is Converge → Organism → mosaic; lookup
/// primitives don't care about insertion order, but the trace is
/// reproducible run to run.
#[must_use]
pub fn all() -> DiscoveryCatalog {
    let mut catalog = DiscoveryCatalog::new();
    for entry in converge::descriptors() {
        catalog.register(entry);
    }
    for entry in organism::descriptors() {
        catalog.register(entry);
    }
    for entry in mosaic::descriptors() {
        catalog.register(entry);
    }
    catalog
}

/// Returns a [`DiscoveryCatalog`] seeded with the Converge tree only.
#[must_use]
pub fn converge_only() -> DiscoveryCatalog {
    let mut catalog = DiscoveryCatalog::new();
    for entry in converge::descriptors() {
        catalog.register(entry);
    }
    catalog
}

/// Returns a [`DiscoveryCatalog`] seeded with the Organism tree only.
#[must_use]
pub fn organism_only() -> DiscoveryCatalog {
    let mut catalog = DiscoveryCatalog::new();
    for entry in organism::descriptors() {
        catalog.register(entry);
    }
    catalog
}

/// Returns a [`DiscoveryCatalog`] seeded with the mosaic-extensions
/// tree only.
#[must_use]
pub fn mosaic_only() -> DiscoveryCatalog {
    let mut catalog = DiscoveryCatalog::new();
    for entry in mosaic::descriptors() {
        catalog.register(entry);
    }
    catalog
}

/// Union of every tree's `pinned_to` manifest. Each entry is
/// `(crate_name, version)` of a source crate the seed targets. Hosts
/// can assert this against their actual workspace pins at startup to
/// catch drift.
#[must_use]
pub fn pinned_to() -> Vec<(&'static str, &'static str)> {
    let mut all = Vec::new();
    all.extend_from_slice(converge::pinned_to());
    all.extend_from_slice(organism::pinned_to());
    all.extend_from_slice(mosaic::pinned_to());
    all
}
