//! Default executable factories for the well-known catalog ids.
//!
//! [`ExecutableSuggestorCatalog`] is host-wired by design: the host
//! crate (typically a `marquee-apps` binary) decides which factories
//! to register. That keeps the runtime free of strong link-level
//! dependencies on every adapter, and lets hosts pick narrower
//! constructors (configured constraint rules, calibrated thresholds,
//! domain-specific synthesizer producers).
//!
//! For the round-driven design huddle and other Formations that just
//! want "real Suggestors from the seed without writing per-host
//! wiring," this module ships sensible-default factories for the
//! subset of catalog ids whose Suggestors expose a zero-arg or
//! `default_config()` constructor:
//!
//! * `organism-assumption-breaker` — [`AssumptionBreakerAgent::new`]
//! * `organism-constraint-checker` — [`ConstraintCheckerAgent::default_config`]
//!   (empty constraint set; replace via a custom factory for real rules)
//! * `organism-economic-skeptic` — [`EconomicSkepticAgent::default_config`]
//! * `organism-operational-skeptic` — [`OperationalSkepticAgent::default_config`]
//! * `organism-anomaly-skeptic` — [`AnomalySkepticAgent::default_config`]
//! * `organism-disagreement-mapper` — [`DisagreementMapper::new`]
//!
//! Suggestors that require a host decision at construction time
//! (e.g. `RoundStarter::new(max_rounds)`,
//! `ConsensusEvaluator::new(rule, total_voters)`,
//! `RoundSynthesizer::new(expected, producer)`) are deliberately
//! **not** registered here — the host must choose the configuration
//! and call `catalog.register_factory(...)` itself.

use organism_adversarial::{
    AnomalySkepticAgent, AssumptionBreakerAgent, ConstraintCheckerAgent, EconomicSkepticAgent,
    OperationalSkepticAgent,
};

use crate::execution::{ExecutableSuggestorCatalog, FormationInstantiationError};
use crate::huddle::DisagreementMapper;

/// Register the default-config zero-arg factories listed in the
/// module docs into `catalog`. Idempotent only on a fresh catalog —
/// duplicate-id errors propagate from
/// [`ExecutableSuggestorCatalog::register_factory`] so a host that
/// has already registered one of these ids (with a stricter config,
/// for instance) gets a clear error instead of a silent override.
pub fn register_default_factories(
    catalog: &mut ExecutableSuggestorCatalog,
) -> Result<(), FormationInstantiationError> {
    catalog.register_factory("organism-assumption-breaker", AssumptionBreakerAgent::new)?;
    catalog.register_factory(
        "organism-constraint-checker",
        ConstraintCheckerAgent::default_config,
    )?;
    catalog.register_factory(
        "organism-economic-skeptic",
        EconomicSkepticAgent::default_config,
    )?;
    catalog.register_factory(
        "organism-operational-skeptic",
        OperationalSkepticAgent::default_config,
    )?;
    catalog.register_factory(
        "organism-anomaly-skeptic",
        AnomalySkepticAgent::default_config,
    )?;
    catalog.register_factory("organism-disagreement-mapper", DisagreementMapper::new)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_factories_register_without_error() {
        let mut catalog = ExecutableSuggestorCatalog::new();
        register_default_factories(&mut catalog).expect("default registration must succeed");
    }

    #[test]
    fn default_factories_cover_the_documented_ids() {
        let mut catalog = ExecutableSuggestorCatalog::new();
        register_default_factories(&mut catalog).unwrap();
        for id in [
            "organism-assumption-breaker",
            "organism-constraint-checker",
            "organism-economic-skeptic",
            "organism-operational-skeptic",
            "organism-anomaly-skeptic",
            "organism-disagreement-mapper",
        ] {
            assert!(
                catalog.contains(id),
                "default registration missing id: {id}"
            );
        }
    }

    #[test]
    fn duplicate_registration_returns_error_not_silent_override() {
        let mut catalog = ExecutableSuggestorCatalog::new();
        register_default_factories(&mut catalog).unwrap();
        // Second call must fail loudly on the first duplicate id.
        let result = register_default_factories(&mut catalog);
        assert!(matches!(
            result,
            Err(FormationInstantiationError::DuplicateSuggestorFactory { .. })
        ));
    }
}
