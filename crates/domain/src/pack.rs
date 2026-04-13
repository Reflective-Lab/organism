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
