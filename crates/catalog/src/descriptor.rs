//! Core descriptor types — extracted from `organism-runtime::compiler`.
//!
//! These are the serializable, side-effect-free building blocks that the
//! formation compiler and any lookup layer reason over. They carry no
//! factory references and never touch live Suggestor instances.

use std::borrow::Borrow;
use std::fmt;

use converge_kernel::ContextKey;
use converge_kernel::formation::ProfileSnapshot;
use converge_provider::BackendRequirements;
use serde::{Deserialize, Serialize};

/// Stable, human-readable identifier for a [`SuggestorDescriptor`] in
/// the Organism catalog (e.g. `"signal-a"`, `"decision-synthesis"`).
///
/// This is intentionally **not** `converge_core::SuggestorId` (which
/// is a `u32` ordering token internal to Converge's engine). Organism
/// owns the human-readable descriptor name, so we own a typed wrapper
/// for it. Serializes transparently as the inner string so wire
/// format is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SuggestorDescriptorId(String);

impl SuggestorDescriptorId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }

    #[must_use]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    #[must_use]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }

    #[must_use]
    pub fn contains(&self, needle: &str) -> bool {
        self.0.contains(needle)
    }
}

impl fmt::Display for SuggestorDescriptorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for SuggestorDescriptorId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for SuggestorDescriptorId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for SuggestorDescriptorId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<&str> for SuggestorDescriptorId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for SuggestorDescriptorId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for SuggestorDescriptorId {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<SuggestorDescriptorId> for String {
    fn from(id: SuggestorDescriptorId) -> Self {
        id.0
    }
}

impl PartialEq<str> for SuggestorDescriptorId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for SuggestorDescriptorId {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_str() == *other
    }
}

impl PartialEq<String> for SuggestorDescriptorId {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<SuggestorDescriptorId> for str {
    fn eq(&self, other: &SuggestorDescriptorId) -> bool {
        self == other.0.as_str()
    }
}

impl PartialEq<SuggestorDescriptorId> for &str {
    fn eq(&self, other: &SuggestorDescriptorId) -> bool {
        *self == other.0.as_str()
    }
}

impl PartialEq<SuggestorDescriptorId> for String {
    fn eq(&self, other: &SuggestorDescriptorId) -> bool {
        self == &other.0
    }
}

/// Stable, human-readable identifier for a [`ProviderDescriptor`] in
/// the Organism catalog (e.g. `"reasoning-llm"`, `"cedar-local"`).
/// Same shape as [`SuggestorDescriptorId`] — wraps `String` with
/// `#[serde(transparent)]` so wire format stays a bare string while
/// in-memory code passes a typed handle around.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderId(String);

impl ProviderId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ProviderId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for ProviderId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for ProviderId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ProviderId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ProviderId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for ProviderId {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<ProviderId> for String {
    fn from(id: ProviderId) -> Self {
        id.0
    }
}

impl PartialEq<str> for ProviderId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for ProviderId {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_str() == *other
    }
}

impl PartialEq<String> for ProviderId {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<ProviderId> for &str {
    fn eq(&self, other: &ProviderId) -> bool {
        *self == other.0.as_str()
    }
}

impl PartialEq<ProviderId> for String {
    fn eq(&self, other: &ProviderId) -> bool {
        self == &other.0
    }
}

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
    pub id: SuggestorDescriptorId,
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
    pub fn new(id: impl Into<SuggestorDescriptorId>, profile: ProfileSnapshot) -> Self {
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
    pub id: ProviderId,
    pub label: String,
    pub requirements: BackendRequirements,
    pub role_affinity: Vec<converge_kernel::formation::SuggestorRole>,
    pub domain_tags: Vec<String>,
}

impl ProviderDescriptor {
    pub fn new(
        id: impl Into<ProviderId>,
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
