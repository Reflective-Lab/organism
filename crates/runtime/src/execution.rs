//! Execution bridge from compiled formation plans to runnable formations.
//!
//! The compiler produces an auditable plan. This module keeps the next boundary
//! explicit: a compiled `suggestor_id` is executable only when the embedding app
//! registered a concrete factory for it.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use converge_kernel::{StopReason, Suggestor};

use crate::compiler::CompiledFormationPlan;
use crate::formation::{Formation, FormationResult, Seed};
use crate::outcome::{FormationOutcomeRecord, FormationOutcomeStatus};

type SuggestorFactory = Arc<dyn Fn() -> Box<dyn Suggestor> + Send + Sync>;

/// Registry of concrete suggestor factories that Organism may instantiate.
#[derive(Clone, Default)]
pub struct ExecutableSuggestorCatalog {
    factories: BTreeMap<String, SuggestorFactory>,
}

/// Result of compiling, instantiating, and running one formation candidate.
pub struct FormationExecutionRecord {
    pub plan: CompiledFormationPlan,
    pub result: FormationResult,
    pub outcome: FormationOutcomeRecord,
}

impl ExecutableSuggestorCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a factory for a compiled `suggestor_id`.
    pub fn register_factory<S, F>(
        &mut self,
        suggestor_id: impl Into<String>,
        factory: F,
    ) -> Result<(), FormationInstantiationError>
    where
        S: Suggestor + 'static,
        F: Fn() -> S + Send + Sync + 'static,
    {
        self.register_boxed_factory(suggestor_id, move || Box::new(factory()))
    }

    /// Register a boxed factory for suggestors that already erase their type.
    pub fn register_boxed_factory<F>(
        &mut self,
        suggestor_id: impl Into<String>,
        factory: F,
    ) -> Result<(), FormationInstantiationError>
    where
        F: Fn() -> Box<dyn Suggestor> + Send + Sync + 'static,
    {
        let suggestor_id = suggestor_id.into();
        if self.factories.contains_key(&suggestor_id) {
            return Err(FormationInstantiationError::DuplicateSuggestorFactory { suggestor_id });
        }

        self.factories.insert(suggestor_id, Arc::new(factory));
        Ok(())
    }

    #[must_use]
    pub fn contains(&self, suggestor_id: &str) -> bool {
        self.factories.contains_key(suggestor_id)
    }

    #[must_use]
    pub fn suggestor_ids(&self) -> Vec<&str> {
        self.factories.keys().map(String::as_str).collect()
    }

    /// Instantiate a compiled plan into a runnable formation.
    ///
    /// Provider assignments remain part of the compiled plan and outcome record.
    /// Concrete provider clients should be captured by the registered factories.
    pub fn instantiate(
        &self,
        plan: &CompiledFormationPlan,
        seeds: impl IntoIterator<Item = Seed>,
    ) -> Result<Formation, FormationInstantiationError> {
        let missing = plan
            .roster
            .iter()
            .filter(|member| !self.factories.contains_key(&member.suggestor_id))
            .map(|member| member.suggestor_id.clone())
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            return Err(FormationInstantiationError::MissingSuggestorFactories {
                suggestor_ids: missing,
            });
        }

        let mut formation = Formation::new(plan.template_id.clone());
        for seed in seeds {
            formation = formation.seed(seed.key, seed.id, seed.content, seed.provenance);
        }

        for member in &plan.roster {
            let factory = self.factories.get(&member.suggestor_id).ok_or_else(|| {
                FormationInstantiationError::MissingSuggestorFactories {
                    suggestor_ids: vec![member.suggestor_id.clone()],
                }
            })?;
            formation = formation.agent_boxed(factory());
        }

        Ok(formation)
    }
}

impl FormationExecutionRecord {
    #[must_use]
    pub fn from_plan_and_result(plan: CompiledFormationPlan, result: FormationResult) -> Self {
        let status = outcome_status(
            &result.converge_result.stop_reason,
            result.converge_result.converged,
        );
        let outcome = FormationOutcomeRecord::from_compiled_plan(&plan, status)
            .with_stop_reason(format!("{:?}", result.converge_result.stop_reason));

        Self {
            plan,
            result,
            outcome,
        }
    }
}

impl fmt::Debug for ExecutableSuggestorCatalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecutableSuggestorCatalog")
            .field("suggestor_ids", &self.suggestor_ids())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormationInstantiationError {
    #[error("executable suggestor factory already registered for '{suggestor_id}'")]
    DuplicateSuggestorFactory { suggestor_id: String },
    #[error(
        "compiled formation references suggestors without executable factories: {suggestor_ids:?}"
    )]
    MissingSuggestorFactories { suggestor_ids: Vec<String> },
}

fn outcome_status(stop_reason: &StopReason, converged: bool) -> FormationOutcomeStatus {
    match stop_reason {
        StopReason::Converged if converged => FormationOutcomeStatus::Converged,
        StopReason::CriteriaMet { .. } => FormationOutcomeStatus::Converged,
        StopReason::HumanInterventionRequired { .. } | StopReason::HitlGatePending { .. } => {
            FormationOutcomeStatus::NeedsReview
        }
        StopReason::CycleBudgetExhausted { .. }
        | StopReason::FactBudgetExhausted { .. }
        | StopReason::TokenBudgetExhausted { .. }
        | StopReason::TimeBudgetExhausted { .. } => FormationOutcomeStatus::BudgetExhausted,
        StopReason::InvariantViolated { .. } | StopReason::PromotionRejected { .. } => {
            FormationOutcomeStatus::CriteriaBlocked
        }
        StopReason::UserCancelled | StopReason::AgentRefused { .. } | StopReason::Error { .. } => {
            FormationOutcomeStatus::Failed
        }
        _ => FormationOutcomeStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{AgentEffect, Context, ContextKey, ProposedFact};

    const SEED_DEPENDENCIES: &[ContextKey] = &[ContextKey::Seeds];

    struct TestSuggestor {
        name: &'static str,
    }

    #[async_trait::async_trait]
    impl Suggestor for TestSuggestor {
        fn name(&self) -> &'static str {
            self.name
        }

        fn dependencies(&self) -> &[ContextKey] {
            SEED_DEPENDENCIES
        }

        fn accepts(&self, ctx: &dyn Context) -> bool {
            ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
        }

        async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
            let seed = &ctx.get(ContextKey::Seeds)[0];
            AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Hypotheses,
                format!("{}-{}", self.name, seed.id),
                "instantiated suggestor ran",
                self.name,
            ))
        }
    }

    #[test]
    fn rejects_duplicate_factories() {
        let mut catalog = ExecutableSuggestorCatalog::new();
        catalog
            .register_factory("dup", || TestSuggestor { name: "dup-a" })
            .expect("first registration should succeed");

        let error = catalog
            .register_factory("dup", || TestSuggestor { name: "dup-b" })
            .expect_err("duplicate registration should fail");

        assert_eq!(
            error,
            FormationInstantiationError::DuplicateSuggestorFactory {
                suggestor_id: "dup".to_string()
            }
        );
    }
}
