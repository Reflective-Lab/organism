//! Formation compiler — Organism-owned selection before Converge execution.
//!
//! The compiler turns a business intent classification into an executable
//! formation plan. Converge still owns execution, promotion, gates, and audit.

use converge_kernel::ContextKey;
use converge_kernel::formation::{
    FormationCatalog, FormationKind, FormationTemplateQuery, SuggestorCapability, SuggestorRole,
};
use converge_provider::{BackendRequirements, ComplianceLevel, DataSovereignty};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use organism_catalog::{CatalogSuggestorDescriptor, DiscoveryCatalog};
pub use organism_catalog::{
    DataContract, GovernanceClass, ProviderDescriptor, ProviderDescriptorCatalog, ReplayMode,
    SuggestorDescriptor, SuggestorDescriptorCatalog,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationCompilerCatalogs {
    pub formation_templates: FormationCatalog,
    pub suggestors: SuggestorDescriptorCatalog,
    pub providers: ProviderDescriptorCatalog,
}

impl FormationCompilerCatalogs {
    pub fn new(formation_templates: FormationCatalog) -> Self {
        Self {
            formation_templates,
            suggestors: SuggestorDescriptorCatalog::new(),
            providers: ProviderDescriptorCatalog::new(),
        }
    }

    pub fn with_suggestor(mut self, descriptor: SuggestorDescriptor) -> Self {
        self.suggestors.register(descriptor);
        self
    }

    pub fn with_provider(mut self, descriptor: ProviderDescriptor) -> Self {
        self.providers.register(descriptor);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationCompileRequest {
    pub plan_id: Uuid,
    pub correlation_id: Uuid,
    pub tenant_id: Option<String>,
    pub query: FormationTemplateQuery,
    pub domain_tags: Vec<String>,
}

impl FormationCompileRequest {
    pub fn new(plan_id: Uuid, correlation_id: Uuid, query: FormationTemplateQuery) -> Self {
        Self {
            plan_id,
            correlation_id,
            tenant_id: None,
            query,
            domain_tags: Vec::new(),
        }
    }

    pub fn with_tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn with_domain_tag(mut self, tag: impl Into<String>) -> Self {
        self.domain_tags.push(tag.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledSuggestorRole {
    pub suggestor_id: String,
    pub role: SuggestorRole,
    pub capabilities: Vec<SuggestorCapability>,
    pub reads: Vec<ContextKey>,
    pub writes: Vec<ContextKey>,
    pub input_contracts: Vec<DataContract>,
    pub output_contracts: Vec<DataContract>,
    pub replay_mode: ReplayMode,
    pub governance_class: GovernanceClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleProviderAssignment {
    pub suggestor_id: String,
    pub role: SuggestorRole,
    pub provider_id: String,
    pub requirements: BackendRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledFormationPlan {
    pub plan_id: Uuid,
    pub correlation_id: Uuid,
    pub tenant_id: Option<String>,
    pub template_id: String,
    pub template_kind: FormationKind,
    pub roster: Vec<CompiledSuggestorRole>,
    pub provider_assignments: Vec<RoleProviderAssignment>,
    pub trace: Vec<String>,
}

/// Catalog-aware compile output. Wraps an unchanged
/// [`CompiledFormationPlan`] with the structured per-role decision trace
/// that the catalog-aware path produces.
///
/// This is a wrapper rather than a new field on
/// [`CompiledFormationPlan`] to keep that type's public surface stable
/// (it is a public struct with public fields, so adding a field would
/// break struct-literal consumers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogCompiledFormationPlan {
    pub plan: CompiledFormationPlan,
    pub decisions: Vec<RoleDecision>,
}

/// What was considered, chosen, or omitted when satisfying a single
/// requirement step during catalog-aware compilation.
///
/// `unmatched_roles_at_start` and `unmatched_capabilities_at_start`
/// capture the *full* outstanding requirement set at the start of the
/// iteration, not just the first item. This matters because
/// [`best_from_catalog`] picks globally — a candidate that covers a
/// later role plus several capabilities can win over one that covers
/// only the first remaining role. `chosen_role` records what was
/// actually filled, so audit consumers don't have to infer it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDecision {
    /// All roles still unmatched when this iteration began. The compiler
    /// picks a candidate that maximizes coverage globally — it may fill
    /// any of these roles, not necessarily the first.
    pub unmatched_roles_at_start: Vec<SuggestorRole>,
    /// All capabilities still needed at the start of this iteration.
    pub unmatched_capabilities_at_start: Vec<SuggestorCapability>,
    /// Every candidate the catalog surfaced for this iteration, with
    /// disposition. Includes accepted and rejected candidates.
    pub considered: Vec<CandidateConsideration>,
    /// The descriptor id selected, or None if the iteration ended in
    /// `UncoveredRequirements`.
    pub chosen: Option<String>,
    /// The role of the chosen descriptor, if any. May or may not appear
    /// in `unmatched_roles_at_start` — the greedy ranker can select a
    /// descriptor whose role was already satisfied if it still covers
    /// remaining capabilities.
    pub chosen_role: Option<SuggestorRole>,
}

/// Disposition of a single candidate descriptor during a [`RoleDecision`].
/// Discriminated so trace consumers can match instead of parsing prose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateConsideration {
    pub descriptor_id: String,
    pub disposition: CandidateDisposition,
}

/// Why a candidate was selected or rejected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CandidateDisposition {
    /// Candidate was selected to fill this role iteration.
    Selected { reason: SelectionReason },
    /// Candidate was considered and rejected.
    Rejected { reason: RejectionReason },
}

/// Structured reason a candidate was selected. Carries the relevance
/// signals so downstream consumers (UI, audit, tournament) can reason
/// about the choice without parsing prose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionReason {
    /// Number of unmatched roles + unmatched capabilities this candidate
    /// covers (the primary ranking key).
    pub coverage: usize,
    /// Number of domain-tag intersections with the compile request.
    pub domain_hits: usize,
    /// Whether the candidate appeared in an externally-supplied advisory
    /// ranking (e.g. from an LLM lookup). Advisory is a tie-breaker, never
    /// authority.
    pub advisory_hit: bool,
}

/// Structured reason a candidate was rejected.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RejectionReason {
    /// Already chosen in an earlier role iteration.
    AlreadySelected,
    /// Did not cover any remaining role or capability.
    NoCoverage,
    /// Outranked by another candidate with better coverage / domain affinity.
    Outranked {
        chosen_id: String,
        own_coverage: usize,
        own_domain_hits: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormationCompileError {
    #[error("no formation template matched the compile request")]
    NoTemplate,
    #[error("formation requirements were not covered")]
    UncoveredRequirements {
        unmatched_roles: Vec<SuggestorRole>,
        unmatched_capabilities: Vec<SuggestorCapability>,
    },
    #[error("no provider matched backend requirements for suggestor '{suggestor_id}'")]
    MissingProvider {
        suggestor_id: String,
        role: SuggestorRole,
    },
}

/// Failure outcome from [`FormationCompiler::compile_from_catalog`].
/// Carries the underlying [`FormationCompileError`] plus the partial
/// per-role decision trace built up to the point of failure, so callers
/// can explain *why* the requirement could not be satisfied.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{error}")]
pub struct CatalogCompileFailure {
    #[source]
    pub error: FormationCompileError,
    pub decisions: Vec<RoleDecision>,
}

#[derive(Debug, Default)]
pub struct FormationCompiler;

impl FormationCompiler {
    pub fn new() -> Self {
        Self
    }

    pub fn compile(
        &self,
        request: &FormationCompileRequest,
        catalogs: &FormationCompilerCatalogs,
    ) -> Result<CompiledFormationPlan, FormationCompileError> {
        let template = catalogs
            .formation_templates
            .top_match(&request.query)
            .ok_or(FormationCompileError::NoTemplate)?;
        let metadata = template.metadata();

        let mut unmatched_roles = metadata.required_roles.clone();
        let mut unmatched_capabilities = unique_capabilities(
            metadata
                .required_capabilities
                .iter()
                .chain(request.query.required_capabilities.iter())
                .copied(),
        );
        let mut selected: Vec<&SuggestorDescriptor> = Vec::new();
        let mut trace = vec![format!("selected template '{}'", metadata.id)];

        while !unmatched_roles.is_empty() || !unmatched_capabilities.is_empty() {
            let Some(next) = best_suggestor(
                (&catalogs.suggestors).into_iter(),
                &selected,
                &unmatched_roles,
                &unmatched_capabilities,
                &request.domain_tags,
            ) else {
                return Err(FormationCompileError::UncoveredRequirements {
                    unmatched_roles,
                    unmatched_capabilities,
                });
            };

            trace.push(format!(
                "selected suggestor '{}' for role {:?}",
                next.id, next.profile.role
            ));
            remove_role(&mut unmatched_roles, next.profile.role);
            remove_capabilities(&mut unmatched_capabilities, &next.profile.capabilities);
            selected.push(next);
        }

        let mut provider_assignments = Vec::new();
        for descriptor in &selected {
            let Some(requirements) = &descriptor.backend_requirements else {
                continue;
            };
            let Some(provider) =
                best_provider((&catalogs.providers).into_iter(), descriptor, requirements)
            else {
                return Err(FormationCompileError::MissingProvider {
                    suggestor_id: descriptor.id.clone(),
                    role: descriptor.profile.role,
                });
            };
            trace.push(format!(
                "assigned provider '{}' to suggestor '{}'",
                provider.id, descriptor.id
            ));
            provider_assignments.push(RoleProviderAssignment {
                suggestor_id: descriptor.id.clone(),
                role: descriptor.profile.role,
                provider_id: provider.id.clone(),
                requirements: requirements.clone(),
            });
        }

        let roster = selected
            .into_iter()
            .map(|descriptor| CompiledSuggestorRole {
                suggestor_id: descriptor.id.clone(),
                role: descriptor.profile.role,
                capabilities: descriptor.profile.capabilities.clone(),
                reads: descriptor.reads.clone(),
                writes: descriptor.profile.output_keys.clone(),
                input_contracts: descriptor.input_contracts.clone(),
                output_contracts: descriptor.output_contracts.clone(),
                replay_mode: descriptor.replay_mode,
                governance_class: descriptor.governance_class,
            })
            .collect();

        Ok(CompiledFormationPlan {
            plan_id: request.plan_id,
            correlation_id: request.correlation_id,
            tenant_id: request.tenant_id.clone(),
            template_id: metadata.id.clone(),
            template_kind: template.kind(),
            roster,
            provider_assignments,
            trace,
        })
    }

    /// Catalog-aware compile. Sources Suggestor candidates from a
    /// [`DiscoveryCatalog`] using structural filters
    /// ([`DiscoveryCatalog::find_by_role`] /
    /// [`DiscoveryCatalog::find_by_capability`]), then applies the same
    /// deterministic coverage-and-affinity ranking as [`Self::compile`].
    ///
    /// The returned [`CatalogCompiledFormationPlan`] wraps an unchanged
    /// [`CompiledFormationPlan`] with a structured per-role decision
    /// trace, so callers can see why each capability was satisfied or
    /// left uncovered.
    ///
    /// `advisory_order` is an optional ranked list of descriptor IDs from
    /// an out-of-band advisor (e.g. an LLM-backed [`CatalogLookup`]). The
    /// compiler uses it strictly as a tie-breaker after deterministic
    /// scoring — it cannot promote a candidate above one with better
    /// coverage or domain affinity. LLM output is advisory, never
    /// authority.
    #[allow(clippy::too_many_lines)]
    pub fn compile_from_catalog(
        &self,
        request: &FormationCompileRequest,
        formation_templates: &FormationCatalog,
        catalog: &DiscoveryCatalog,
        providers: &ProviderDescriptorCatalog,
        advisory_order: Option<&[String]>,
    ) -> Result<CatalogCompiledFormationPlan, CatalogCompileFailure> {
        let mut decisions: Vec<RoleDecision> = Vec::new();

        let template = formation_templates
            .top_match(&request.query)
            .ok_or_else(|| CatalogCompileFailure {
                error: FormationCompileError::NoTemplate,
                decisions: decisions.clone(),
            })?;
        let metadata = template.metadata();

        let mut unmatched_roles = metadata.required_roles.clone();
        let mut unmatched_capabilities = unique_capabilities(
            metadata
                .required_capabilities
                .iter()
                .chain(request.query.required_capabilities.iter())
                .copied(),
        );
        let mut selected: Vec<&CatalogSuggestorDescriptor> = Vec::new();
        let mut trace = vec![format!("selected template '{}'", metadata.id)];

        while !unmatched_roles.is_empty() || !unmatched_capabilities.is_empty() {
            let unmatched_roles_at_start = unmatched_roles.clone();
            let unmatched_capabilities_at_start = unmatched_capabilities.clone();

            let (chosen, considered) = best_from_catalog(
                catalog,
                &selected,
                &unmatched_roles,
                &unmatched_capabilities,
                &request.domain_tags,
                advisory_order,
            );

            let Some(next) = chosen else {
                decisions.push(RoleDecision {
                    unmatched_roles_at_start,
                    unmatched_capabilities_at_start,
                    considered,
                    chosen: None,
                    chosen_role: None,
                });
                return Err(CatalogCompileFailure {
                    error: FormationCompileError::UncoveredRequirements {
                        unmatched_roles,
                        unmatched_capabilities,
                    },
                    decisions,
                });
            };

            trace.push(format!(
                "selected suggestor '{}' for role {:?}",
                next.descriptor.id, next.descriptor.profile.role
            ));
            let chosen_id = next.descriptor.id.clone();
            let chosen_role = next.descriptor.profile.role;
            decisions.push(RoleDecision {
                unmatched_roles_at_start,
                unmatched_capabilities_at_start,
                considered,
                chosen: Some(chosen_id),
                chosen_role: Some(chosen_role),
            });
            remove_role(&mut unmatched_roles, next.descriptor.profile.role);
            remove_capabilities(
                &mut unmatched_capabilities,
                &next.descriptor.profile.capabilities,
            );
            selected.push(next);
        }

        let mut provider_assignments = Vec::new();
        for entry in &selected {
            let descriptor = &entry.descriptor;
            let Some(requirements) = &descriptor.backend_requirements else {
                continue;
            };
            let Some(provider) = best_provider(providers.into_iter(), descriptor, requirements)
            else {
                return Err(CatalogCompileFailure {
                    error: FormationCompileError::MissingProvider {
                        suggestor_id: descriptor.id.clone(),
                        role: descriptor.profile.role,
                    },
                    decisions,
                });
            };
            trace.push(format!(
                "assigned provider '{}' to suggestor '{}'",
                provider.id, descriptor.id
            ));
            provider_assignments.push(RoleProviderAssignment {
                suggestor_id: descriptor.id.clone(),
                role: descriptor.profile.role,
                provider_id: provider.id.clone(),
                requirements: requirements.clone(),
            });
        }

        let roster = selected
            .into_iter()
            .map(|entry| {
                let descriptor = &entry.descriptor;
                CompiledSuggestorRole {
                    suggestor_id: descriptor.id.clone(),
                    role: descriptor.profile.role,
                    capabilities: descriptor.profile.capabilities.clone(),
                    reads: descriptor.reads.clone(),
                    writes: descriptor.profile.output_keys.clone(),
                    input_contracts: descriptor.input_contracts.clone(),
                    output_contracts: descriptor.output_contracts.clone(),
                    replay_mode: descriptor.replay_mode,
                    governance_class: descriptor.governance_class,
                }
            })
            .collect();

        let plan = CompiledFormationPlan {
            plan_id: request.plan_id,
            correlation_id: request.correlation_id,
            tenant_id: request.tenant_id.clone(),
            template_id: metadata.id.clone(),
            template_kind: template.kind(),
            roster,
            provider_assignments,
            trace,
        };

        Ok(CatalogCompiledFormationPlan { plan, decisions })
    }
}

fn best_suggestor<'a>(
    candidates: impl Iterator<Item = &'a SuggestorDescriptor>,
    selected: &[&SuggestorDescriptor],
    unmatched_roles: &[SuggestorRole],
    unmatched_capabilities: &[SuggestorCapability],
    domain_tags: &[String],
) -> Option<&'a SuggestorDescriptor> {
    candidates
        .filter(|candidate| !selected.iter().any(|chosen| chosen.id == candidate.id))
        .map(|candidate| {
            let coverage = suggestor_coverage(candidate, unmatched_roles, unmatched_capabilities);
            let domain_hits = domain_overlap(&candidate.domain_tags, domain_tags);
            (candidate, coverage, domain_hits)
        })
        .filter(|(_, coverage, _)| *coverage > 0)
        .max_by(
            |(left, left_coverage, left_domain), (right, right_coverage, right_domain)| {
                left_coverage
                    .cmp(right_coverage)
                    .then_with(|| left_domain.cmp(right_domain))
                    .then_with(|| right.profile.cost_hint.cmp(&left.profile.cost_hint))
                    .then_with(|| right.profile.latency_hint.cmp(&left.profile.latency_hint))
                    .then_with(|| right.id.cmp(&left.id))
            },
        )
        .map(|(candidate, _, _)| candidate)
}

fn suggestor_coverage(
    candidate: &SuggestorDescriptor,
    unmatched_roles: &[SuggestorRole],
    unmatched_capabilities: &[SuggestorCapability],
) -> usize {
    let role_score = usize::from(unmatched_roles.contains(&candidate.profile.role));
    let capability_score = unmatched_capabilities
        .iter()
        .filter(|capability| candidate.profile.capabilities.contains(capability))
        .count();
    role_score + capability_score
}

impl FormationCompiler {
    /// Source up to `k` distinct candidate rosters from the same
    /// [`DiscoveryCatalog`] by iterative swap-out diversity.
    ///
    /// After each candidate is compiled, the function tests whether
    /// excluding each chosen descriptor would still leave the catalog
    /// capable of satisfying the template — by running a trial
    /// [`Self::compile_from_catalog`] against the catalog minus the
    /// descriptor and the current exclude set. A descriptor is added
    /// to the exclude set for the next iteration only when the trial
    /// compile succeeds, i.e. the remaining catalog can still produce
    /// *some* valid roster without that descriptor (possibly via a
    /// compositional alternative — multiple other descriptors covering
    /// the slot collectively). This is stricter and more correct than
    /// the previous heuristic "shares one capability with another
    /// descriptor of the same role", which could mis-classify a
    /// broad specialist as swappable even when it was the only
    /// provider of a separate required capability.
    ///
    /// Stops early when the filtered catalog can no longer cover the
    /// formation requirements — that's the graceful end of the pool,
    /// not an error. Returns whatever candidates were produced.
    ///
    /// Returns the underlying [`CatalogCompileFailure`] **only** when
    /// the very first iteration fails (i.e. the catalog can't satisfy
    /// the template even unfiltered). All later failures are absorbed
    /// as "pool exhausted" and the loop stops.
    ///
    /// **Cost.** Per produced candidate, this runs `1 + roster_size`
    /// compiles: one to produce the candidate, and one per chosen
    /// descriptor to test swappability. `compile_from_catalog` is
    /// pure metadata work (no executable instantiation), so the
    /// constant is small.
    ///
    /// `k = 0` is well-defined and returns `Ok(vec![])`. `k = 1` is
    /// equivalent to a single [`Self::compile_from_catalog`] call.
    pub fn compile_k_candidates(
        &self,
        request: &FormationCompileRequest,
        formation_templates: &FormationCatalog,
        catalog: &DiscoveryCatalog,
        providers: &ProviderDescriptorCatalog,
        k: usize,
    ) -> Result<Vec<CatalogCompiledFormationPlan>, CatalogCompileFailure> {
        let mut candidates: Vec<CatalogCompiledFormationPlan> = Vec::new();
        let mut excluded: Vec<String> = Vec::new();

        for _ in 0..k {
            let filtered = filter_out_ids(catalog, &excluded);
            match self.compile_from_catalog(
                request,
                formation_templates,
                &filtered,
                providers,
                None,
            ) {
                Ok(plan) => {
                    let excluded_before = excluded.len();
                    for role in &plan.plan.roster {
                        // Trial-compile against the catalog minus the
                        // current exclude set AND this descriptor. If
                        // the trial succeeds, the descriptor is
                        // genuinely swappable (a compositional
                        // alternative exists) and is safe to exclude
                        // for the next iteration. Scarce specialists
                        // — and broad specialists whose contribution
                        // is irreplaceable — stay available.
                        let mut trial_exclude = excluded.clone();
                        trial_exclude.push(role.suggestor_id.clone());
                        let trial_catalog = filter_out_ids(catalog, &trial_exclude);
                        if self
                            .compile_from_catalog(
                                request,
                                formation_templates,
                                &trial_catalog,
                                providers,
                                None,
                            )
                            .is_ok()
                        {
                            excluded.push(role.suggestor_id.clone());
                        }
                    }
                    let candidate_added_no_exclusions = excluded.len() == excluded_before;
                    candidates.push(plan);
                    // If no descriptor in this roster was swappable,
                    // the next iteration would compile against the
                    // same filtered catalog and produce an identical
                    // roster. Stop now to avoid emitting duplicates.
                    if candidate_added_no_exclusions {
                        break;
                    }
                }
                Err(failure) => {
                    if candidates.is_empty() {
                        return Err(failure);
                    }
                    // Pool exhausted for later candidates — graceful stop.
                    break;
                }
            }
        }

        Ok(candidates)
    }
}

/// Build a new [`DiscoveryCatalog`] containing every entry of `source`
/// whose id is not in `exclude_ids`. Used by k-best swap-out to source
/// diverse candidate rosters from the same underlying catalog.
fn filter_out_ids(source: &DiscoveryCatalog, exclude_ids: &[String]) -> DiscoveryCatalog {
    let mut filtered = DiscoveryCatalog::new();
    for entry in source {
        if !exclude_ids.iter().any(|id| id == entry.id()) {
            filtered.register(entry.clone());
        }
    }
    filtered
}

/// Catalog-aware variant of [`best_suggestor`]. Uses the
/// [`DiscoveryCatalog`]'s structural filters to source candidates and
/// records every considered candidate (chosen, outranked, already
/// selected, no coverage) so the caller can build a [`RoleDecision`].
///
/// `advisory_order` is an optional ranked list of descriptor IDs from an
/// out-of-band advisor (e.g. an LLM-backed [`CatalogLookup`]). It is
/// applied strictly as a tie-breaker after deterministic scoring —
/// candidates earlier in the list are preferred when all other ranking
/// keys are equal.
///
/// Returns `(chosen, considered)`. `chosen` is `None` when no catalog
/// entry covers any remaining requirement.
#[allow(clippy::too_many_lines)]
fn best_from_catalog<'a>(
    catalog: &'a DiscoveryCatalog,
    selected: &[&'a CatalogSuggestorDescriptor],
    unmatched_roles: &[SuggestorRole],
    unmatched_capabilities: &[SuggestorCapability],
    domain_tags: &[String],
    advisory_order: Option<&[String]>,
) -> (
    Option<&'a CatalogSuggestorDescriptor>,
    Vec<CandidateConsideration>,
) {
    // Source candidates via structural filters: union of "matches an
    // unmatched role" with "matches at least one unmatched capability".
    // Dedupe by descriptor id while preserving first occurrence order.
    let mut candidate_ids: Vec<String> = Vec::new();
    let mut candidate_refs: Vec<&CatalogSuggestorDescriptor> = Vec::new();
    let mut push = |entry: &'a CatalogSuggestorDescriptor| {
        if !candidate_ids.iter().any(|id| id == entry.id()) {
            candidate_ids.push(entry.id().to_string());
            candidate_refs.push(entry);
        }
    };
    for role in unmatched_roles {
        for entry in catalog.find_by_role(*role) {
            push(entry);
        }
    }
    for capability in unmatched_capabilities {
        for entry in catalog.find_by_capability(*capability) {
            push(entry);
        }
    }

    let mut considered: Vec<CandidateConsideration> = Vec::new();
    let mut ranked: Vec<(&CatalogSuggestorDescriptor, usize, usize, bool)> = Vec::new();

    for entry in candidate_refs {
        if selected.iter().any(|chosen| chosen.id() == entry.id()) {
            considered.push(CandidateConsideration {
                descriptor_id: entry.id().to_string(),
                disposition: CandidateDisposition::Rejected {
                    reason: RejectionReason::AlreadySelected,
                },
            });
            continue;
        }
        let coverage =
            suggestor_coverage(&entry.descriptor, unmatched_roles, unmatched_capabilities);
        if coverage == 0 {
            considered.push(CandidateConsideration {
                descriptor_id: entry.id().to_string(),
                disposition: CandidateDisposition::Rejected {
                    reason: RejectionReason::NoCoverage,
                },
            });
            continue;
        }
        let domain_hits = domain_overlap(&entry.descriptor.domain_tags, domain_tags);
        let advisory_hit =
            advisory_order.is_some_and(|order| order.iter().any(|id| id == entry.id()));
        ranked.push((entry, coverage, domain_hits, advisory_hit));
    }

    // Advisory rank: lower index = higher preference. usize::MAX for
    // entries not present in the advisory list (sorts last among ties).
    let advisory_rank = |id: &str| -> usize {
        advisory_order
            .and_then(|order| order.iter().position(|x| x == id))
            .unwrap_or(usize::MAX)
    };

    let chosen = ranked
        .iter()
        .max_by(|(left, l_cov, l_dom, _), (right, r_cov, r_dom, _)| {
            l_cov
                .cmp(r_cov)
                .then_with(|| l_dom.cmp(r_dom))
                .then_with(|| {
                    right
                        .descriptor
                        .profile
                        .cost_hint
                        .cmp(&left.descriptor.profile.cost_hint)
                })
                .then_with(|| {
                    right
                        .descriptor
                        .profile
                        .latency_hint
                        .cmp(&left.descriptor.profile.latency_hint)
                })
                .then_with(|| advisory_rank(right.id()).cmp(&advisory_rank(left.id())))
                .then_with(|| right.id().cmp(left.id()))
        })
        .map(|(entry, _, _, _)| *entry);

    for (entry, coverage, domain_hits, advisory_hit) in &ranked {
        let is_chosen = chosen.is_some_and(|c| c.id() == entry.id());
        let disposition = if is_chosen {
            CandidateDisposition::Selected {
                reason: SelectionReason {
                    coverage: *coverage,
                    domain_hits: *domain_hits,
                    advisory_hit: *advisory_hit,
                },
            }
        } else {
            let chosen_id = chosen.map(|c| c.id().to_string()).unwrap_or_default();
            CandidateDisposition::Rejected {
                reason: RejectionReason::Outranked {
                    chosen_id,
                    own_coverage: *coverage,
                    own_domain_hits: *domain_hits,
                },
            }
        };
        considered.push(CandidateConsideration {
            descriptor_id: entry.id().to_string(),
            disposition,
        });
    }

    (chosen, considered)
}

fn best_provider<'a>(
    candidates: impl Iterator<Item = &'a ProviderDescriptor>,
    descriptor: &SuggestorDescriptor,
    requirements: &BackendRequirements,
) -> Option<&'a ProviderDescriptor> {
    candidates
        .filter(|candidate| provider_satisfies(candidate, requirements))
        .map(|candidate| {
            let role_hit = usize::from(candidate.role_affinity.contains(&descriptor.profile.role));
            let domain_hits = domain_overlap(&candidate.domain_tags, &descriptor.domain_tags);
            (candidate, role_hit, domain_hits)
        })
        .max_by(
            |(left, left_role, left_domain), (right, right_role, right_domain)| {
                left_role
                    .cmp(right_role)
                    .then_with(|| left_domain.cmp(right_domain))
                    .then_with(|| {
                        right
                            .requirements
                            .max_cost_class
                            .cmp(&left.requirements.max_cost_class)
                    })
                    .then_with(|| {
                        right
                            .requirements
                            .max_latency_ms
                            .cmp(&left.requirements.max_latency_ms)
                    })
                    .then_with(|| right.id.cmp(&left.id))
            },
        )
        .map(|(candidate, _, _)| candidate)
}

fn provider_satisfies(provider: &ProviderDescriptor, requirements: &BackendRequirements) -> bool {
    provider.requirements.kind == requirements.kind
        && requirements.required_capabilities.iter().all(|capability| {
            provider
                .requirements
                .required_capabilities
                .contains(capability)
        })
        && provider.requirements.max_cost_class <= requirements.max_cost_class
        && latency_satisfies(
            provider.requirements.max_latency_ms,
            requirements.max_latency_ms,
        )
        && sovereignty_satisfies(
            provider.requirements.data_sovereignty,
            requirements.data_sovereignty,
        )
        && compliance_satisfies(provider.requirements.compliance, requirements.compliance)
        && (!requirements.requires_replay || provider.requirements.requires_replay)
        && (!requirements.requires_offline || provider.requirements.requires_offline)
}

fn latency_satisfies(provider_ms: u32, required_ms: u32) -> bool {
    required_ms == 0 || provider_ms <= required_ms
}

fn sovereignty_satisfies(provider: DataSovereignty, required: DataSovereignty) -> bool {
    match required {
        DataSovereignty::Any => true,
        _ => provider == required || provider == DataSovereignty::OnPremises,
    }
}

fn compliance_satisfies(provider: ComplianceLevel, required: ComplianceLevel) -> bool {
    required == ComplianceLevel::None || provider == required
}

fn domain_overlap(left: &[String], right: &[String]) -> usize {
    left.iter().filter(|tag| right.contains(tag)).count()
}

fn unique_capabilities(
    capabilities: impl IntoIterator<Item = SuggestorCapability>,
) -> Vec<SuggestorCapability> {
    let mut unique = Vec::new();
    for capability in capabilities {
        if !unique.contains(&capability) {
            unique.push(capability);
        }
    }
    unique
}

fn remove_role(roles: &mut Vec<SuggestorRole>, role: SuggestorRole) {
    if let Some(index) = roles.iter().position(|candidate| *candidate == role) {
        roles.remove(index);
    }
}

fn remove_capabilities(
    capabilities: &mut Vec<SuggestorCapability>,
    covered: &[SuggestorCapability],
) {
    capabilities.retain(|capability| !covered.contains(capability));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vendor_selection::vendor_selection_formation_catalog;
    use converge_kernel::formation::ProfileSnapshot;
    use converge_provider::{BackendKind, Capability, CostClass, LatencyClass};

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn profile(
        name: &str,
        role: SuggestorRole,
        output_keys: Vec<ContextKey>,
        capabilities: Vec<SuggestorCapability>,
    ) -> ProfileSnapshot {
        ProfileSnapshot {
            name: name.to_string(),
            role,
            output_keys,
            cost_hint: CostClass::Low,
            latency_hint: LatencyClass::Interactive,
            capabilities,
            confidence_min: 0.7,
            confidence_max: 0.95,
        }
    }

    fn market_scan_descriptor() -> SuggestorDescriptor {
        SuggestorDescriptor::new(
            "market-scan",
            profile(
                "market-scan",
                SuggestorRole::Signal,
                vec![ContextKey::Signals],
                vec![SuggestorCapability::KnowledgeRetrieval],
            ),
        )
        .with_read(ContextKey::Seeds)
        .with_domain_tag("vendor-selection")
        .with_output_contract(DataContract::new("MarketEvidence", "1.0"))
    }

    fn weighted_evaluator_descriptor() -> SuggestorDescriptor {
        SuggestorDescriptor::new(
            "weighted-evaluator",
            profile(
                "weighted-evaluator",
                SuggestorRole::Evaluation,
                vec![ContextKey::Evaluations],
                vec![SuggestorCapability::Analytics],
            ),
        )
        .with_read(ContextKey::Signals)
        .with_domain_tag("vendor-selection")
        .with_input_contract(DataContract::new("NormalizedVendorResponse", "1.0"))
    }

    fn policy_gate_descriptor(policy_requirements: BackendRequirements) -> SuggestorDescriptor {
        SuggestorDescriptor::new(
            "policy-gate",
            profile(
                "policy-gate",
                SuggestorRole::Constraint,
                vec![ContextKey::Constraints],
                vec![SuggestorCapability::PolicyEnforcement],
            ),
        )
        .with_read(ContextKey::Evaluations)
        .with_domain_tag("vendor-selection")
        .with_replay_mode(ReplayMode::Required)
        .with_governance_class(GovernanceClass::HumanApprovalRequired)
        .with_backend_requirements(policy_requirements)
    }

    fn decision_synthesis_descriptor() -> SuggestorDescriptor {
        SuggestorDescriptor::new(
            "decision-synthesis",
            profile(
                "decision-synthesis",
                SuggestorRole::Synthesis,
                vec![ContextKey::Proposals],
                vec![SuggestorCapability::LlmReasoning],
            ),
        )
        .with_read(ContextKey::Evaluations)
        .with_read(ContextKey::Constraints)
        .with_domain_tag("vendor-selection")
        .with_output_contract(DataContract::new("VendorSelectionDecisionRecord", "1.0"))
    }

    fn cedar_provider(policy_requirements: BackendRequirements) -> ProviderDescriptor {
        ProviderDescriptor::new(
            "cedar-local",
            "Cedar local policy engine",
            policy_requirements,
        )
        .with_role_affinity(SuggestorRole::Constraint)
        .with_domain_tag("vendor-selection")
    }

    fn complete_vendor_selection_catalogs(
        policy_requirements: BackendRequirements,
    ) -> FormationCompilerCatalogs {
        FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(market_scan_descriptor())
            .with_suggestor(weighted_evaluator_descriptor())
            .with_suggestor(policy_gate_descriptor(policy_requirements.clone()))
            .with_suggestor(decision_synthesis_descriptor())
            .with_provider(cedar_provider(policy_requirements))
    }

    #[test]
    fn compiles_complementary_vendor_selection_team() {
        let request = FormationCompileRequest::new(
            id(1),
            id(2),
            FormationTemplateQuery::new()
                .with_keyword("vendor")
                .with_keyword("diligence-evaluate-decide")
                .with_entity("VendorSelectionDecisionRecord"),
        )
        .with_tenant_id("tenant-a")
        .with_domain_tag("vendor-selection");

        let policy_requirements = BackendRequirements::access_policy().with_replay();
        let catalogs = complete_vendor_selection_catalogs(policy_requirements);

        let plan = FormationCompiler::new()
            .compile(&request, &catalogs)
            .expect("vendor selection should compile");

        assert_eq!(plan.template_id, "vendor-selection-decide");
        assert_eq!(plan.correlation_id, id(2));
        assert_eq!(plan.tenant_id.as_deref(), Some("tenant-a"));
        assert_eq!(plan.roster.len(), 4);
        assert_eq!(plan.provider_assignments.len(), 1);
        assert_eq!(plan.provider_assignments[0].provider_id, "cedar-local");
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "market-scan")
        );
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "weighted-evaluator")
        );
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "policy-gate")
        );
        assert!(
            plan.roster
                .iter()
                .any(|role| role.suggestor_id == "decision-synthesis")
        );
    }

    #[test]
    fn reports_uncovered_requirements_instead_of_over_filtering() {
        let request = FormationCompileRequest::new(
            id(3),
            id(4),
            FormationTemplateQuery::new()
                .with_keyword("vendor")
                .with_keyword("diligence-evaluate-decide"),
        );
        let catalogs = FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(SuggestorDescriptor::new(
                "analytics-only",
                profile(
                    "analytics-only",
                    SuggestorRole::Evaluation,
                    vec![ContextKey::Evaluations],
                    vec![SuggestorCapability::Analytics],
                ),
            ));

        let error = FormationCompiler::new()
            .compile(&request, &catalogs)
            .expect_err("missing roles and capabilities should be explicit");

        match error {
            FormationCompileError::UncoveredRequirements {
                unmatched_roles,
                unmatched_capabilities,
            } => {
                assert!(unmatched_roles.contains(&SuggestorRole::Signal));
                assert!(unmatched_roles.contains(&SuggestorRole::Constraint));
                assert!(unmatched_roles.contains(&SuggestorRole::Synthesis));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::KnowledgeRetrieval));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::PolicyEnforcement));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::LlmReasoning));
            }
            other => panic!("unexpected compile error: {other:?}"),
        }
    }

    #[test]
    fn requires_role_level_provider_match_when_backend_is_declared() {
        let request = FormationCompileRequest::new(
            id(5),
            id(6),
            FormationTemplateQuery::new()
                .with_keyword("vendor")
                .with_keyword("diligence-evaluate-decide"),
        );
        let policy_requirements = BackendRequirements::access_policy().with_replay();
        let catalogs = FormationCompilerCatalogs::new(vendor_selection_formation_catalog())
            .with_suggestor(market_scan_descriptor())
            .with_suggestor(weighted_evaluator_descriptor())
            .with_suggestor(policy_gate_descriptor(policy_requirements))
            .with_suggestor(decision_synthesis_descriptor())
            .with_provider(ProviderDescriptor::new(
                "generic-llm",
                "Generic LLM",
                BackendRequirements::reasoning_llm(),
            ));

        let error = FormationCompiler::new()
            .compile(&request, &catalogs)
            .expect_err("policy role should not route to an LLM provider");

        assert_eq!(
            error,
            FormationCompileError::MissingProvider {
                suggestor_id: "policy-gate".to_string(),
                role: SuggestorRole::Constraint,
            }
        );
    }

    #[test]
    fn carries_rich_provider_requirements_per_role() {
        let requirements = BackendRequirements::new(BackendKind::Llm)
            .with_capability(Capability::TextGeneration)
            .with_capability(Capability::Reasoning)
            .with_data_sovereignty(DataSovereignty::EU)
            .with_compliance(ComplianceLevel::HighExplainability)
            .with_capability(Capability::StructuredOutput);

        let descriptor = SuggestorDescriptor::new(
            "decision-synthesis",
            profile(
                "decision-synthesis",
                SuggestorRole::Synthesis,
                vec![ContextKey::Proposals],
                vec![SuggestorCapability::LlmReasoning],
            ),
        )
        .with_backend_requirements(requirements.clone());

        assert_eq!(
            descriptor
                .backend_requirements
                .as_ref()
                .expect("requirements should be present")
                .data_sovereignty,
            DataSovereignty::EU
        );
        assert!(
            descriptor
                .backend_requirements
                .as_ref()
                .expect("requirements should be present")
                .required_capabilities
                .contains(&Capability::StructuredOutput)
        );
    }

    // ------------------------------------------------------------------
    // Catalog-aware compile path — acceptance tests with a synthetic
    // 4-entry DiscoveryCatalog covering retrieve / score / optimize /
    // authorize loop contributions. These also serve as the acceptance
    // harness for the upcoming organism-catalog-mosaic seed crate.
    // ------------------------------------------------------------------

    use converge_kernel::formation::{FormationTemplate, StaticFormationTemplate};
    use organism_catalog::{
        CatalogSuggestorDescriptor, DiscoveryCatalog, DiscoveryMetadata, LoopContribution,
    };

    fn loop_demo_template_catalog() -> FormationCatalog {
        let metadata = converge_kernel::formation::FormationTemplateMetadata::new(
            "loop-demo",
            "Demonstrate Retrieve / Score / Optimize / Authorize loop coverage.",
            vec![
                SuggestorRole::Signal,
                SuggestorRole::Evaluation,
                SuggestorRole::Planning,
                SuggestorRole::Constraint,
            ],
        )
        .with_keyword("loop-demo")
        .with_required_capability(SuggestorCapability::KnowledgeRetrieval)
        .with_required_capability(SuggestorCapability::Analytics)
        .with_required_capability(SuggestorCapability::Optimization)
        .with_required_capability(SuggestorCapability::PolicyEnforcement);
        FormationCatalog::new().with_template(FormationTemplate::static_template(
            StaticFormationTemplate::new(metadata),
        ))
    }

    fn loop_demo_query() -> FormationCompileRequest {
        FormationCompileRequest::new(
            id(100),
            id(200),
            FormationTemplateQuery::new().with_keyword("loop-demo"),
        )
    }

    fn catalog_entry(
        id: &str,
        role: SuggestorRole,
        capability: SuggestorCapability,
        contribution: LoopContribution,
        summary: &str,
    ) -> CatalogSuggestorDescriptor {
        let descriptor =
            SuggestorDescriptor::new(id, profile(id, role, Vec::new(), vec![capability]));
        let discovery = DiscoveryMetadata::new(summary, "Synthetic test fixture.")
            .with_loop_contribution(contribution);
        CatalogSuggestorDescriptor::new(descriptor, discovery)
    }

    fn loop_demo_catalog_full() -> DiscoveryCatalog {
        DiscoveryCatalog::new()
            .with_entry(catalog_entry(
                "retrieve-suggestor",
                SuggestorRole::Signal,
                SuggestorCapability::KnowledgeRetrieval,
                LoopContribution::Retrieve,
                "Pull external evidence into context.",
            ))
            .with_entry(catalog_entry(
                "score-suggestor",
                SuggestorRole::Evaluation,
                SuggestorCapability::Analytics,
                LoopContribution::Score,
                "Score candidates against weighted criteria.",
            ))
            .with_entry(catalog_entry(
                "optimize-suggestor",
                SuggestorRole::Planning,
                SuggestorCapability::Optimization,
                LoopContribution::Optimize,
                "Optimize selection under declared constraints.",
            ))
            .with_entry(catalog_entry(
                "authorize-suggestor",
                SuggestorRole::Constraint,
                SuggestorCapability::PolicyEnforcement,
                LoopContribution::Authorize,
                "Authorize the proposal via a policy gate.",
            ))
    }

    #[test]
    fn catalog_compile_satisfies_four_contribution_formation() {
        let templates = loop_demo_template_catalog();
        let catalog = loop_demo_catalog_full();
        let providers = ProviderDescriptorCatalog::new();
        let request = loop_demo_query();

        let outcome = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, None)
            .expect("4-entry catalog should satisfy the loop-demo template");

        assert_eq!(outcome.plan.template_id, "loop-demo");
        assert_eq!(outcome.plan.roster.len(), 4);
        assert_eq!(outcome.decisions.len(), 4);

        let chosen: Vec<&str> = outcome
            .decisions
            .iter()
            .filter_map(|d| d.chosen.as_deref())
            .collect();
        for expected in [
            "retrieve-suggestor",
            "score-suggestor",
            "optimize-suggestor",
            "authorize-suggestor",
        ] {
            assert!(
                chosen.contains(&expected),
                "expected {expected} in decisions, got {chosen:?}"
            );
        }
    }

    #[test]
    fn catalog_compile_records_selected_disposition_with_structured_reason() {
        let templates = loop_demo_template_catalog();
        let catalog = loop_demo_catalog_full();
        let providers = ProviderDescriptorCatalog::new();
        let request = loop_demo_query();

        let outcome = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, None)
            .expect("compile should succeed");

        for decision in &outcome.decisions {
            let chosen_id = decision.chosen.as_deref().expect("each iteration chose");
            let chosen_consideration = decision
                .considered
                .iter()
                .find(|c| c.descriptor_id == chosen_id)
                .expect("chosen descriptor must appear in considered list");
            match &chosen_consideration.disposition {
                CandidateDisposition::Selected { reason } => {
                    assert!(reason.coverage >= 1, "chosen must cover at least one need");
                    // No advisor was passed, so no advisory_hit expected.
                    assert!(!reason.advisory_hit);
                }
                CandidateDisposition::Rejected { reason } => {
                    panic!("chosen descriptor must be Selected, got Rejected({reason:?})")
                }
            }
        }
    }

    #[test]
    fn catalog_compile_fails_with_partial_trace_when_capability_missing() {
        let templates = loop_demo_template_catalog();
        // Drop optimize-suggestor; the remaining catalog cannot cover
        // Planning role + Optimization capability.
        let catalog = DiscoveryCatalog::new()
            .with_entry(catalog_entry(
                "retrieve-suggestor",
                SuggestorRole::Signal,
                SuggestorCapability::KnowledgeRetrieval,
                LoopContribution::Retrieve,
                "Pull external evidence into context.",
            ))
            .with_entry(catalog_entry(
                "score-suggestor",
                SuggestorRole::Evaluation,
                SuggestorCapability::Analytics,
                LoopContribution::Score,
                "Score candidates.",
            ))
            .with_entry(catalog_entry(
                "authorize-suggestor",
                SuggestorRole::Constraint,
                SuggestorCapability::PolicyEnforcement,
                LoopContribution::Authorize,
                "Authorize via policy gate.",
            ));
        let providers = ProviderDescriptorCatalog::new();
        let request = loop_demo_query();

        let failure = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, None)
            .expect_err("missing optimize should fail to compile");

        match &failure.error {
            FormationCompileError::UncoveredRequirements {
                unmatched_roles,
                unmatched_capabilities,
            } => {
                assert!(unmatched_roles.contains(&SuggestorRole::Planning));
                assert!(unmatched_capabilities.contains(&SuggestorCapability::Optimization));
            }
            other => panic!("expected UncoveredRequirements, got {other:?}"),
        }

        // Decisions must contain a final iteration with chosen=None
        // explaining the absence.
        let final_decision = failure
            .decisions
            .last()
            .expect("partial trace must exist even on failure");
        assert!(final_decision.chosen.is_none());
        // The greedy ranker may have filled Planning's role-slot before
        // it ran out — assert via the unmatched snapshot, which is
        // authoritative for what was still open at the failing step.
        assert!(
            final_decision
                .unmatched_roles_at_start
                .contains(&SuggestorRole::Planning)
        );
    }

    #[test]
    fn catalog_compile_is_deterministic_across_repeated_runs() {
        let templates = loop_demo_template_catalog();
        let catalog = loop_demo_catalog_full();
        let providers = ProviderDescriptorCatalog::new();

        let a = FormationCompiler::new()
            .compile_from_catalog(&loop_demo_query(), &templates, &catalog, &providers, None)
            .expect("compile a");
        let b = FormationCompiler::new()
            .compile_from_catalog(&loop_demo_query(), &templates, &catalog, &providers, None)
            .expect("compile b");

        let ids_a: Vec<_> = a
            .plan
            .roster
            .iter()
            .map(|r| r.suggestor_id.clone())
            .collect();
        let ids_b: Vec<_> = b
            .plan
            .roster
            .iter()
            .map(|r| r.suggestor_id.clone())
            .collect();
        assert_eq!(ids_a, ids_b);
    }

    #[test]
    fn catalog_compile_records_outranked_disposition_for_competing_candidates() {
        // Two retrieve-capable candidates. The compiler picks one; the
        // other must appear in considered with an Outranked rejection.
        let templates = loop_demo_template_catalog();
        let catalog = loop_demo_catalog_full().with_entry(catalog_entry(
            "retrieve-suggestor-alt",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            LoopContribution::Retrieve,
            "Alternative retrieve specialist.",
        ));
        let providers = ProviderDescriptorCatalog::new();
        let request = loop_demo_query();

        let outcome = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, None)
            .expect("compile should succeed");

        let retrieve_decision = outcome
            .decisions
            .iter()
            .find(|d| d.chosen_role == Some(SuggestorRole::Signal))
            .expect("Signal-role decision should exist");
        let retrieve_alt = retrieve_decision
            .considered
            .iter()
            .find(|c| {
                c.descriptor_id == "retrieve-suggestor-alt"
                    || c.descriptor_id == "retrieve-suggestor"
            })
            .expect("at least one retrieve candidate should be considered");
        // Exactly one of the two is Selected; the other must be Outranked.
        let chosen_id = retrieve_decision.chosen.as_deref().unwrap();
        let other_id = if chosen_id == "retrieve-suggestor" {
            "retrieve-suggestor-alt"
        } else {
            "retrieve-suggestor"
        };
        let other = retrieve_decision
            .considered
            .iter()
            .find(|c| c.descriptor_id == other_id)
            .expect("other retrieve candidate should appear");
        match &other.disposition {
            CandidateDisposition::Rejected {
                reason: RejectionReason::Outranked { chosen_id: cid, .. },
            } => assert_eq!(cid, chosen_id),
            other => panic!("expected Outranked, got {other:?}"),
        }
        let _ = retrieve_alt; // suppress unused warning
    }

    #[test]
    fn catalog_compile_trace_reports_actual_chosen_role_when_later_role_wins() {
        // Scenario: the greedy ranker picks a candidate whose role is
        // NOT the first remaining role, because that candidate also
        // covers multiple capabilities. The trace must show:
        //   - unmatched_roles_at_start: the full snapshot at iteration start
        //   - chosen_role: the actual role filled (not the first remaining)
        //
        // This guards against the prior bug where `seeking_role` was
        // recorded as `unmatched_roles.first()`, which lied when the
        // chosen candidate actually filled a later role.
        let templates = loop_demo_template_catalog();
        // narrow-signal: 1 role + 1 cap = coverage 2
        // broad-evaluation: 1 role + 3 caps = coverage 4
        // → broad-evaluation wins iteration 1 even though Signal is first.
        let catalog = DiscoveryCatalog::new()
            .with_entry(CatalogSuggestorDescriptor::new(
                SuggestorDescriptor::new(
                    "narrow-signal",
                    profile(
                        "narrow-signal",
                        SuggestorRole::Signal,
                        Vec::new(),
                        vec![SuggestorCapability::KnowledgeRetrieval],
                    ),
                ),
                DiscoveryMetadata::new("Narrow signal.", "Test fixture."),
            ))
            .with_entry(CatalogSuggestorDescriptor::new(
                SuggestorDescriptor::new(
                    "broad-evaluation",
                    profile(
                        "broad-evaluation",
                        SuggestorRole::Evaluation,
                        Vec::new(),
                        vec![
                            SuggestorCapability::Analytics,
                            SuggestorCapability::Optimization,
                            SuggestorCapability::PolicyEnforcement,
                        ],
                    ),
                ),
                DiscoveryMetadata::new("Broad evaluation.", "Test fixture."),
            ))
            .with_entry(catalog_entry(
                "narrow-planning",
                SuggestorRole::Planning,
                SuggestorCapability::Optimization,
                LoopContribution::Optimize,
                "Narrow planning.",
            ))
            .with_entry(catalog_entry(
                "narrow-constraint",
                SuggestorRole::Constraint,
                SuggestorCapability::PolicyEnforcement,
                LoopContribution::Authorize,
                "Narrow constraint.",
            ));
        let providers = ProviderDescriptorCatalog::new();
        let request = loop_demo_query();

        let outcome = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, None)
            .expect("compile should succeed");

        let first = &outcome.decisions[0];

        // Sanity: the trace snapshot at iteration 1 includes ALL four
        // unfilled roles in the original order.
        assert_eq!(
            first.unmatched_roles_at_start,
            vec![
                SuggestorRole::Signal,
                SuggestorRole::Evaluation,
                SuggestorRole::Planning,
                SuggestorRole::Constraint,
            ],
        );

        // The fix: chosen_role must reflect what was filled — Evaluation,
        // NOT the first remaining (Signal). The previous shape would
        // have recorded seeking_role = Some(Signal), which lied about
        // what the compiler actually did.
        assert_eq!(first.chosen.as_deref(), Some("broad-evaluation"));
        assert_eq!(first.chosen_role, Some(SuggestorRole::Evaluation));
        assert_ne!(
            first.chosen_role,
            first.unmatched_roles_at_start.first().copied(),
            "chosen_role must reflect actual fill, not the first remaining role"
        );
    }

    #[test]
    fn catalog_compile_advisory_order_breaks_ties_but_not_coverage() {
        // Two equally-scoring retrieve candidates. With no advisor, deterministic
        // id ordering picks the lexicographically-later id ("retrieve-suggestor-alt"
        // > "retrieve-suggestor" — but the comparator picks via
        // `right.id().cmp(left.id())` so "retrieve-suggestor" wins on the last
        // tie-breaker because it's lexicographically lesser → right_id is greater
        // → comparison favors left). Confirm advisory_order can flip the choice.
        let templates = loop_demo_template_catalog();
        let catalog = loop_demo_catalog_full().with_entry(catalog_entry(
            "retrieve-suggestor-alt",
            SuggestorRole::Signal,
            SuggestorCapability::KnowledgeRetrieval,
            LoopContribution::Retrieve,
            "Alternative retrieve specialist.",
        ));
        let providers = ProviderDescriptorCatalog::new();
        let request = loop_demo_query();

        // Baseline: no advisor.
        let baseline = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, None)
            .expect("baseline compile");
        let baseline_signal_pick = baseline
            .decisions
            .iter()
            .find(|d| d.chosen_role == Some(SuggestorRole::Signal))
            .and_then(|d| d.chosen.clone())
            .unwrap();

        // Advisory: prefer the other id.
        let other = if baseline_signal_pick == "retrieve-suggestor" {
            "retrieve-suggestor-alt"
        } else {
            "retrieve-suggestor"
        };
        let advisory = vec![other.to_string()];
        let advised = FormationCompiler::new()
            .compile_from_catalog(&request, &templates, &catalog, &providers, Some(&advisory))
            .expect("advised compile");
        let advised_signal_pick = advised
            .decisions
            .iter()
            .find(|d| d.chosen_role == Some(SuggestorRole::Signal))
            .and_then(|d| d.chosen.clone())
            .unwrap();
        assert_eq!(advised_signal_pick, other);

        // Sanity: advisory cannot create coverage. If the advisor names a
        // descriptor that doesn't exist in the catalog, the chosen still
        // comes from real candidates.
        let bogus_advisory = vec!["does-not-exist".to_string()];
        let unaffected = FormationCompiler::new()
            .compile_from_catalog(
                &request,
                &templates,
                &catalog,
                &providers,
                Some(&bogus_advisory),
            )
            .expect("bogus advisor compile");
        assert_eq!(
            unaffected
                .decisions
                .iter()
                .find(|d| d.chosen_role == Some(SuggestorRole::Signal))
                .and_then(|d| d.chosen.clone())
                .unwrap(),
            baseline_signal_pick
        );
    }
}
