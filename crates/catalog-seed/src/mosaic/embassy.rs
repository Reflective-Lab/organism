//! Descriptors for `converge-embassy-pack` / `converge-embassy-linkedin`
//! Suggestors.
//!
//! Authored against `converge-embassy-pack = "1.3.0"` and
//! `converge-embassy-linkedin = "1.3.0"`. Embassy crates expose
//! external-data lookups (legal entity registries, sanctions lists,
//! tax/VAT validations, government notices).

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        gleif_lookup(),
        ofac_sls_screen(),
        eu_sanctions_screen(),
        vat_vies_validation(),
        ted_notice_retrieval(),
    ]
}

#[must_use]
pub fn gleif_lookup() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "embassy-gleif-lookup",
        role: SuggestorRole::Signal,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Signals],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["lookup", "legal-entity", "lei", "gleif", "vendor-selection"],
        cost: CostClass::High,
        latency: LatencyClass::Interactive,
        summary: "Look up a legal entity in the GLEIF LEI registry.",
        use_when: "When verifying a company exists as a registered legal entity and resolving its LEI / jurisdiction.",
        examples: vec![
            "verify this vendor is a real company",
            "find the LEI for Acme Corp",
            "what jurisdiction is this entity registered in",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Validate],
        produces: vec!["embassy.legal-entity.gleif"],
    })
}

#[must_use]
pub fn ofac_sls_screen() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "embassy-ofac-sls-screen",
        role: SuggestorRole::Signal,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Signals, ContextKey::Constraints],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["lookup", "sanctions", "ofac", "compliance"],
        cost: CostClass::High,
        latency: LatencyClass::Interactive,
        summary: "Screen an entity against the US OFAC Sanctions List Search.",
        use_when: "When a vendor or counterparty must be checked against US sanctions before engagement.",
        examples: vec![
            "is this counterparty on OFAC",
            "sanctions screen this vendor",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Validate],
        produces: vec!["embassy.sanctions.ofac-sls"],
    })
}

#[must_use]
pub fn eu_sanctions_screen() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "embassy-eu-sanctions-screen",
        role: SuggestorRole::Signal,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Signals, ContextKey::Constraints],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["lookup", "sanctions", "eu", "compliance"],
        cost: CostClass::High,
        latency: LatencyClass::Interactive,
        summary: "Screen an entity against the EU consolidated sanctions list.",
        use_when: "When a vendor or counterparty must be checked against EU sanctions.",
        examples: vec![
            "is this entity on EU sanctions",
            "screen this counterparty against EU consolidated list",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Validate],
        produces: vec!["embassy.sanctions.eu-consolidated"],
    })
}

#[must_use]
pub fn vat_vies_validation() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "embassy-vat-vies-validation",
        role: SuggestorRole::Signal,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Signals],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["lookup", "vat", "vies", "tax", "eu"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Validate an EU VAT number via the VIES service.",
        use_when: "When invoicing or contracting with an EU vendor and the VAT number must be real.",
        examples: vec![
            "is this VAT number valid",
            "verify VIES status for this supplier",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Validate],
        produces: vec!["embassy.tax.vat-vies"],
    })
}

#[must_use]
pub fn ted_notice_retrieval() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "embassy-ted-notice-retrieval",
        role: SuggestorRole::Signal,
        capabilities: vec![SuggestorCapability::KnowledgeRetrieval],
        output_keys: vec![ContextKey::Signals],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["lookup", "ted", "procurement", "eu", "tenders"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Retrieve EU TED (Tenders Electronic Daily) procurement notices.",
        use_when: "When tracking or evaluating EU public-procurement opportunities.",
        examples: vec![
            "find recent TED notices in this CPV code",
            "track tenders relevant to this vendor",
        ],
        loop_contributions: vec![LoopContribution::Retrieve],
        produces: vec!["embassy.procurement.ted"],
    })
}
