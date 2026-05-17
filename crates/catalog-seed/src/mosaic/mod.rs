//! Descriptors for Suggestors shipped by mosaic-extensions crates.
//!
//! Authored against the published mosaic versions on crates.io as of
//! 2026-05-17. See `pinned_to()` for the exact version manifest.

pub mod arbiter;
pub mod embassy;
pub mod ferrox;
pub mod mnemos;
pub mod prism;
pub mod soter;

use organism_catalog::CatalogSuggestorDescriptor;

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    let mut all = Vec::new();
    all.extend(arbiter::descriptors());
    all.extend(embassy::descriptors());
    all.extend(ferrox::descriptors());
    all.extend(mnemos::descriptors());
    all.extend(prism::descriptors());
    all.extend(soter::descriptors());
    all
}

#[must_use]
pub const fn pinned_to() -> &'static [(&'static str, &'static str)] {
    &[
        ("converge-arbiter-policy", "2.0.1"),
        ("converge-embassy-pack", "1.3.0"),
        ("converge-embassy-linkedin", "1.3.0"),
        ("converge-ferrox-solver", "0.7.1"),
        ("converge-manifold-adapters", "1.1.1"),
        ("converge-mnemos-knowledge", "1.2.2"),
        ("converge-prism-analytics", "2.0.0"),
        ("converge-soter-smt", "0.2.2"),
    ]
}
