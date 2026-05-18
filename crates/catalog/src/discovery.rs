//! Additive discovery sidecar — wraps [`SuggestorDescriptor`] without
//! mutating its public surface.
//!
//! [`CatalogSuggestorDescriptor`] composes the existing descriptor with a
//! [`DiscoveryMetadata`] block carrying the natural-language summary,
//! use-when guidance, example task phrasings, the fact families the
//! Suggestor produces, and one or more [`LoopContribution`]s describing how
//! it participates in deliberation. The wrapper pattern preserves the
//! existing `SuggestorDescriptor` public API — adding fields to that type
//! would be semver-breaking.

use converge_pack::FactFamilyId;
use serde::{Deserialize, Serialize};

use crate::SuggestorDescriptor;

/// How a Suggestor contributes to Organism's deliberation loop. Plural in
/// [`DiscoveryMetadata::loop_contributions`] — real specialists often
/// combine roles (e.g. retrieve + score, validate + authorize, propose +
/// synthesize).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopContribution {
    /// Generate a candidate fact or hypothesis.
    Propose,
    /// Pull facts from an external source into context.
    Retrieve,
    /// Confirm a proposal against known invariants.
    Validate,
    /// Adversarially challenge a proposal or fact.
    Challenge,
    /// Rank or weight candidates against criteria.
    Score,
    /// Solve a constrained problem to produce an optimal choice.
    Optimize,
    /// Apply a policy gate (budget, approval, compliance, classification).
    Authorize,
    /// Emit an observation about the loop's own behavior (telemetry).
    Observe,
    /// Combine multiple inputs into a single decision or artifact.
    Synthesize,
}

/// Natural-language and discovery-oriented metadata attached to a
/// descriptor. Designed to support both deterministic keyword/structural
/// lookup and advisory LLM-backed reranking.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiscoveryMetadata {
    /// One sentence: what this Suggestor does.
    pub summary: String,
    /// One sentence: when a caller should reach for this Suggestor.
    pub use_when: String,
    /// 2–5 example task phrasings that would match this Suggestor. These
    /// are the primary grist for keyword and semantic lookup.
    pub examples: Vec<String>,
    /// How the Suggestor participates in the deliberation loop. Plural —
    /// many Suggestors contribute in more than one way.
    pub loop_contributions: Vec<LoopContribution>,
    /// Fact families this Suggestor is expected to produce. Reuses
    /// Converge's existing [`FactFamilyId`] vocabulary; no parallel "kind"
    /// taxonomy.
    pub produces: Vec<FactFamilyId>,
}

impl DiscoveryMetadata {
    /// Creates a discovery metadata block with summary and use-when text.
    #[must_use]
    pub fn new(summary: impl Into<String>, use_when: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            use_when: use_when.into(),
            examples: Vec::new(),
            loop_contributions: Vec::new(),
            produces: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    #[must_use]
    pub fn with_loop_contribution(mut self, contribution: LoopContribution) -> Self {
        self.loop_contributions.push(contribution);
        self
    }

    #[must_use]
    pub fn with_produces(mut self, family: FactFamilyId) -> Self {
        self.produces.push(family);
        self
    }
}

/// Descriptor + discovery sidecar. Wrapper, not mutation: the inner
/// [`SuggestorDescriptor`] keeps its existing public surface intact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSuggestorDescriptor {
    pub descriptor: SuggestorDescriptor,
    pub discovery: DiscoveryMetadata,
}

impl CatalogSuggestorDescriptor {
    #[must_use]
    pub fn new(descriptor: SuggestorDescriptor, discovery: DiscoveryMetadata) -> Self {
        Self {
            descriptor,
            discovery,
        }
    }

    /// Returns the descriptor id (delegated to the inner descriptor).
    #[must_use]
    pub fn id(&self) -> &crate::SuggestorDescriptorId {
        &self.descriptor.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_metadata_builder_accumulates() {
        let metadata = DiscoveryMetadata::new(
            "Look up legal entity in GLEIF.",
            "When verifying a company is a registered legal entity.",
        )
        .with_example("verify this vendor is a real company")
        .with_example("find the LEI for Acme Corp")
        .with_loop_contribution(LoopContribution::Retrieve)
        .with_loop_contribution(LoopContribution::Validate)
        .with_produces(FactFamilyId::from("legal-entity.gleif"));

        assert_eq!(metadata.examples.len(), 2);
        assert_eq!(metadata.loop_contributions.len(), 2);
        assert_eq!(
            metadata.produces,
            vec![FactFamilyId::from("legal-entity.gleif")]
        );
    }

    #[test]
    fn loop_contribution_serializes_snake_case() {
        let json = serde_json::to_string(&LoopContribution::Authorize).unwrap();
        assert_eq!(json, "\"authorize\"");
    }
}
