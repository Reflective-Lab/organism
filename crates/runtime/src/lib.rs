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

pub mod classifier;
pub mod collaboration;
pub mod compiler;
pub mod execution;
pub mod experience;
pub mod formation;
pub mod guru;
pub mod huddle;
pub mod outcome;
pub mod provenance;
pub mod readiness;
pub mod registry;
pub mod stall;
pub mod templates;
pub mod tournament;
pub mod vendor_selection;

pub use classifier::{ProblemClassifierSuggestor, extract_classification};
pub use collaboration::{
    CollaborationParticipant, CollaborationRunner, CollaborationRunnerError, TransitionRecord,
};
pub use compiler::{
    CandidateConsideration, CandidateDisposition, CatalogCompileFailure,
    CatalogCompiledFormationPlan, CompiledFormationPlan, CompiledSuggestorRole, DataContract,
    FormationCompileError, FormationCompileRequest, FormationCompiler, FormationCompilerCatalogs,
    GovernanceClass, ProviderDescriptor, ProviderDescriptorCatalog, RejectionReason, ReplayMode,
    RoleDecision, RoleProviderAssignment, SelectionReason, SuggestorDescriptor,
    SuggestorDescriptorCatalog,
};
pub use execution::{
    ExecutableSuggestorCatalog, FormationExecutionRecord, FormationInstantiationError,
};
pub use experience::{ExperienceEnvelopeSink, FormationExperienceObserver};
pub use formation::{Formation, FormationError, FormationResult, Seed};
pub use guru::{CandidateScore, FormationGuru, GuruError, GuruSelection, SelectionTrace};
pub use huddle::{
    ConsensusEvaluator, DisagreementMap, DisagreementMapper, RoundConventions, RoundStarter,
    RoundSynthesizer, SynthesisProducer, TerminalPredicate,
};
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
pub use stall::RoleStallSuggestor;
pub use templates::{
    CostHint, cost_hint_for, decision_formation, diligence_formation, evaluation_formation,
    planning_formation, research_formation, standard_formation_catalog, template_id_for,
};
pub use tournament::{FormationScore, FormationTournament, TournamentError, TournamentResult};
pub use vendor_selection::{
    VendorSelectionFlow, VendorSelectionFlowSpec, vendor_selection_formation_catalog,
    vendor_selection_lifecycle,
};

use organism_catalog::DiscoveryCatalog;

use converge_kernel::admission::{
    AdmissionActor, AdmissionContent, AdmissionError, AdmissionReceipt, AdmissionRequest,
    AdmissionSource, admit_observation,
};
use converge_kernel::formation::{FormationCatalog, SuggestorCapability};
use converge_kernel::{ContextKey, ContextState, ConvergeError};
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

/// A single scored catalog-sourced candidate. Pairs the per-role
/// decision trace (why this roster was chosen) with the tournament
/// score (how it performed). Indexed-paired entries are how callers
/// join "selection rationale" to "score outcome" without parsing
/// labels.
#[derive(Debug, Clone)]
pub struct ScoredCatalogCandidate {
    /// Stable index 0..k matching position in the originating
    /// `compile_k_candidates` call. The label of the underlying
    /// `Formation` was set to `format!("{template_id}#{index}")` at
    /// instantiation so the tournament's `FormationScore.label` can be
    /// joined back here unambiguously.
    pub index: usize,
    pub candidate: CatalogCompiledFormationPlan,
    pub score: FormationScore,
}

/// Result of [`Runtime::compile_k_and_run_tournament`]. Pairs each
/// candidate's selection rationale (decisions) with its tournament
/// score so the audit trail can show *why* each roster was chosen
/// alongside *how* it performed. Pair-by-index is the join key — the
/// tournament's `FormationScore.label` is `{template_id}#{index}` for
/// candidate at that index.
///
/// `winner_index` is the *original candidate index* (0..k) of the
/// winner, not a position inside `scored_candidates`. Use
/// [`Self::winner`] to dereference safely — if any non-winning
/// candidate failed at runtime, the tournament drops it from
/// `scored_candidates`, and looking up by position would index a
/// different (or invalid) element.
#[derive(Debug, Clone)]
pub struct CatalogTournamentOutcome {
    /// Original candidate index (0..k) of the tournament winner.
    pub winner_index: usize,
    /// Candidates that produced a scored result. Ordered by `index`
    /// ascending; may be shorter than the originating `k` if any
    /// candidate's formation failed at run time.
    pub scored_candidates: Vec<ScoredCatalogCandidate>,
    /// Calibrated priors ready to feed the next planning prior agent.
    pub priors: Vec<organism_learning::PriorCalibration>,
}

impl CatalogTournamentOutcome {
    /// Returns the winning [`ScoredCatalogCandidate`] by matching
    /// `winner_index` against each entry's `index`. Safe against
    /// partial tournament failures where some candidates were dropped
    /// from `scored_candidates`.
    #[must_use]
    pub fn winner(&self) -> &ScoredCatalogCandidate {
        self.scored_candidates
            .iter()
            .find(|sc| sc.index == self.winner_index)
            .expect("winner_index must correspond to a scored candidate")
    }
}

/// Why the pipeline rejected an intent or formation.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("admission rejected: {0}")]
    Rejected(String),
    #[error("formation compile error: {0}")]
    Compile(#[from] FormationCompileError),
    /// Catalog-aware compile failure. Carries the partial per-role
    /// decision trace so callers can explain why the requirement could
    /// not be satisfied.
    #[error("catalog compile error: {0}")]
    CatalogCompile(#[from] CatalogCompileFailure),
    #[error("formation instantiation error: {0}")]
    Instantiate(#[from] FormationInstantiationError),
    #[error("all formations failed: {0}")]
    AllFormationsFailed(String),
    #[error("formation error: {0}")]
    Formation(#[from] FormationError),
    /// Tournament error (e.g. no formations to score, all failed).
    #[error("tournament error: {0}")]
    Tournament(String),
}

/// Why an IntentPacket failed organism's structural gate or Converge's typed
/// admission boundary.
#[derive(Debug, thiserror::Error)]
pub enum IntentAdmissionError {
    /// Organism's structural admission gate rejected the IntentPacket.
    #[error("admission rejected: {0}")]
    Rejected(String),
    /// Constructing the Converge admission request failed (empty actor / source / id / content).
    #[error("admission request invalid: {0}")]
    AdmissionRequest(#[from] AdmissionError),
    /// Serializing the IntentPacket payload failed.
    #[error("intent payload could not be serialized: {0}")]
    Serialize(String),
    /// `converge_kernel::admission::admit_observation` rejected the staged proposal.
    #[error("converge admission failed: {0}")]
    Converge(String),
}

impl From<ConvergeError> for IntentAdmissionError {
    fn from(err: ConvergeError) -> Self {
        Self::Converge(err.to_string())
    }
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

    /// Run organism's structural admission gate on an [`IntentPacket`] and
    /// stage it through Converge's typed admission boundary.
    ///
    /// This is the public Organism → Helms contract for getting work into the
    /// runtime. Callers compile their input (e.g. with `axiom_truth::compile_intent`
    /// for Truth-shaped sources) into an [`IntentPacket`] and pass it here.
    ///
    /// Flow:
    /// 1. Organism's structural admission gate runs (cheap, deterministic).
    /// 2. The intent is staged through
    ///    [`converge_kernel::admission::admit_observation`] under
    ///    [`ContextKey::Seeds`]. The kernel produces the [`AdmissionReceipt`];
    ///    promotion to a governed fact happens later through the engine's
    ///    normal gate.
    ///
    /// Returns the [`AdmissionReceipt`] — proof the intent has been staged.
    /// The caller already holds the `IntentPacket` and can use it directly to
    /// drive resolution and planning.
    ///
    /// # Errors
    ///
    /// Returns [`IntentAdmissionError`] if the intent fails the admission
    /// gate, or fails Converge admission validation.
    pub fn admit_intent(
        &self,
        intent: &IntentPacket,
        actor: AdmissionActor,
        source: AdmissionSource,
        context: &mut ContextState,
    ) -> Result<AdmissionReceipt, IntentAdmissionError> {
        gate_admission(intent).map_err(|err| match err {
            PipelineError::Rejected(msg) => IntentAdmissionError::Rejected(msg),
            other => IntentAdmissionError::Rejected(other.to_string()),
        })?;

        let payload = serde_json::to_string(intent)
            .map_err(|err| IntentAdmissionError::Serialize(err.to_string()))?;
        let admission_body = AdmissionContent::new(payload)?;
        let request = AdmissionRequest::new(
            actor,
            source,
            ContextKey::Seeds,
            format!("intent:{}", intent.id),
            admission_body,
        )?;
        let receipt = admit_observation(context, request)?;
        Ok(receipt)
    }

    /// Pick a formation template for `intent` from `catalog` given the host's
    /// available `capabilities`. The guru classifies the intent, queries the
    /// catalog by class-derived keywords, and post-filters by the host's
    /// declared capability inventory. Returns the chosen primary plus up to
    /// two alternates and a [`SelectionTrace`] explaining the choice.
    ///
    /// This is auto-mode's *front half* — selection without execution. To run
    /// the chosen template, build a [`FormationCompileRequest`] keyed on
    /// `selection.primary.id()` and call [`compile_and_run_formation`].
    ///
    /// # Errors
    ///
    /// Returns [`GuruError`] if no template in `catalog` satisfies the
    /// classified problem under `capabilities`.
    pub fn select_formation<'cat>(
        &self,
        intent: &IntentPacket,
        catalog: &'cat FormationCatalog,
        capabilities: &[SuggestorCapability],
    ) -> Result<GuruSelection<'cat>, GuruError> {
        FormationGuru::new(catalog).select(intent, capabilities)
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
        gate_admission(intent)?;
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

    // -- Catalog-aware compile path ----------------------------------------
    //
    // These methods source Suggestor candidates from a `DiscoveryCatalog`
    // (organism-catalog) via deterministic structural filters, and return
    // the structured per-role decision trace so callers can explain why
    // each specialist is present or absent. `advisory_order` is an
    // optional ranked list of descriptor IDs from an out-of-band advisor
    // (e.g. an LLM-backed `CatalogLookup`); the compiler uses it strictly
    // as a tie-breaker after deterministic scoring — never as authority.

    /// Admit an intent and compile a formation plan from a
    /// [`DiscoveryCatalog`]. Catalog-aware parallel to
    /// [`Self::compile_formation`].
    pub fn compile_formation_from_catalog(
        &self,
        intent: &IntentPacket,
        request: &FormationCompileRequest,
        formation_templates: &FormationCatalog,
        catalog: &DiscoveryCatalog,
        providers: &ProviderDescriptorCatalog,
        advisory_order: Option<&[String]>,
    ) -> Result<CatalogCompiledFormationPlan, PipelineError> {
        gate_admission(intent)?;
        Ok(FormationCompiler::new().compile_from_catalog(
            request,
            formation_templates,
            catalog,
            providers,
            advisory_order,
        )?)
    }

    /// Admit, compile from catalog, and instantiate a runnable formation.
    /// Catalog-aware parallel to [`Self::compile_and_instantiate_formation`].
    #[allow(clippy::too_many_arguments)]
    pub fn compile_and_instantiate_from_catalog(
        &self,
        intent: &IntentPacket,
        request: &FormationCompileRequest,
        formation_templates: &FormationCatalog,
        catalog: &DiscoveryCatalog,
        providers: &ProviderDescriptorCatalog,
        executables: &ExecutableSuggestorCatalog,
        seeds: impl IntoIterator<Item = Seed>,
        advisory_order: Option<&[String]>,
    ) -> Result<(CatalogCompiledFormationPlan, Formation), PipelineError> {
        let outcome = self.compile_formation_from_catalog(
            intent,
            request,
            formation_templates,
            catalog,
            providers,
            advisory_order,
        )?;
        let formation = executables.instantiate(&outcome.plan, seeds)?;
        Ok((outcome, formation))
    }

    /// Source `k` candidate rosters from the catalog, instantiate each,
    /// and run a [`FormationTournament`] to pick the winner.
    ///
    /// Each candidate covers the same formation template requirements
    /// but draws a different roster from the catalog (via swap-out
    /// diversity — see [`FormationCompiler::compile_k_candidates`]).
    /// The returned [`CatalogTournamentOutcome`] carries both the
    /// tournament result (winner + scores + priors) and each candidate's
    /// [`CatalogCompiledFormationPlan`] so the audit trail shows
    /// selection rationale AND score outcome side-by-side.
    ///
    /// `seeds_fn` is called once per candidate to produce its seed
    /// inventory — formations consume their seeds when run, so each
    /// candidate needs its own fresh `Vec<Seed>`.
    #[allow(clippy::too_many_arguments)]
    pub async fn compile_k_and_run_tournament<F>(
        &self,
        intent: &IntentPacket,
        request: &FormationCompileRequest,
        formation_templates: &FormationCatalog,
        catalog: &DiscoveryCatalog,
        providers: &ProviderDescriptorCatalog,
        executables: &ExecutableSuggestorCatalog,
        seeds_fn: F,
        k: usize,
    ) -> Result<CatalogTournamentOutcome, PipelineError>
    where
        F: Fn(usize, &CatalogCompiledFormationPlan) -> Vec<Seed>,
    {
        gate_admission(intent)?;

        let candidates = FormationCompiler::new().compile_k_candidates(
            request,
            formation_templates,
            catalog,
            providers,
            k,
        )?;

        if candidates.is_empty() {
            return Err(PipelineError::Tournament(
                "compile_k_candidates returned no candidates".to_string(),
            ));
        }

        let mut formations: Vec<Formation> = Vec::with_capacity(candidates.len());
        for (index, candidate) in candidates.iter().enumerate() {
            let seeds = seeds_fn(index, candidate);
            // Unique label per candidate so the tournament's
            // FormationScore.label is the join key back to the
            // originating candidate. Format: "{template_id}#{index}".
            let label = format!("{}#{index}", candidate.plan.template_id);
            let formation = executables.instantiate_with_label(&candidate.plan, seeds, label)?;
            formations.push(formation);
        }

        let tournament = FormationTournament::new(intent.id, request.plan_id, formations);
        let tournament_result = tournament
            .run()
            .await
            .map_err(|err| PipelineError::Tournament(err.to_string()))?;

        // Pair each FormationScore back to its candidate by parsing the
        // index suffix from the label.
        let mut scored_candidates: Vec<ScoredCatalogCandidate> =
            Vec::with_capacity(candidates.len());
        for score in &tournament_result.all_scores {
            let index = candidate_index_from_label(&score.label).ok_or_else(|| {
                PipelineError::Tournament(format!(
                    "tournament returned an unjoinable score label: {label}",
                    label = score.label
                ))
            })?;
            let candidate = candidates.get(index).cloned().ok_or_else(|| {
                PipelineError::Tournament(format!(
                    "score index {index} out of range (k = {})",
                    candidates.len()
                ))
            })?;
            scored_candidates.push(ScoredCatalogCandidate {
                index,
                candidate,
                score: score.clone(),
            });
        }
        // Sort by index so the order matches the original
        // compile_k_candidates output for predictable consumption.
        scored_candidates.sort_by_key(|sc| sc.index);

        let winner_index =
            candidate_index_from_label(&tournament_result.winner.label).ok_or_else(|| {
                PipelineError::Tournament(format!(
                    "tournament winner has unjoinable label: {label}",
                    label = tournament_result.winner.label
                ))
            })?;

        Ok(CatalogTournamentOutcome {
            winner_index,
            scored_candidates,
            priors: tournament_result.priors,
        })
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
        gate_admission(&intent)?;

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

/// Extract the candidate index from a label of the form
/// `{template_id}#{index}` as produced by
/// [`Runtime::compile_k_and_run_tournament`]. Returns `None` if the
/// label has no `#` or the suffix is not a valid `usize`.
fn candidate_index_from_label(label: &str) -> Option<usize> {
    label.rsplit_once('#')?.1.parse::<usize>().ok()
}

fn gate_admission(intent: &IntentPacket) -> Result<(), PipelineError> {
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
    use converge_kernel::{AgentEffect, Context, ContextKey, Suggestor};
    use converge_pack::{ProvenanceSource, TextPayload};
    use converge_provider::{BackendRequirements, CostClass, LatencyClass};

    fn id(n: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(n)
    }

    fn valid_intent() -> IntentPacket {
        IntentPacket::new("select the best AI vendor", Utc::now() + Duration::hours(1))
    }

    /// HIGH #1 regression. Verify [`CatalogTournamentOutcome::winner`]
    /// looks up by `sc.index`, not by array position. When a
    /// non-winning candidate fails at runtime, `FormationTournament`
    /// drops it from `scored_candidates` — but `winner_index` stays
    /// as the original candidate index. Indexing
    /// `scored_candidates[winner_index]` would panic; finding by
    /// `sc.index == winner_index` is safe.
    #[test]
    fn catalog_tournament_winner_lookup_safe_when_lower_index_dropped() {
        // Construct an outcome that mimics: candidate 0 was dropped,
        // candidate 1 won. scored_candidates has length 1 with
        // index = 1; winner_index = 1.
        let plan = CompiledFormationPlan {
            plan_id: id(0xCAFE),
            correlation_id: id(0xBEEF),
            tenant_id: None,
            template_id: "winner-template".to_string(),
            template_kind: converge_kernel::formation::FormationKind::Static,
            roster: Vec::new(),
            provider_assignments: Vec::new(),
            trace: Vec::new(),
        };
        let candidate = CatalogCompiledFormationPlan {
            plan,
            decisions: Vec::new(),
        };
        let score = FormationScore {
            label: "winner-template#1".to_string(),
            score: 0.9,
            converged: true,
            cycles: 1,
            criteria_met: 0,
            criteria_total: 0,
        };
        let outcome = CatalogTournamentOutcome {
            winner_index: 1,
            scored_candidates: vec![ScoredCatalogCandidate {
                index: 1,
                candidate,
                score,
            }],
            priors: Vec::new(),
        };

        // Before the fix: scored_candidates[winner_index] would index
        // position 1 in a length-1 Vec → out-of-bounds panic.
        let winner = outcome.winner();
        assert_eq!(winner.index, 1);
        assert!((winner.score.score - 0.9).abs() < f64::EPSILON);
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

        fn provenance(&self) -> &'static str {
            crate::provenance::ORGANISM_RUNTIME_PROVENANCE.as_str()
        }

        fn accepts(&self, ctx: &dyn Context) -> bool {
            self.dependencies.iter().any(|key| ctx.has(*key)) && !ctx.has(self.output)
        }

        async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
            AgentEffect::with_proposal(
                crate::provenance::ORGANISM_RUNTIME_PROVENANCE.proposed_fact(
                    self.output,
                    format!("{}-output", self.name),
                    TextPayload::new(format!("{} produced compiled-role output", self.name)),
                ),
            )
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
    fn runtime_selects_decision_template_for_decision_intent() {
        let catalog = standard_formation_catalog();
        let caps = [
            SuggestorCapability::LlmReasoning,
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::Analytics,
        ];
        let intent = IntentPacket::new(
            "decide which vendor to approve",
            Utc::now() + Duration::hours(1),
        );

        let selection = Runtime::new()
            .select_formation(&intent, &catalog, &caps)
            .expect("decision intent matches the standard catalog");

        assert_eq!(selection.primary.id(), "organism-decision");
        assert_eq!(
            selection.classification.class,
            organism_intent::problem::ProblemClass::Decision
        );
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
