//! Discovery-aware catalog holding [`CatalogSuggestorDescriptor`]s, with
//! deterministic structural and keyword lookup primitives.
//!
//! This is the searchable layer that semantic lookup (LLM, embedding) sits
//! on top of. All primitives here are deterministic and side-effect-free —
//! they take borrows and return iterators of references. No async, no
//! network, no provider dependencies.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_pack::FactFamilyId;
use serde::{Deserialize, Serialize};

use crate::{CatalogSuggestorDescriptor, LoopContribution};

/// Registry of [`CatalogSuggestorDescriptor`]s with structural and keyword
/// lookup. Append-only, ordered, serializable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryCatalog {
    entries: Vec<CatalogSuggestorDescriptor>,
}

impl DiscoveryCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_entry(mut self, entry: CatalogSuggestorDescriptor) -> Self {
        self.register(entry);
        self
    }

    pub fn register(&mut self, entry: CatalogSuggestorDescriptor) {
        self.entries.push(entry);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CatalogSuggestorDescriptor> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the entry with the given descriptor id, if present.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&CatalogSuggestorDescriptor> {
        self.entries.iter().find(|entry| entry.id() == id)
    }

    // -- structural lookup ----------------------------------------------

    pub fn find_by_capability(
        &self,
        capability: SuggestorCapability,
    ) -> impl Iterator<Item = &CatalogSuggestorDescriptor> {
        self.entries
            .iter()
            .filter(move |entry| entry.descriptor.profile.capabilities.contains(&capability))
    }

    pub fn find_by_role(
        &self,
        role: SuggestorRole,
    ) -> impl Iterator<Item = &CatalogSuggestorDescriptor> {
        self.entries
            .iter()
            .filter(move |entry| entry.descriptor.profile.role == role)
    }

    pub fn find_by_loop_contribution(
        &self,
        contribution: LoopContribution,
    ) -> impl Iterator<Item = &CatalogSuggestorDescriptor> {
        self.entries
            .iter()
            .filter(move |entry| entry.discovery.loop_contributions.contains(&contribution))
    }

    pub fn find_by_domain_tag<'a>(
        &'a self,
        tag: &'a str,
    ) -> impl Iterator<Item = &'a CatalogSuggestorDescriptor> {
        self.entries
            .iter()
            .filter(move |entry| entry.descriptor.domain_tags.iter().any(|t| t == tag))
    }

    pub fn find_producing<'a>(
        &'a self,
        family: &'a FactFamilyId,
    ) -> impl Iterator<Item = &'a CatalogSuggestorDescriptor> {
        self.entries
            .iter()
            .filter(move |entry| entry.discovery.produces.iter().any(|f| f == family))
    }

    pub fn find_reading<'a>(
        &'a self,
        key: &'a ContextKey,
    ) -> impl Iterator<Item = &'a CatalogSuggestorDescriptor> {
        self.entries
            .iter()
            .filter(move |entry| entry.descriptor.reads.iter().any(|k| k == key))
    }

    // -- keyword lookup -------------------------------------------------

    /// Case-insensitive substring search across the discovery `summary`,
    /// `use_when`, and `examples` fields. Returns entries whose discovery
    /// text contains the query (no scoring — that's [`crate::lookup`]).
    pub fn find_by_keyword<'a>(
        &'a self,
        query: &'a str,
    ) -> impl Iterator<Item = &'a CatalogSuggestorDescriptor> {
        let needle = query.to_lowercase();
        self.entries.iter().filter(move |entry| {
            let d = &entry.discovery;
            haystack_contains(&d.summary, &needle)
                || haystack_contains(&d.use_when, &needle)
                || d.examples.iter().any(|ex| haystack_contains(ex, &needle))
        })
    }
}

impl<'a> IntoIterator for &'a DiscoveryCatalog {
    type IntoIter = std::slice::Iter<'a, CatalogSuggestorDescriptor>;
    type Item = &'a CatalogSuggestorDescriptor;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

fn haystack_contains(haystack: &str, needle_lowercase: &str) -> bool {
    haystack.to_lowercase().contains(needle_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::formation::{ProfileSnapshot, SuggestorCapability, SuggestorRole};
    use converge_provider::{CostClass, LatencyClass};

    use crate::{DiscoveryMetadata, SuggestorDescriptor};

    fn snap(name: &str, role: SuggestorRole, caps: Vec<SuggestorCapability>) -> ProfileSnapshot {
        ProfileSnapshot {
            name: name.to_string(),
            role,
            output_keys: Vec::new(),
            cost_hint: CostClass::Low,
            latency_hint: LatencyClass::Interactive,
            capabilities: caps,
            confidence_min: 0.7,
            confidence_max: 0.95,
        }
    }

    struct EntrySpec<'a> {
        id: &'a str,
        role: SuggestorRole,
        caps: Vec<SuggestorCapability>,
        domain: &'a str,
        produces: Vec<FactFamilyId>,
        reads: Vec<ContextKey>,
        contributions: Vec<LoopContribution>,
        summary: &'a str,
        use_when: &'a str,
        examples: Vec<&'a str>,
    }

    fn entry(spec: EntrySpec<'_>) -> CatalogSuggestorDescriptor {
        let mut descriptor = SuggestorDescriptor::new(spec.id, snap(spec.id, spec.role, spec.caps))
            .with_domain_tag(spec.domain);
        for key in spec.reads {
            descriptor = descriptor.with_read(key);
        }
        let mut discovery = DiscoveryMetadata::new(spec.summary, spec.use_when);
        for ex in spec.examples {
            discovery = discovery.with_example(ex);
        }
        for c in spec.contributions {
            discovery = discovery.with_loop_contribution(c);
        }
        for f in spec.produces {
            discovery = discovery.with_produces(f);
        }
        CatalogSuggestorDescriptor::new(descriptor, discovery)
    }

    fn fixture() -> DiscoveryCatalog {
        DiscoveryCatalog::new()
            .with_entry(entry(EntrySpec {
                id: "gleif-lookup",
                role: SuggestorRole::Signal,
                caps: vec![SuggestorCapability::KnowledgeRetrieval],
                domain: "vendor-selection",
                produces: vec![FactFamilyId::from("legal-entity.gleif")],
                reads: vec![ContextKey::Seeds],
                contributions: vec![LoopContribution::Retrieve, LoopContribution::Validate],
                summary: "Look up legal entity in GLEIF LEI registry.",
                use_when: "When verifying a company is a registered legal entity.",
                examples: vec!["verify this vendor is a real company"],
            }))
            .with_entry(entry(EntrySpec {
                id: "budget-gate",
                role: SuggestorRole::Constraint,
                caps: vec![SuggestorCapability::PolicyEnforcement],
                domain: "vendor-selection",
                produces: vec![FactFamilyId::from("policy.budget")],
                reads: vec![ContextKey::Constraints],
                contributions: vec![LoopContribution::Authorize],
                summary: "Gate proposals against a budget envelope.",
                use_when: "When a proposal must not exceed declared spend.",
                examples: vec!["does this exceed our budget"],
            }))
            .with_entry(entry(EntrySpec {
                id: "cpsat-solver",
                role: SuggestorRole::Constraint,
                caps: vec![SuggestorCapability::Optimization],
                domain: "optimization",
                produces: vec![FactFamilyId::from("optim.cpsat")],
                reads: vec![ContextKey::Constraints],
                contributions: vec![LoopContribution::Optimize],
                summary: "CP-SAT constraint-satisfaction solver.",
                use_when: "When you need an optimal assignment under constraints.",
                examples: vec!["pick the best schedule given constraints"],
            }))
    }

    #[test]
    fn find_by_capability_filters_by_capability() {
        let catalog = fixture();
        let hits: Vec<_> = catalog
            .find_by_capability(SuggestorCapability::Optimization)
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        assert_eq!(hits, vec!["cpsat-solver"]);
    }

    #[test]
    fn find_by_role_filters_by_role() {
        let catalog = fixture();
        let hits: Vec<_> = catalog
            .find_by_role(SuggestorRole::Constraint)
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        assert_eq!(hits, vec!["budget-gate", "cpsat-solver"]);
    }

    #[test]
    fn find_by_loop_contribution_filters_by_contribution() {
        let catalog = fixture();
        let hits: Vec<_> = catalog
            .find_by_loop_contribution(LoopContribution::Validate)
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        assert_eq!(hits, vec!["gleif-lookup"]);
    }

    #[test]
    fn find_by_domain_tag_matches_exact_tag() {
        let catalog = fixture();
        let hits: Vec<_> = catalog
            .find_by_domain_tag("vendor-selection")
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        assert_eq!(hits, vec!["gleif-lookup", "budget-gate"]);
    }

    #[test]
    fn find_producing_matches_fact_family() {
        let catalog = fixture();
        let family = FactFamilyId::from("legal-entity.gleif");
        let hits: Vec<_> = catalog
            .find_producing(&family)
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        assert_eq!(hits, vec!["gleif-lookup"]);
    }

    #[test]
    fn find_reading_matches_context_key() {
        let catalog = fixture();
        let key = ContextKey::Constraints;
        let hits: Vec<_> = catalog
            .find_reading(&key)
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        assert_eq!(hits, vec!["budget-gate", "cpsat-solver"]);
    }

    #[test]
    fn find_by_keyword_is_case_insensitive_and_scans_examples() {
        let catalog = fixture();
        let hits: Vec<_> = catalog
            .find_by_keyword("VENDOR")
            .map(CatalogSuggestorDescriptor::id)
            .collect();
        // "vendor" appears in gleif-lookup's example.
        assert!(hits.contains(&"gleif-lookup"));
    }

    #[test]
    fn get_returns_entry_by_id() {
        let catalog = fixture();
        assert!(catalog.get("budget-gate").is_some());
        assert!(catalog.get("does-not-exist").is_none());
    }
}
