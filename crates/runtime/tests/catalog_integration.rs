//! Integration tests for the catalog-aware Runtime methods and the
//! tournament-from-catalog path.
//!
//! Split into two sections:
//! - `compile_k_*` tests use `organism-catalog-seed` to prove k-best
//!   diversity against realistic descriptor metadata. No executable
//!   factories needed — the tests inspect compile output only.
//! - `tournament_*` tests use a small inline synthetic catalog wired to
//!   id-matched no-op factories, so the full
//!   `compile_k_and_run_tournament` pipeline can run end-to-end without
//!   pulling additional Suggestor-implementing crates.

use chrono::{Duration, Utc};
use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
    ProfileSnapshot, StaticFormationTemplate, SuggestorCapability, SuggestorRole,
};
use converge_kernel::{AgentEffect, Context, ContextKey};
use converge_pack::{ProvenanceSource, Suggestor, TextPayload};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{
    CatalogSuggestorDescriptor, DiscoveryCatalog, DiscoveryMetadata, LoopContribution,
    ProviderDescriptorCatalog, SuggestorDescriptor,
};
use organism_catalog_seed as seed;
use organism_intent::IntentPacket;
use organism_runtime::{
    CatalogCompileFailure, ExecutableSuggestorCatalog, FormationCompileError,
    FormationCompileRequest, FormationCompiler, PipelineError, Runtime, Seed,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn intent() -> IntentPacket {
    IntentPacket::new(
        "compile-k-and-run-tournament integration",
        Utc::now() + Duration::hours(1),
    )
}

fn expired_intent() -> IntentPacket {
    IntentPacket::new("expired intent", Utc::now() - Duration::hours(1))
}

fn request(plan_id: u128, keyword: &str) -> FormationCompileRequest {
    FormationCompileRequest::new(
        Uuid::from_u128(plan_id),
        Uuid::from_u128(plan_id + 1),
        FormationTemplateQuery::new().with_keyword(keyword),
    )
}

fn template_catalog(
    id: &str,
    keyword: &str,
    roles: Vec<SuggestorRole>,
    capabilities: Vec<SuggestorCapability>,
) -> FormationCatalog {
    let mut metadata = FormationTemplateMetadata::new(id, format!("Integration: {id}"), roles)
        .with_keyword(keyword);
    for cap in capabilities {
        metadata = metadata.with_required_capability(cap);
    }
    FormationCatalog::new().with_template(FormationTemplate::static_template(
        StaticFormationTemplate::new(metadata),
    ))
}

// ---------------------------------------------------------------------------
// compile_k_candidates tests — use seed, no executable factories needed
// ---------------------------------------------------------------------------

fn loop_demo_templates() -> FormationCatalog {
    template_catalog(
        "tournament-due-diligence",
        "tournament-due-diligence",
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Constraint,
            SuggestorRole::Evaluation,
            SuggestorRole::Planning,
        ],
        vec![
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::Analytics,
            SuggestorCapability::Optimization,
        ],
    )
}

#[test]
fn compile_k_candidates_produces_distinct_rosters() {
    let templates = loop_demo_templates();
    let catalog = seed::mosaic_only();
    let providers = ProviderDescriptorCatalog::new();
    let req = request(0xA00, "tournament-due-diligence");

    let candidates = FormationCompiler::new()
        .compile_k_candidates(&req, &templates, &catalog, &providers, 3)
        .expect("k=3 should compile against the mosaic seed");

    assert!(
        candidates.len() >= 2,
        "expected at least 2 distinct candidates, got {}",
        candidates.len()
    );

    // Each candidate's roster must differ from every other by at least
    // one descriptor — the contract of swap-out diversity.
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            let ids_i: Vec<&str> = candidates[i]
                .plan
                .roster
                .iter()
                .map(|r| r.suggestor_id.as_str())
                .collect();
            let ids_j: Vec<&str> = candidates[j]
                .plan
                .roster
                .iter()
                .map(|r| r.suggestor_id.as_str())
                .collect();
            assert_ne!(
                ids_i, ids_j,
                "candidates {i} and {j} produced identical rosters"
            );
        }
    }
}

#[test]
fn compile_k_zero_returns_empty_vec() {
    let templates = loop_demo_templates();
    let catalog = seed::mosaic_only();
    let providers = ProviderDescriptorCatalog::new();
    let req = request(0xA10, "tournament-due-diligence");

    let candidates = FormationCompiler::new()
        .compile_k_candidates(&req, &templates, &catalog, &providers, 0)
        .expect("k=0 is well-defined");
    assert!(candidates.is_empty());
}

#[test]
fn compile_k_one_matches_single_compile_from_catalog() {
    let templates = loop_demo_templates();
    let catalog = seed::mosaic_only();
    let providers = ProviderDescriptorCatalog::new();
    let req = request(0xA20, "tournament-due-diligence");

    let compiler = FormationCompiler::new();
    let k1 = compiler
        .compile_k_candidates(&req, &templates, &catalog, &providers, 1)
        .expect("k=1 should succeed");
    let single = compiler
        .compile_from_catalog(&req, &templates, &catalog, &providers, None)
        .expect("single compile should succeed");

    assert_eq!(k1.len(), 1);
    let k1_ids: Vec<_> = k1[0]
        .plan
        .roster
        .iter()
        .map(|r| r.suggestor_id.clone())
        .collect();
    let single_ids: Vec<_> = single
        .plan
        .roster
        .iter()
        .map(|r| r.suggestor_id.clone())
        .collect();
    assert_eq!(k1_ids, single_ids);
}

#[test]
fn compile_k_stops_gracefully_when_pool_exhausted() {
    let templates = loop_demo_templates();
    let catalog = seed::mosaic_only();
    let providers = ProviderDescriptorCatalog::new();
    let req = request(0xA30, "tournament-due-diligence");

    // k=20 is much larger than what the mosaic seed can produce as
    // distinct rosters; we should get whatever fits without error.
    let candidates = FormationCompiler::new()
        .compile_k_candidates(&req, &templates, &catalog, &providers, 20)
        .expect("graceful stop should not be an error");
    assert!(!candidates.is_empty());
    assert!(candidates.len() < 20, "should not actually produce 20");
}

#[test]
fn compile_k_returns_error_when_first_compile_fails() {
    // Empty catalog can't satisfy anything; first compile fails →
    // compile_k surfaces the underlying CatalogCompileFailure.
    let templates = loop_demo_templates();
    let empty = DiscoveryCatalog::new();
    let providers = ProviderDescriptorCatalog::new();
    let req = request(0xA40, "tournament-due-diligence");

    let err = FormationCompiler::new()
        .compile_k_candidates(&req, &templates, &empty, &providers, 3)
        .expect_err("empty catalog cannot satisfy the template");
    assert!(matches!(
        err.error,
        FormationCompileError::UncoveredRequirements { .. }
    ));
}

// ---------------------------------------------------------------------------
// HIGH #2 regression: scarce specialist must not block diversity in others
// ---------------------------------------------------------------------------

fn synthetic_descriptor(
    id: &'static str,
    role: SuggestorRole,
    capability: SuggestorCapability,
    writes: ContextKey,
) -> CatalogSuggestorDescriptor {
    let descriptor = SuggestorDescriptor::new(
        id,
        ProfileSnapshot {
            name: id.to_string(),
            role,
            output_keys: vec![writes],
            cost_hint: CostClass::Low,
            latency_hint: LatencyClass::Interactive,
            capabilities: vec![capability],
            confidence_min: 0.7,
            confidence_max: 0.95,
        },
    );
    let discovery = DiscoveryMetadata::new("Test fixture.", "Test fixture only.")
        .with_loop_contribution(LoopContribution::Synthesize);
    CatalogSuggestorDescriptor::new(descriptor, discovery)
}

#[test]
fn compile_k_keeps_scarce_specialist_across_candidates() {
    // Role A has a single valid descriptor (a1). Role B has three
    // alternatives (b1, b2, b3). The expected behavior is three
    // distinct rosters that all share a1 but rotate the b slot:
    //   {a1,b1}, {a1,b2}, {a1,b3}
    //
    // The previous "exclude every descriptor from prior rosters"
    // policy would have produced only one candidate, because a1 was
    // excluded after iteration 1 and no other A descriptor exists.
    let templates = template_catalog(
        "scarce-a",
        "scarce-a",
        vec![SuggestorRole::Signal, SuggestorRole::Constraint],
        vec![
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::PolicyEnforcement,
        ],
    );
    let catalog = DiscoveryCatalog::new()
        .with_entry(synthetic_descriptor(
            "a1",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            ContextKey::Hypotheses,
        ))
        .with_entry(synthetic_descriptor(
            "b1",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ))
        .with_entry(synthetic_descriptor(
            "b2",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ))
        .with_entry(synthetic_descriptor(
            "b3",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ));
    let providers = ProviderDescriptorCatalog::new();
    let req = request(0xA50, "scarce-a");

    let candidates = FormationCompiler::new()
        .compile_k_candidates(&req, &templates, &catalog, &providers, 3)
        .expect("scarce specialist must not block diversity");

    assert_eq!(candidates.len(), 3, "expected 3 distinct rosters");

    // Every candidate must contain a1 (it's the only valid A).
    for (i, cand) in candidates.iter().enumerate() {
        let ids: Vec<&str> = cand
            .plan
            .roster
            .iter()
            .map(|r| r.suggestor_id.as_str())
            .collect();
        assert!(
            ids.contains(&"a1"),
            "candidate {i} missing the scarce specialist a1: {ids:?}"
        );
    }

    // Each candidate must pick a different B.
    let mut chosen_bs: Vec<String> = candidates
        .iter()
        .flat_map(|cand| cand.plan.roster.iter().map(|r| r.suggestor_id.clone()))
        .filter(|id| id.starts_with('b'))
        .collect();
    chosen_bs.sort();
    assert_eq!(
        chosen_bs,
        vec!["b1".to_string(), "b2".to_string(), "b3".to_string()],
        "expected each B alternative to be used exactly once"
    );
}

// ---------------------------------------------------------------------------
// Tournament tests — small inline synthetic catalog + id-matched factories
// ---------------------------------------------------------------------------

/// Converging Suggestor parameterized by name + write key. Each
/// registered descriptor in the synthetic catalog has a matching
/// factory using this implementation.
struct ConvergingTagSuggestor {
    name: &'static str,
    writes: ContextKey,
}

#[async_trait::async_trait]
impl Suggestor for ConvergingTagSuggestor {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn provenance(&self) -> &'static str {
        "test-tournament-suggestor"
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(self.writes)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        let provenance = TestProvenance;
        AgentEffect::builder()
            .proposal(provenance.proposed_fact(
                self.writes,
                format!("{}-{}", self.name, seeds[0].id()),
                TextPayload::new(format!("converged from {}", self.name)),
            ))
            .build()
    }
}

#[derive(Debug, Clone, Copy)]
struct TestProvenance;

impl ProvenanceSource for TestProvenance {
    fn as_str(&self) -> &'static str {
        "test-tournament-suggestor"
    }
}

fn tournament_catalog() -> DiscoveryCatalog {
    DiscoveryCatalog::new()
        .with_entry(synthetic_descriptor(
            "signal-a",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            ContextKey::Hypotheses,
        ))
        .with_entry(synthetic_descriptor(
            "signal-b",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            ContextKey::Hypotheses,
        ))
        .with_entry(synthetic_descriptor(
            "constraint-a",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ))
        .with_entry(synthetic_descriptor(
            "constraint-b",
            SuggestorRole::Constraint,
            SuggestorCapability::PolicyEnforcement,
            ContextKey::Constraints,
        ))
}

fn tournament_template_catalog() -> FormationCatalog {
    template_catalog(
        "tournament-sample",
        "tournament-sample",
        vec![SuggestorRole::Signal, SuggestorRole::Constraint],
        vec![
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::PolicyEnforcement,
        ],
    )
}

fn register_tag(catalog: &mut ExecutableSuggestorCatalog, id: &'static str, writes: ContextKey) {
    catalog
        .register_factory(id, move || ConvergingTagSuggestor { name: id, writes })
        .expect("register factory");
}

fn tournament_executables() -> ExecutableSuggestorCatalog {
    let mut catalog = ExecutableSuggestorCatalog::new();
    register_tag(&mut catalog, "signal-a", ContextKey::Hypotheses);
    register_tag(&mut catalog, "signal-b", ContextKey::Hypotheses);
    register_tag(&mut catalog, "constraint-a", ContextKey::Constraints);
    register_tag(&mut catalog, "constraint-b", ContextKey::Constraints);
    catalog
}

#[tokio::test]
async fn compile_k_and_run_tournament_pairs_scores_to_candidates() {
    let runtime = Runtime::new();
    let templates = tournament_template_catalog();
    let catalog = tournament_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let executables = tournament_executables();
    let intent = intent();
    let req = request(0xB10, "tournament-sample");

    let outcome = runtime
        .compile_k_and_run_tournament(
            &intent,
            &req,
            &templates,
            &catalog,
            &providers,
            &executables,
            |_index, _plan| {
                vec![Seed {
                    key: ContextKey::Seeds,
                    id: "s1".into(),
                    content: "seed-content".to_string(),
                    provenance: "test".to_string(),
                }]
            },
            2,
        )
        .await
        .expect("tournament should run end-to-end");

    // Two distinct candidates produced and scored.
    assert_eq!(outcome.scored_candidates.len(), 2);

    // Each candidate carries its own decisions AND its own score; the
    // pair shape makes the join trivial (no label parsing required by
    // consumers).
    for sc in &outcome.scored_candidates {
        assert_eq!(
            sc.candidate.plan.roster.len(),
            2,
            "each candidate fills 2 roles"
        );
        assert!(
            !sc.candidate.decisions.is_empty(),
            "per-candidate decision trace must survive into ScoredCatalogCandidate"
        );
        // Score's label encodes the candidate index for join.
        assert!(
            sc.score.label.ends_with(&format!("#{}", sc.index)),
            "score label '{}' must end with #{} for candidate index {}",
            sc.score.label,
            sc.index,
            sc.index
        );
    }

    // The two candidates picked disjoint rosters (HIGH #2 fix).
    let cand_0_ids: Vec<_> = outcome.scored_candidates[0]
        .candidate
        .plan
        .roster
        .iter()
        .map(|r| r.suggestor_id.clone())
        .collect();
    let cand_1_ids: Vec<_> = outcome.scored_candidates[1]
        .candidate
        .plan
        .roster
        .iter()
        .map(|r| r.suggestor_id.clone())
        .collect();
    assert_ne!(cand_0_ids, cand_1_ids, "rosters should differ");

    // Winner can be looked up by index.
    let winner = outcome.winner();
    assert!(winner.score.converged);
    assert!(outcome.winner_index < outcome.scored_candidates.len());
}

#[tokio::test]
async fn compile_k_and_run_tournament_errors_when_intent_rejected() {
    let runtime = Runtime::new();
    let templates = tournament_template_catalog();
    let catalog = tournament_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let executables = tournament_executables();
    let req = request(0xB30, "tournament-sample");

    let err = runtime
        .compile_k_and_run_tournament(
            &expired_intent(),
            &req,
            &templates,
            &catalog,
            &providers,
            &executables,
            |_, _| {
                vec![Seed {
                    key: ContextKey::Seeds,
                    id: "s1".into(),
                    content: "x".to_string(),
                    provenance: "test".to_string(),
                }]
            },
            2,
        )
        .await
        .expect_err("expired intent should be rejected by admission gate");

    assert!(matches!(err, PipelineError::Rejected(_)));
}

#[test]
fn pipeline_error_carries_catalog_compile_failure_trace() {
    // Confirm the new PipelineError::CatalogCompile variant preserves
    // the partial decision trace through `?` conversion.
    let failure = CatalogCompileFailure {
        error: FormationCompileError::NoTemplate,
        decisions: Vec::new(),
    };
    let pipeline_err: PipelineError = failure.into();
    assert!(matches!(pipeline_err, PipelineError::CatalogCompile(_)));
}
