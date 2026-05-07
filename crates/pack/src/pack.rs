//! Pack framework contract types.
//!
//! Vocabulary used by domain packs and the runtime registry. Packs (e.g. those
//! shipped by `organism-domain`) describe their agents, invariants, and
//! resolution profile in these types. The runtime consumes them via
//! `organism_runtime::Registry` to drive intent resolution.
//!
//! When wired to Converge, agents implement `converge_pack::Suggestor`
//! and invariants implement `converge_pack::Invariant`.

use serde::{Deserialize, Serialize};

/// Context key partitions — mirrors converge-pack's ContextKey.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextKey {
    Seeds,
    Signals,
    Proposals,
    Evaluations,
    Strategies,
    Constraints,
    Hypotheses,
    Diagnostic,
    Votes,
    Disagreements,
    ConsensusOutcomes,
}

/// Invariant severity class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvariantClass {
    /// Checked every merge, rejects immediately.
    Structural,
    /// End of cycle, blocks convergence.
    Semantic,
    /// Convergence claim, rejects result.
    Acceptance,
}

/// Metadata for a domain pack agent.
#[derive(Debug, Clone)]
pub struct AgentMeta {
    pub name: &'static str,
    pub dependencies: &'static [ContextKey],
    pub fact_prefix: &'static str,
    pub target_key: ContextKey,
    pub description: &'static str,
}

/// Metadata for a domain pack invariant.
#[derive(Debug, Clone)]
pub struct InvariantMeta {
    pub name: &'static str,
    pub class: InvariantClass,
    pub description: &'static str,
}

/// A domain pack: a named collection of agents and invariants.
pub trait Pack {
    fn name(&self) -> &str;
    fn agents(&self) -> &[AgentMeta];
    fn invariants(&self) -> &[InvariantMeta];
}

/// Extended metadata for intent resolution matching.
/// Declared per-pack, consumed by the registry and resolver.
#[derive(Debug, Clone, Default)]
pub struct PackProfile {
    /// Domain entities this pack handles (e.g., "lead", "vendor", "contract").
    pub entities: &'static [&'static str],
    /// Capabilities this pack needs to function (e.g., "linkedin", "web", "ocr").
    pub required_capabilities: &'static [&'static str],
    /// Whether agents in this pack call LLMs (affects cost profile).
    pub uses_llm: bool,
    /// Whether this pack requires HITL gates for high-stakes decisions.
    pub requires_hitl: bool,
    /// Minimum reversibility level this pack handles safely.
    /// Packs with Acceptance invariants can handle irreversible intents.
    pub handles_irreversible: bool,
    /// Keywords for semantic matching beyond agent descriptions.
    pub keywords: &'static [&'static str],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_key_serde_roundtrip() {
        for key in [
            ContextKey::Seeds,
            ContextKey::Signals,
            ContextKey::Proposals,
            ContextKey::Evaluations,
            ContextKey::Strategies,
            ContextKey::Constraints,
            ContextKey::Hypotheses,
            ContextKey::Diagnostic,
            ContextKey::Votes,
            ContextKey::Disagreements,
            ContextKey::ConsensusOutcomes,
        ] {
            let json = serde_json::to_string(&key).unwrap();
            let back: ContextKey = serde_json::from_str(&json).unwrap();
            assert_eq!(key, back);
        }
    }

    #[test]
    fn context_key_snake_case_serialization() {
        assert_eq!(
            serde_json::to_string(&ContextKey::Seeds).unwrap(),
            "\"seeds\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::ConsensusOutcomes).unwrap(),
            "\"consensus_outcomes\""
        );
    }

    #[test]
    fn context_key_rejects_unknown_variant() {
        let result = serde_json::from_str::<ContextKey>("\"nonexistent\"");
        assert!(result.is_err());
    }

    #[test]
    fn invariant_class_serde_roundtrip() {
        for class in [
            InvariantClass::Structural,
            InvariantClass::Semantic,
            InvariantClass::Acceptance,
        ] {
            let json = serde_json::to_string(&class).unwrap();
            let back: InvariantClass = serde_json::from_str(&json).unwrap();
            assert_eq!(class, back);
        }
    }

    #[test]
    fn agent_meta_fields() {
        let agent = AgentMeta {
            name: "test_agent",
            dependencies: &[ContextKey::Seeds, ContextKey::Signals],
            fact_prefix: "test:",
            target_key: ContextKey::Proposals,
            description: "A test agent",
        };
        assert_eq!(agent.name, "test_agent");
        assert_eq!(agent.dependencies.len(), 2);
        assert_eq!(agent.target_key, ContextKey::Proposals);
    }

    #[test]
    fn invariant_meta_fields() {
        let inv = InvariantMeta {
            name: "test_invariant",
            class: InvariantClass::Acceptance,
            description: "Must pass",
        };
        assert_eq!(inv.name, "test_invariant");
        assert_eq!(inv.class, InvariantClass::Acceptance);
    }

    #[test]
    fn pack_profile_default() {
        let profile = PackProfile::default();
        assert!(profile.entities.is_empty());
        assert!(profile.required_capabilities.is_empty());
        assert!(!profile.uses_llm);
        assert!(!profile.requires_hitl);
        assert!(!profile.handles_irreversible);
        assert!(profile.keywords.is_empty());
    }
}
