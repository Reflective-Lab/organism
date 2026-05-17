//! Core descriptor types — extracted from `organism-runtime::compiler`.
//!
//! These are the serializable, side-effect-free building blocks that the
//! formation compiler and any lookup layer reason over. They carry no
//! factory references and never touch live Suggestor instances.

use converge_kernel::ContextKey;
use converge_kernel::formation::ProfileSnapshot;
use converge_provider::BackendRequirements;
use serde::{Deserialize, Serialize};

/// Named, versioned data contract carried alongside descriptor inputs and
/// outputs. Used by the compiler to verify input/output shape compatibility
/// between adjacent roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataContract {
    pub name: String,
    pub version: String,
}

impl DataContract {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// How strictly a Suggestor must be replayed under audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayMode {
    Required,
    Preferred,
    NotRequired,
}

/// The governance posture of facts produced by a Suggestor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceClass {
    LowRisk,
    BusinessDecision,
    RegulatedDecision,
    HumanApprovalRequired,
}

/// Descriptor of a Suggestor candidate. Carries the profile snapshot plus
/// the operating-envelope metadata the compiler needs to assemble a roster
/// (reads, domain tags, contracts, replay, governance, backend).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestorDescriptor {
    pub id: String,
    pub profile: ProfileSnapshot,
    pub reads: Vec<ContextKey>,
    pub domain_tags: Vec<String>,
    pub input_contracts: Vec<DataContract>,
    pub output_contracts: Vec<DataContract>,
    pub replay_mode: ReplayMode,
    pub governance_class: GovernanceClass,
    pub backend_requirements: Option<BackendRequirements>,
}

impl SuggestorDescriptor {
    pub fn new(id: impl Into<String>, profile: ProfileSnapshot) -> Self {
        Self {
            id: id.into(),
            profile,
            reads: Vec::new(),
            domain_tags: Vec::new(),
            input_contracts: Vec::new(),
            output_contracts: Vec::new(),
            replay_mode: ReplayMode::NotRequired,
            governance_class: GovernanceClass::BusinessDecision,
            backend_requirements: None,
        }
    }

    pub fn with_read(mut self, key: ContextKey) -> Self {
        self.reads.push(key);
        self
    }

    pub fn with_domain_tag(mut self, tag: impl Into<String>) -> Self {
        self.domain_tags.push(tag.into());
        self
    }

    pub fn with_input_contract(mut self, contract: DataContract) -> Self {
        self.input_contracts.push(contract);
        self
    }

    pub fn with_output_contract(mut self, contract: DataContract) -> Self {
        self.output_contracts.push(contract);
        self
    }

    pub fn with_replay_mode(mut self, mode: ReplayMode) -> Self {
        self.replay_mode = mode;
        self
    }

    pub fn with_governance_class(mut self, class: GovernanceClass) -> Self {
        self.governance_class = class;
        self
    }

    pub fn with_backend_requirements(mut self, requirements: BackendRequirements) -> Self {
        self.backend_requirements = Some(requirements);
        self
    }
}

/// Registry of Suggestor descriptors. Append-only, ordered, serializable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SuggestorDescriptorCatalog {
    descriptors: Vec<SuggestorDescriptor>,
}

impl SuggestorDescriptorCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_descriptor(mut self, descriptor: SuggestorDescriptor) -> Self {
        self.register(descriptor);
        self
    }

    pub fn register(&mut self, descriptor: SuggestorDescriptor) {
        self.descriptors.push(descriptor);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SuggestorDescriptor> {
        self.descriptors.iter()
    }
}

impl<'a> IntoIterator for &'a SuggestorDescriptorCatalog {
    type IntoIter = std::slice::Iter<'a, SuggestorDescriptor>;
    type Item = &'a SuggestorDescriptor;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Descriptor of a provider candidate (LLM backend, policy engine, etc.).
/// Carries the backend requirements it satisfies plus affinity hints used
/// by the compiler when matching providers to Suggestor roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub id: String,
    pub label: String,
    pub requirements: BackendRequirements,
    pub role_affinity: Vec<converge_kernel::formation::SuggestorRole>,
    pub domain_tags: Vec<String>,
}

impl ProviderDescriptor {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        requirements: BackendRequirements,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            requirements,
            role_affinity: Vec::new(),
            domain_tags: Vec::new(),
        }
    }

    pub fn with_role_affinity(mut self, role: converge_kernel::formation::SuggestorRole) -> Self {
        self.role_affinity.push(role);
        self
    }

    pub fn with_domain_tag(mut self, tag: impl Into<String>) -> Self {
        self.domain_tags.push(tag.into());
        self
    }
}

/// Registry of Provider descriptors. Append-only, ordered, serializable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderDescriptorCatalog {
    descriptors: Vec<ProviderDescriptor>,
}

impl ProviderDescriptorCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_descriptor(mut self, descriptor: ProviderDescriptor) -> Self {
        self.register(descriptor);
        self
    }

    pub fn register(&mut self, descriptor: ProviderDescriptor) {
        self.descriptors.push(descriptor);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ProviderDescriptor> {
        self.descriptors.iter()
    }
}

impl<'a> IntoIterator for &'a ProviderDescriptorCatalog {
    type IntoIter = std::slice::Iter<'a, ProviderDescriptor>;
    type Item = &'a ProviderDescriptor;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
