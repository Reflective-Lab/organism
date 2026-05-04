//! Pack framework types.
//!
//! These types define the structure of organizational domain packs.
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

// ── Pack Profile (resolution metadata) ─────────────────────────────

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
            serde_json::to_string(&ContextKey::Signals).unwrap(),
            "\"signals\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Proposals).unwrap(),
            "\"proposals\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Evaluations).unwrap(),
            "\"evaluations\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Strategies).unwrap(),
            "\"strategies\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Constraints).unwrap(),
            "\"constraints\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Hypotheses).unwrap(),
            "\"hypotheses\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Diagnostic).unwrap(),
            "\"diagnostic\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Votes).unwrap(),
            "\"votes\""
        );
        assert_eq!(
            serde_json::to_string(&ContextKey::Disagreements).unwrap(),
            "\"disagreements\""
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
    fn invariant_class_snake_case_serialization() {
        assert_eq!(
            serde_json::to_string(&InvariantClass::Structural).unwrap(),
            "\"structural\""
        );
        assert_eq!(
            serde_json::to_string(&InvariantClass::Semantic).unwrap(),
            "\"semantic\""
        );
        assert_eq!(
            serde_json::to_string(&InvariantClass::Acceptance).unwrap(),
            "\"acceptance\""
        );
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
        assert_eq!(agent.fact_prefix, "test:");
        assert_eq!(agent.target_key, ContextKey::Proposals);
        assert_eq!(agent.description, "A test agent");
    }

    #[test]
    fn agent_meta_clone() {
        let agent = AgentMeta {
            name: "cloneable",
            dependencies: &[ContextKey::Hypotheses],
            fact_prefix: "clone:",
            target_key: ContextKey::Evaluations,
            description: "Clone test",
        };
        let cloned = agent.clone();
        assert_eq!(cloned.name, agent.name);
        assert_eq!(cloned.fact_prefix, agent.fact_prefix);
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

    #[test]
    fn pack_profile_clone() {
        let profile = PackProfile {
            entities: &["lead", "deal"],
            required_capabilities: &["web"],
            uses_llm: true,
            requires_hitl: true,
            handles_irreversible: false,
            keywords: &["sales"],
        };
        let cloned = profile.clone();
        assert_eq!(cloned.entities, profile.entities);
        assert_eq!(cloned.uses_llm, profile.uses_llm);
        assert_eq!(cloned.requires_hitl, profile.requires_hitl);
    }
}
