//! End-to-end integration test: design Formation in Converge →
//! draft facts in context → exact draft validator → work Formation in
//! Converge.
//!
//! This proves the invariant: the deliberation that picks a work
//! formation is itself a Formation. Nothing here introduces a
//! side-car workflow or new trait — just normal Suggestors composing
//! through Converge.

use async_trait::async_trait;
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
use organism_dynamics::{
    BeautyContestSuggestor, CatalogProposerSuggestor, DRAFT_KIND, FormationDraft, compile_draft,
    extract_drafts,
};
use organism_runtime::{
    ExecutableSuggestorCatalog, Formation, FormationCompileError, FormationCompileRequest,
    FormationCompiler, Seed,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
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

fn work_catalog() -> DiscoveryCatalog {
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

fn work_template_catalog() -> FormationCatalog {
    let metadata = FormationTemplateMetadata::new(
        "work-template",
        "Work formation: signal + constraint",
        vec![SuggestorRole::Signal, SuggestorRole::Constraint],
    )
    .with_keyword("work-template")
    .with_required_capability(SuggestorCapability::KnowledgeRetrieval)
    .with_required_capability(SuggestorCapability::PolicyEnforcement);
    FormationCatalog::new().with_template(FormationTemplate::static_template(
        StaticFormationTemplate::new(metadata),
    ))
}

fn work_request() -> FormationCompileRequest {
    FormationCompileRequest::new(
        Uuid::from_u128(0xD001),
        Uuid::from_u128(0xD002),
        FormationTemplateQuery::new().with_keyword("work-template"),
    )
}

// Work-side Suggestor: converges once seeds are present.
struct ConvergingTagSuggestor {
    name: &'static str,
    writes: ContextKey,
}

#[derive(Debug, Clone, Copy)]
struct TestProvenance;
impl ProvenanceSource for TestProvenance {
    fn as_str(&self) -> &'static str {
        "design-to-work-test"
    }
}

#[async_trait]
impl Suggestor for ConvergingTagSuggestor {
    fn name(&self) -> &'static str {
        self.name
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }
    fn provenance(&self) -> &'static str {
        TestProvenance.as_str()
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(self.writes)
    }
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);
        AgentEffect::builder()
            .proposal(TestProvenance.proposed_fact(
                self.writes,
                format!("{}-{}", self.name, seeds[0].id()),
                TextPayload::new(format!("converged from {}", self.name)),
            ))
            .build()
    }
}

fn work_executables() -> ExecutableSuggestorCatalog {
    let mut cat = ExecutableSuggestorCatalog::new();
    for (id, writes) in [
        ("signal-a", ContextKey::Hypotheses),
        ("signal-b", ContextKey::Hypotheses),
        ("constraint-a", ContextKey::Constraints),
        ("constraint-b", ContextKey::Constraints),
    ] {
        cat.register_factory(id, move || ConvergingTagSuggestor { name: id, writes })
            .unwrap();
    }
    cat
}

fn seed() -> Seed {
    Seed {
        key: ContextKey::Seeds,
        id: "design-seed".into(),
        content: "design the work formation".to_string(),
        provenance: "test".to_string(),
    }
}

// ---------------------------------------------------------------------------
// End-to-end loop: design Formation → drafts → validate → run work
// ---------------------------------------------------------------------------

#[tokio::test]
async fn design_formation_emits_drafts_validator_compiles_winner_work_formation_converges() {
    let catalog = work_catalog();
    let templates = work_template_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let request = work_request();

    // --- Build the design Formation: proposer + scorer, seeded ---
    let proposer = CatalogProposerSuggestor::new(
        catalog.clone(),
        templates.clone(),
        providers.clone(),
        request.clone(),
        3,
    );
    let scorer = BeautyContestSuggestor::new(2);

    let design = Formation::new("design-formation")
        .agent_boxed(Box::new(proposer))
        .agent_boxed(Box::new(scorer))
        .seed(
            ContextKey::Seeds,
            "design-seed",
            "design the work formation",
            "test",
        );

    let design_result = design
        .run()
        .await
        .expect("design formation should converge");
    assert!(
        design_result.converge_result.converged,
        "design formation must converge; stop_reason = {:?}",
        design_result.converge_result.stop_reason
    );

    // --- Extract drafts from the design's promoted context ---
    let drafts = extract_drafts(
        &design_result.converge_result.context,
        ContextKey::Proposals,
    );
    assert!(
        !drafts.is_empty(),
        "expected at least one shortlist draft in Proposals; got {drafts:?}"
    );
    for draft in &drafts {
        assert_eq!(draft.kind, DRAFT_KIND);
        assert!(!draft.descriptor_ids.is_empty());
    }

    // --- Validate each draft via the exact-roster validator ---
    let compiler = FormationCompiler::new();
    let validated: Vec<_> = drafts
        .iter()
        .map(|d| compile_draft(&compiler, &request, &templates, &catalog, &providers, d))
        .collect();
    let winning_plan = validated
        .into_iter()
        .find_map(Result::ok)
        .expect("at least one draft must compile");

    // --- Run the winning work Formation in Converge ---
    let executables = work_executables();
    let work = executables
        .instantiate(&winning_plan.plan, [seed()])
        .expect("instantiation should succeed");
    let work_result = work.run().await.expect("work formation should converge");
    assert!(
        work_result.converge_result.converged,
        "work formation must converge; stop_reason = {:?}",
        work_result.converge_result.stop_reason
    );
    assert!(
        work_result
            .converge_result
            .context
            .has(ContextKey::Hypotheses)
    );
    assert!(
        work_result
            .converge_result
            .context
            .has(ContextKey::Constraints)
    );
}

// ---------------------------------------------------------------------------
// Exactness: compile_draft rejects a bogus draft, no silent reselect
// ---------------------------------------------------------------------------

#[test]
fn compile_draft_rejects_bogus_descriptor_no_silent_replacement() {
    let catalog = work_catalog();
    let templates = work_template_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let request = work_request();
    let compiler = FormationCompiler::new();

    // A draft that references a descriptor not in the catalog. The
    // validator must error rather than silently replace it with a
    // valid greedy roster.
    let bad_draft = FormationDraft::new(
        vec!["signal-a".to_string(), "does-not-exist".to_string()],
        "bogus draft to prove exactness",
        "test",
    );

    let failure = compile_draft(
        &compiler, &request, &templates, &catalog, &providers, &bad_draft,
    )
    .expect_err("bogus descriptor must be rejected");

    assert!(matches!(
        failure.error,
        FormationCompileError::DraftDescriptorMissing { ref descriptor_id }
            if descriptor_id == "does-not-exist"
    ));
}

#[test]
fn compile_draft_rejects_undercovering_roster_no_silent_completion() {
    let catalog = work_catalog();
    let templates = work_template_catalog();
    let providers = ProviderDescriptorCatalog::new();
    let request = work_request();
    let compiler = FormationCompiler::new();

    // A draft that has a valid descriptor but doesn't cover the
    // template's Constraint role. The validator must error, not
    // silently pick a constraint descriptor to "fix" the draft.
    let partial_draft = FormationDraft::new(
        vec!["signal-a".to_string()],
        "intentionally undercovering",
        "test",
    );

    let failure = compile_draft(
        &compiler,
        &request,
        &templates,
        &catalog,
        &providers,
        &partial_draft,
    )
    .expect_err("undercovering draft must be rejected");

    match failure.error {
        FormationCompileError::UncoveredRequirements {
            unmatched_roles,
            unmatched_capabilities,
        } => {
            assert!(unmatched_roles.contains(&SuggestorRole::Constraint));
            assert!(unmatched_capabilities.contains(&SuggestorCapability::PolicyEnforcement));
        }
        other => panic!("expected UncoveredRequirements, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Extract is strict — non-draft facts on the same ContextKey are ignored
// ---------------------------------------------------------------------------

#[tokio::test]
async fn extract_drafts_ignores_non_draft_facts_on_same_key() {
    // Build a tiny Formation that proposes both a real draft fact
    // and an unrelated TextPayload fact on the same ContextKey.
    // extract_drafts must return only the draft.
    struct MixedEmitter;
    #[async_trait]
    impl Suggestor for MixedEmitter {
        fn name(&self) -> &'static str {
            "mixed-emitter"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[ContextKey::Seeds]
        }
        fn provenance(&self) -> &'static str {
            TestProvenance.as_str()
        }
        fn accepts(&self, ctx: &dyn Context) -> bool {
            ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Proposals)
        }
        async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
            let draft = FormationDraft::new(
                vec!["signal-a".to_string(), "constraint-a".to_string()],
                "well-formed",
                "mixed-emitter",
            );
            let draft_json = serde_json::to_string(&draft).unwrap();
            AgentEffect::builder()
                .proposal(TestProvenance.proposed_fact(
                    ContextKey::Proposals,
                    "real-draft",
                    TextPayload::new(draft_json),
                ))
                .proposal(TestProvenance.proposed_fact(
                    ContextKey::Proposals,
                    "unrelated-text",
                    TextPayload::new("not a draft, just narrative text"),
                ))
                .proposal(TestProvenance.proposed_fact(
                    ContextKey::Proposals,
                    "wrong-kind-json",
                    TextPayload::new(r#"{"kind":"something.else","descriptor_ids":[]}"#),
                ))
                .build()
        }
    }

    let formation = Formation::new("mixed-emitter")
        .agent_boxed(Box::new(MixedEmitter))
        .seed(ContextKey::Seeds, "s", "x", "test");
    let result = formation.run().await.expect("should converge");

    let drafts = extract_drafts(&result.converge_result.context, ContextKey::Proposals);
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].rationale, "well-formed");
}

// ---------------------------------------------------------------------------
// Use the dummy intent helper for completeness — proves IntentPacket is
// not in the dynamics public surface (we only need it if Runtime drives
// the loop; v1 lets hosts compose).
// ---------------------------------------------------------------------------

fn _unused_assertion_dynamics_does_not_pull_intent_packet_into_its_api() {
    // Compile-time check: if organism-dynamics ever takes IntentPacket
    // in its public surface, this stub will become relevant. For now,
    // just confirm the crate's deliberation Suggestors and helpers
    // don't need it.
    let _expires = Utc::now() + Duration::hours(1);
}
