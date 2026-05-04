//! # Organism Runtime
//!
//! The formation guru. Given an intent, assembles teams of heterogeneous
//! agents and runs them in Converge Engine instances.
//!
//! There is ONE model: everything is a Suggestor. Adversarial review,
//! simulation, planning, policy, optimization — all participate in the
//! same convergence loop. No side-car pipelines.
//!
//! ```text
//! Intent → Admit → Form (pick Suggestors) → Engine.run() → Evaluate → Learn
//!                    ↑                                          ↓
//!                    └──── reform if needed ────────────────────┘
//! ```

pub mod collaboration;
pub mod compiler;
pub mod execution;
pub mod experience;
pub mod formation;
pub mod huddle;
pub mod outcome;
pub mod readiness;
pub mod registry;
pub mod tournament;
pub mod vendor_selection;

pub use collaboration::{
    CollaborationParticipant, CollaborationRunner, CollaborationRunnerError, TransitionRecord,
};
pub use compiler::{
    CompiledFormationPlan, CompiledSuggestorRole, DataContract, FormationCompileError,
    FormationCompileRequest, FormationCompiler, FormationCompilerCatalogs, GovernanceClass,
    ProviderDescriptor, ProviderDescriptorCatalog, ReplayMode, RoleProviderAssignment,
    SuggestorDescriptor, SuggestorDescriptorCatalog,
};
pub use execution::{
    ExecutableSuggestorCatalog, FormationExecutionRecord, FormationInstantiationError,
};
pub use experience::{ExperienceEnvelopeSink, FormationExperienceObserver};
pub use formation::{Formation, FormationError, FormationResult, Seed};
pub use huddle::ConsensusEvaluator;
pub use organism_pack::{
    CapabilityRequirement, DeclarativeBinding, IntentBinding, IntentResolver, PackRequirement,
    ResolutionLevel, ResolutionTrace,
};
pub use outcome::{
    BusinessQualitySignal, FormationOutcomeRecord, FormationOutcomeStatus, FormationRunScope,
    OutcomeProviderAssignment, OutcomeRosterMember, QualityScoreBps, QualityScoreError,
};
pub use readiness::{
    BudgetProbe, CredentialProbe, GapSeverity, PackProbe, ReadinessConfirmation, ReadinessGap,
    ReadinessItem, ReadinessProbe, ReadinessReport, ResourceKind, check as check_readiness,
};
pub use registry::{RegisteredCapability, RegisteredPack, Registry, StructuralResolver};
pub use tournament::{FormationScore, FormationTournament, TournamentError, TournamentResult};
pub use vendor_selection::{
    VendorSelectionFlow, VendorSelectionFlowSpec, vendor_selection_formation_catalog,
    vendor_selection_lifecycle,
};

use organism_intent::admission::{self, Admission};
use organism_pack::IntentPacket;
use std::sync::Arc;

/// Outcome of the full organism pipeline.
#[derive(Debug)]
pub struct OrganismResult {
    /// The formation that produced the winning result.
    pub winning_formation: String,
    /// Converge result from the winning run.
    pub converge_result: converge_kernel::ConvergeResult,
}

/// Why the pipeline rejected an intent or formation.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("admission rejected: {0}")]
    Rejected(String),
    #[error("formation compile error: {0}")]
    Compile(#[from] FormationCompileError),
    #[error("formation instantiation error: {0}")]
    Instantiate(#[from] FormationInstantiationError),
    #[error("all formations failed: {0}")]
    AllFormationsFailed(String),
    #[error("formation error: {0}")]
    Formation(#[from] FormationError),
}

/// The formation guru.
///
/// Organism's runtime does exactly three things:
/// 1. Quick admission gate (is the intent even valid?)
/// 2. Run formations in Converge (each is a team of heterogeneous Suggestors)
/// 3. Pick the winner
///
/// Everything else — adversarial review, simulation, planning, policy checks —
/// happens INSIDE the formation as Suggestors in the convergence loop.
pub struct Runtime;

impl Runtime {
    pub fn new() -> Self {
        Self
    }

    /// Admit an intent and compile the formation plan Organism would run.
    ///
    /// This is the pure compiler boundary: descriptor catalogs produce an
    /// auditable formation plan without creating live suggestor instances.
    pub fn compile_formation(
        &self,
        intent: &IntentPacket,
        request: &FormationCompileRequest,
        catalogs: &FormationCompilerCatalogs,
    ) -> Result<CompiledFormationPlan, PipelineError> {
        admit_intent(intent)?;
        Ok(FormationCompiler::new().compile(request, catalogs)?)
    }

    /// Admit, compile, and instantiate a runnable formation from registered
    /// executable suggestor factories.
    ///
    /// This keeps the boundary honest: a plan can run only when every compiled
    /// `suggestor_id` has a concrete factory in `executables`.
    pub fn compile_and_instantiate_formation(
        &self,
        intent: &IntentPacket,
        request: &FormationCompileRequest,
        catalogs: &FormationCompilerCatalogs,
        executables: &ExecutableSuggestorCatalog,
        seeds: impl IntoIterator<Item = Seed>,
    ) -> Result<(CompiledFormationPlan, Formation), PipelineError> {
        let plan = self.compile_formation(intent, request, catalogs)?;
        let formation = executables.instantiate(&plan, seeds)?;
        Ok((plan, formation))
    }

    /// Admit, compile, instantiate, and run one formation candidate.
    ///
    /// This is the single-candidate execution path. Tournaments can build on
    /// top of this by running multiple compile requests and comparing returned
    /// `FormationExecutionRecord` values.
    pub async fn compile_and_run_formation(
        &self,
        intent: &IntentPacket,
        request: &FormationCompileRequest,
        catalogs: &FormationCompilerCatalogs,
        executables: &ExecutableSuggestorCatalog,
        seeds: impl IntoIterator<Item = Seed>,
        observer: Option<Arc<dyn converge_kernel::ExperienceEventObserver>>,
    ) -> Result<FormationExecutionRecord, PipelineError> {
        let (plan, formation) =
            self.compile_and_instantiate_formation(intent, request, catalogs, executables, seeds)?;
        let result = if let Some(observer) = observer {
            formation.run_with_event_observer(observer).await?
        } else {
            formation.run().await?
        };

        Ok(FormationExecutionRecord::from_plan_and_result(plan, result))
    }

    /// Drive an intent through the pipeline.
    ///
    /// The caller is responsible for assembling formations (teams of Suggestors).
    /// That's the formation-guru logic — deciding which agents to include based
    /// on the intent's characteristics, available capabilities, and learned priors.
    ///
    /// Each formation may include any mix of:
    /// - LLM reasoning agents
    /// - Optimization solvers
    /// - Policy gates
    /// - Analytics/ML agents
    /// - Adversarial skeptics
    /// - Domain-specific pack agents
    ///
    /// All participate through the same `Suggestor` trait. Same contract,
    /// same governance, same convergence loop.
    pub async fn handle(
        &self,
        intent: IntentPacket,
        formations: Vec<Formation>,
    ) -> Result<OrganismResult, PipelineError> {
        // 1. Admission — the one imperative check that stays outside the loop.
        //    Is the intent structurally valid? Not expired? Not empty?
        admit_intent(&intent)?;

        // 2. Run formations (concurrently in the future; sequential for now).
        //    Each formation is a complete Converge Engine run with its own
        //    team of Suggestors. Adversarial agents, simulators, planners —
        //    they're all in there, converging together.
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for formation in formations {
            match formation.run().await {
                Ok(result) => results.push(result),
                Err(e) => errors.push(e.to_string()),
            }
        }

        if results.is_empty() {
            return Err(PipelineError::AllFormationsFailed(errors.join("; ")));
        }

        // 3. Pick the winner.
        //    Future: evaluate competing results via learned quality metrics,
        //    convergence quality, cycle count, fact coverage.
        let winner = results.into_iter().next().unwrap();

        Ok(OrganismResult {
            winning_formation: winner.label,
            converge_result: winner.converge_result,
        })
    }
}

fn admit_intent(intent: &IntentPacket) -> Result<(), PipelineError> {
    match admission::admit(intent) {
        Admission::Admit => Ok(()),
        Admission::Reject(err) => Err(PipelineError::Rejected(err.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use converge_kernel::formation::{
        FormationTemplateQuery, ProfileSnapshot, SuggestorCapability, SuggestorRole,
    };
    use converge_kernel::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
    use converge_provider_api::{BackendRequirements, CostClass, LatencyClass};

    fn id(n: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(n)
    }

    fn valid_intent() -> IntentPacket {
        IntentPacket::new("select the best AI vendor", Utc::now() + Duration::hours(1))
    }

    fn profile(
        name: &str,
        role: SuggestorRole,
        writes: Vec<ContextKey>,
        capabilities: Vec<SuggestorCapability>,
    ) -> ProfileSnapshot {
        ProfileSnapshot {
            name: name.to_string(),
            role,
            output_keys: writes,
            cost_hint: CostClass::Low,
            latency_hint: LatencyClass::Interactive,
            capabilities,
            confidence_min: 0.7,
            confidence_max: 0.95,
        }
    }

    fn request() -> FormationCompileRequest {
        FormationCompileRequest::new(
            id(1),
            id(2),
            FormationTemplateQuery::new()
                .with_keyword("diligence-evaluate-decide")
                .with_entity("VendorSelectionDecisionRecord"),
        )
        .with_tenant_id("tenant-a")
        .with_domain_tag("vendor-selection")
    }

    fn catalogs() -> FormationCompilerCatalogs {
        let policy_requirements = BackendRequirements::access_policy().with_replay();
        FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(
                SuggestorDescriptor::new(
                    "market-scan",
                    profile(
                        "market-scan",
                        SuggestorRole::Signal,
                        vec![ContextKey::Signals],
                        vec![SuggestorCapability::KnowledgeRetrieval],
                    ),
                )
                .with_domain_tag("vendor-selection"),
            )
            .with_suggestor(
                SuggestorDescriptor::new(
                    "weighted-evaluator",
                    profile(
                        "weighted-evaluator",
                        SuggestorRole::Evaluation,
                        vec![ContextKey::Evaluations],
                        vec![SuggestorCapability::Analytics],
                    ),
                )
                .with_domain_tag("vendor-selection"),
            )
            .with_suggestor(
                SuggestorDescriptor::new(
                    "policy-gate",
                    profile(
                        "policy-gate",
                        SuggestorRole::Constraint,
                        vec![ContextKey::Constraints],
                        vec![SuggestorCapability::PolicyEnforcement],
                    ),
                )
                .with_domain_tag("vendor-selection")
                .with_backend_requirements(policy_requirements.clone()),
            )
            .with_suggestor(
                SuggestorDescriptor::new(
                    "decision-synthesis",
                    profile(
                        "decision-synthesis",
                        SuggestorRole::Synthesis,
                        vec![ContextKey::Proposals],
                        vec![SuggestorCapability::LlmReasoning],
                    ),
                )
                .with_domain_tag("vendor-selection"),
            )
            .with_provider(
                ProviderDescriptor::new(
                    "cedar-local",
                    "Cedar local policy engine",
                    policy_requirements,
                )
                .with_role_affinity(SuggestorRole::Constraint)
                .with_domain_tag("vendor-selection"),
            )
    }

    struct FixtureSuggestor {
        name: &'static str,
        dependencies: Vec<ContextKey>,
        output: ContextKey,
    }

    impl FixtureSuggestor {
        fn new(name: &'static str, dependencies: Vec<ContextKey>, output: ContextKey) -> Self {
            Self {
                name,
                dependencies,
                output,
            }
        }
    }

    #[async_trait::async_trait]
    impl Suggestor for FixtureSuggestor {
        fn name(&self) -> &str {
            self.name
        }

        fn dependencies(&self) -> &[ContextKey] {
            &self.dependencies
        }

        fn accepts(&self, ctx: &dyn Context) -> bool {
            self.dependencies.iter().any(|key| ctx.has(*key)) && !ctx.has(self.output)
        }

        async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
            AgentEffect::with_proposal(ProposedFact::new(
                self.output,
                format!("{}-output", self.name),
                format!("{} produced compiled-role output", self.name),
                self.name,
            ))
        }
    }

    fn executable_catalog() -> ExecutableSuggestorCatalog {
        let mut catalog = ExecutableSuggestorCatalog::new();
        catalog
            .register_factory("market-scan", || {
                FixtureSuggestor::new("market-scan", vec![ContextKey::Seeds], ContextKey::Signals)
            })
            .expect("market-scan factory");
        catalog
            .register_factory("weighted-evaluator", || {
                FixtureSuggestor::new(
                    "weighted-evaluator",
                    vec![ContextKey::Signals],
                    ContextKey::Evaluations,
                )
            })
            .expect("weighted-evaluator factory");
        catalog
            .register_factory("policy-gate", || {
                FixtureSuggestor::new(
                    "policy-gate",
                    vec![ContextKey::Evaluations],
                    ContextKey::Constraints,
                )
            })
            .expect("policy-gate factory");
        catalog
            .register_factory("decision-synthesis", || {
                FixtureSuggestor::new(
                    "decision-synthesis",
                    vec![ContextKey::Evaluations, ContextKey::Constraints],
                    ContextKey::Proposals,
                )
            })
            .expect("decision-synthesis factory");
        catalog
    }

    #[test]
    fn runtime_compiles_after_admission() {
        let plan = Runtime::new()
            .compile_formation(&valid_intent(), &request(), &catalogs())
            .expect("valid vendor-selection intent should compile");

        assert_eq!(plan.template_id, "vendor-selection-decide");
        assert_eq!(plan.correlation_id, id(2));
        assert_eq!(plan.tenant_id.as_deref(), Some("tenant-a"));
    }

    #[test]
    fn runtime_rejects_invalid_intent_before_compile() {
        let invalid_intent = IntentPacket::new("   ", Utc::now() + Duration::hours(1));

        let error = Runtime::new()
            .compile_formation(&invalid_intent, &request(), &catalogs())
            .expect_err("blank intent should fail admission");

        assert!(matches!(error, PipelineError::Rejected(_)));
    }

    #[tokio::test]
    async fn runtime_compiles_and_runs_executable_plan_with_outcome_record() {
        let seed = Seed {
            key: ContextKey::Seeds,
            id: "vendor-selection-intent".into(),
            content: "select the AI governance vendor".to_string(),
            provenance: "runtime-test".to_string(),
        };

        let record = Runtime::new()
            .compile_and_run_formation(
                &valid_intent(),
                &request(),
                &catalogs(),
                &executable_catalog(),
                vec![seed],
                None,
            )
            .await
            .expect("plan should compile and run");

        assert_eq!(record.plan.template_id, "vendor-selection-decide");
        assert_eq!(record.outcome.status, FormationOutcomeStatus::Converged);
        assert_eq!(record.outcome.scope.correlation_id, id(2));

        assert!(record.result.converge_result.converged);
        assert!(
            record
                .result
                .converge_result
                .context
                .has(ContextKey::Signals)
        );
        assert!(
            record
                .result
                .converge_result
                .context
                .has(ContextKey::Evaluations)
        );
        assert!(
            record
                .result
                .converge_result
                .context
                .has(ContextKey::Constraints)
        );
        assert!(
            record
                .result
                .converge_result
                .context
                .has(ContextKey::Proposals)
        );
    }

    #[test]
    fn runtime_reports_missing_executable_factories() {
        let seed = Seed {
            key: ContextKey::Seeds,
            id: "vendor-selection-intent".into(),
            content: "select the AI governance vendor".to_string(),
            provenance: "runtime-test".to_string(),
        };
        let mut partial = ExecutableSuggestorCatalog::new();
        partial
            .register_factory("market-scan", || {
                FixtureSuggestor::new("market-scan", vec![ContextKey::Seeds], ContextKey::Signals)
            })
            .expect("market-scan factory");

        let Err(error) = Runtime::new().compile_and_instantiate_formation(
            &valid_intent(),
            &request(),
            &catalogs(),
            &partial,
            vec![seed],
        ) else {
            panic!("missing executable factories should fail explicitly");
        };

        match error {
            PipelineError::Instantiate(
                FormationInstantiationError::MissingSuggestorFactories { suggestor_ids },
            ) => {
                assert!(suggestor_ids.contains(&"weighted-evaluator".to_string()));
                assert!(suggestor_ids.contains(&"policy-gate".to_string()));
                assert!(suggestor_ids.contains(&"decision-synthesis".to_string()));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
