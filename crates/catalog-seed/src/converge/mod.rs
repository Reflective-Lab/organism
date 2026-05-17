//! Descriptors for public Suggestors shipped by Converge crates.
//!
//! Authored against Converge `3.9.1`. Includes:
//! - `converge-optimization::suggestors::*` (Formation assembly, OR
//!   solvers, scheduling/portfolio/routing primitives).
//! - `converge-kernel::ProviderSelectionSuggestor`.
//!
//! Excludes the internal `engine.rs` test fixtures (`SeedSuggestor`,
//! `ReactOnceSuggestor`, etc.) — those are pre-canned helpers used by
//! Converge's own tests, not "pick me for this task" Suggestors.

mod kernel;
mod optimization;

use organism_catalog::CatalogSuggestorDescriptor;

/// Returns every Converge first-party descriptor in this seed.
#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    let mut all = Vec::new();
    all.extend(optimization::descriptors());
    all.extend(kernel::descriptors());
    all
}

/// Converge crate versions these descriptors were authored against.
#[must_use]
pub const fn pinned_to() -> &'static [(&'static str, &'static str)] {
    &[
        ("converge-kernel", "3.9.1"),
        ("converge-optimization", "3.9.1"),
        ("converge-pack", "3.9.1"),
        ("converge-provider", "3.9.1"),
    ]
}
