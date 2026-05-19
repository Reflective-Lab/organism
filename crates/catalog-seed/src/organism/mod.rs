//! Descriptors for production Suggestors shipped by Organism crates.
//!
//! Authored against Organism `1.9.1`. Includes the full production set
//! across the adversarial, dynamics, learning, planning, runtime, and
//! simulation crates — both the long-standing reasoners and the
//! 1.9.x additions (AnomalySkeptic, the design-huddle Suggestors).
//! Excludes test fixtures, bench agents, and wrappers like
//! `BoxedAgent`.

mod adversarial;
mod dynamics;
mod learning;
mod planning;
mod runtime;
mod simulation;

use organism_catalog::CatalogSuggestorDescriptor;

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    let mut all = Vec::new();
    all.extend(adversarial::descriptors());
    all.extend(dynamics::descriptors());
    all.extend(learning::descriptors());
    all.extend(planning::descriptors());
    all.extend(runtime::descriptors());
    all.extend(simulation::descriptors());
    all
}

#[must_use]
pub const fn pinned_to() -> &'static [(&'static str, &'static str)] {
    &[
        ("organism-adversarial", "1.9.1"),
        ("organism-dynamics", "1.9.1"),
        ("organism-learning", "1.9.1"),
        ("organism-planning", "1.9.1"),
        ("organism-runtime", "1.9.1"),
        ("organism-simulation", "1.9.1"),
    ]
}
