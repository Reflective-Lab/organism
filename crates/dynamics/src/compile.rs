//! Thin wrapper over the exact-roster validator on
//! [`organism_runtime::FormationCompiler`]. Lives here so callers of
//! the dynamics crate can hand a [`FormationDraft`] directly without
//! pulling out its `descriptor_ids` themselves.

use converge_kernel::formation::FormationCatalog;
use organism_catalog::{DiscoveryCatalog, ProviderDescriptorCatalog};
use organism_runtime::{
    CatalogCompileFailure, CatalogCompiledFormationPlan, FormationCompileRequest, FormationCompiler,
};

use crate::payload::FormationDraft;

/// Validate `draft` against the catalog and the matched template.
/// Returns the validated plan with the draft's exact roster — no
/// greedy reselection. See
/// [`organism_runtime::FormationCompiler::compile_draft_from_catalog`]
/// for the underlying semantics and error variants.
pub fn compile_draft(
    compiler: &FormationCompiler,
    request: &FormationCompileRequest,
    formation_templates: &FormationCatalog,
    catalog: &DiscoveryCatalog,
    providers: &ProviderDescriptorCatalog,
    draft: &FormationDraft,
) -> Result<CatalogCompiledFormationPlan, CatalogCompileFailure> {
    compiler.compile_draft_from_catalog(
        request,
        formation_templates,
        catalog,
        providers,
        &draft.descriptor_ids,
    )
}
