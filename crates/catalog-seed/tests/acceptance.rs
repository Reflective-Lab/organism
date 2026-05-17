//! Acceptance tests for the seed catalog.
//!
//! Validates: (a) basic shape — non-empty discovery metadata on every
//! descriptor; (b) the seed feeds the catalog-aware compile path
//! end-to-end on realistic formation queries; (c) failure cases produce
//! the structured trace; (d) per-source version manifests match what is
//! actually pinned in the workspace.

use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
    StaticFormationTemplate, SuggestorCapability, SuggestorRole,
};
use organism_catalog::ProviderDescriptorCatalog;
use organism_catalog_seed as seed;
use organism_runtime::{FormationCompileError, FormationCompileRequest, FormationCompiler};
use uuid::Uuid;

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn template_catalog(
    id: &str,
    keyword: &str,
    roles: Vec<SuggestorRole>,
    capabilities: Vec<SuggestorCapability>,
) -> FormationCatalog {
    let mut metadata =
        FormationTemplateMetadata::new(id, format!("Acceptance template: {id}"), roles)
            .with_keyword(keyword);
    for cap in capabilities {
        metadata = metadata.with_required_capability(cap);
    }
    FormationCatalog::new().with_template(FormationTemplate::static_template(
        StaticFormationTemplate::new(metadata),
    ))
}

fn request(keyword: &str) -> FormationCompileRequest {
    FormationCompileRequest::new(
        id(0xACCE),
        id(0xB001),
        FormationTemplateQuery::new().with_keyword(keyword),
    )
}

// ---------------------------------------------------------------------------
// (a) basic shape — every descriptor has populated discovery metadata
// ---------------------------------------------------------------------------

#[test]
fn every_descriptor_has_non_empty_discovery_metadata() {
    let catalog = seed::all();
    assert!(!catalog.is_empty(), "seed catalog must not be empty");

    for entry in &catalog {
        let d = &entry.discovery;
        let id = entry.id();
        assert!(!d.summary.is_empty(), "{id}: empty summary");
        assert!(!d.use_when.is_empty(), "{id}: empty use_when");
        assert!(!d.examples.is_empty(), "{id}: no examples");
        assert!(
            !d.loop_contributions.is_empty(),
            "{id}: no loop_contributions"
        );
        assert!(!d.produces.is_empty(), "{id}: no produces fact families");
    }
}

#[test]
fn descriptor_ids_are_unique_across_all_trees() {
    let catalog = seed::all();
    let mut ids: Vec<&str> = catalog
        .iter()
        .map(organism_catalog::CatalogSuggestorDescriptor::id)
        .collect();
    let before = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        before,
        ids.len(),
        "descriptor ids must be unique across converge + organism + mosaic trees"
    );
}

#[test]
fn per_source_catalogs_partition_the_full_seed() {
    let total = seed::all().len();
    let parts =
        seed::converge_only().len() + seed::organism_only().len() + seed::mosaic_only().len();
    assert_eq!(total, parts, "per-source counts must sum to all()");
}

// ---------------------------------------------------------------------------
// (b) realistic formation compiles against mosaic_only()
// ---------------------------------------------------------------------------

#[test]
fn mosaic_only_satisfies_vendor_due_diligence_formation() {
    // A realistic 4-role due-diligence formation that mosaic alone can
    // satisfy: external lookup (Signal), policy gate (Constraint),
    // analytics scoring (Evaluation), optimization (Planning).
    let templates = template_catalog(
        "vendor-due-diligence",
        "vendor-due-diligence",
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
    );
    let catalog = seed::mosaic_only();
    let providers = ProviderDescriptorCatalog::new();
    let request = request("vendor-due-diligence");

    let outcome = FormationCompiler::new()
        .compile_from_catalog(&request, &templates, &catalog, &providers, None)
        .expect("mosaic_only should satisfy vendor-due-diligence");

    assert_eq!(outcome.plan.roster.len(), 4);

    // Every chosen descriptor must come from the mosaic tree (id prefix
    // is one of the family conventions).
    for role in &outcome.plan.roster {
        let id = &role.suggestor_id;
        let known = id.starts_with("arbiter-")
            || id.starts_with("embassy-")
            || id.starts_with("ferrox-")
            || id.starts_with("mnemos-")
            || id.starts_with("prism-")
            || id.starts_with("soter-");
        assert!(known, "roster contained non-mosaic id: {id}");
    }

    // All four roles must be covered by the chosen descriptors.
    let chosen_roles: Vec<_> = outcome
        .decisions
        .iter()
        .filter_map(|d| d.chosen_role)
        .collect();
    for required in [
        SuggestorRole::Signal,
        SuggestorRole::Constraint,
        SuggestorRole::Evaluation,
        SuggestorRole::Planning,
    ] {
        assert!(
            chosen_roles.contains(&required),
            "role {required:?} not covered; chosen roles: {chosen_roles:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// (b) realistic formation compiles against all() that requires Synthesis
// ---------------------------------------------------------------------------

#[test]
fn all_satisfies_formation_requiring_synthesis_from_organism_tree() {
    // mosaic has no Synthesis-role descriptors; organism has
    // organism-synthesis. all() must satisfy a Synthesis-requiring
    // template by pulling from the organism tree.
    let templates = template_catalog(
        "decision-synthesis",
        "decision-synthesis",
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Constraint,
            SuggestorRole::Evaluation,
            SuggestorRole::Synthesis,
        ],
        vec![
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::Analytics,
            SuggestorCapability::LlmReasoning,
        ],
    );
    let catalog = seed::all();
    let providers = ProviderDescriptorCatalog::new();
    let request = request("decision-synthesis");

    let outcome = FormationCompiler::new()
        .compile_from_catalog(&request, &templates, &catalog, &providers, None)
        .expect("all() should satisfy a Synthesis-requiring formation");

    let chosen_roles: Vec<_> = outcome
        .decisions
        .iter()
        .filter_map(|d| d.chosen_role)
        .collect();
    assert!(
        chosen_roles.contains(&SuggestorRole::Synthesis),
        "Synthesis role must be covered: {chosen_roles:?}"
    );
}

// ---------------------------------------------------------------------------
// (c) failure case — mosaic_only cannot cover Synthesis
// ---------------------------------------------------------------------------

#[test]
fn mosaic_only_fails_when_synthesis_required_with_partial_trace() {
    let templates = template_catalog(
        "needs-synthesis",
        "needs-synthesis",
        vec![SuggestorRole::Synthesis],
        vec![SuggestorCapability::LlmReasoning],
    );
    let catalog = seed::mosaic_only();
    let providers = ProviderDescriptorCatalog::new();
    let request = request("needs-synthesis");

    let failure = FormationCompiler::new()
        .compile_from_catalog(&request, &templates, &catalog, &providers, None)
        .expect_err("mosaic alone cannot cover Synthesis");

    match &failure.error {
        FormationCompileError::UncoveredRequirements {
            unmatched_roles,
            unmatched_capabilities,
        } => {
            assert!(unmatched_roles.contains(&SuggestorRole::Synthesis));
            assert!(unmatched_capabilities.contains(&SuggestorCapability::LlmReasoning));
        }
        other => panic!("expected UncoveredRequirements, got {other:?}"),
    }

    // Partial trace must show the failing iteration.
    let final_decision = failure
        .decisions
        .last()
        .expect("partial trace must exist even on failure");
    assert!(final_decision.chosen.is_none());
    assert!(
        final_decision
            .unmatched_roles_at_start
            .contains(&SuggestorRole::Synthesis)
    );
}

// ---------------------------------------------------------------------------
// (d) per-source pinned_to manifests
//
// NOTE on scope: these tests assert that the seed crate's *self-reported*
// pin manifest matches the values we expect. They do NOT cross-check
// against the actual workspace Cargo.toml or crates.io. A real drift
// check would parse the workspace manifest at test time and assert each
// pinned_to entry resolves to the same version that the workspace
// resolver picks. That belongs in a separate audit step (e.g. a
// pre-release check or a CI job that runs `cargo metadata --format-version 1`
// and joins on crate name). For now the assertions below catch
// hand-edited drift in the seed itself, which is the most common
// regression.
// ---------------------------------------------------------------------------

#[test]
fn converge_pinned_to_matches_published_versions() {
    let pins: Vec<_> = seed::converge::pinned_to().to_vec();
    assert!(pins.contains(&("converge-kernel", "3.9.1")));
    assert!(pins.contains(&("converge-optimization", "3.9.1")));
    assert!(pins.contains(&("converge-pack", "3.9.1")));
    assert!(pins.contains(&("converge-provider", "3.9.1")));
}

#[test]
fn organism_pinned_to_matches_workspace() {
    let pins: Vec<_> = seed::organism::pinned_to().to_vec();
    assert!(pins.contains(&("organism-adversarial", "1.9.0")));
    assert!(pins.contains(&("organism-learning", "1.9.0")));
    assert!(pins.contains(&("organism-planning", "1.9.0")));
    assert!(pins.contains(&("organism-runtime", "1.9.0")));
    assert!(pins.contains(&("organism-simulation", "1.9.0")));
}

#[test]
fn mosaic_pinned_to_matches_published_versions() {
    let pins: Vec<_> = seed::mosaic::pinned_to().to_vec();
    assert!(pins.contains(&("converge-arbiter-policy", "2.0.1")));
    assert!(pins.contains(&("converge-embassy-pack", "1.3.0")));
    assert!(pins.contains(&("converge-embassy-linkedin", "1.3.0")));
    assert!(pins.contains(&("converge-ferrox-solver", "0.7.1")));
    assert!(pins.contains(&("converge-manifold-adapters", "1.1.1")));
    assert!(pins.contains(&("converge-mnemos-knowledge", "1.2.2")));
    assert!(pins.contains(&("converge-prism-analytics", "2.0.0")));
    assert!(pins.contains(&("converge-soter-smt", "0.2.2")));
}

#[test]
fn crate_root_pinned_to_is_union_of_trees() {
    let total = seed::pinned_to().len();
    let parts = seed::converge::pinned_to().len()
        + seed::organism::pinned_to().len()
        + seed::mosaic::pinned_to().len();
    assert_eq!(total, parts);
}
