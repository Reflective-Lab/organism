//! Intent resolution — maps intents to the packs, capabilities, and invariants
//! needed for convergence.
//!
//! Four resolution levels:
//!
//! 1. **Declarative** — intent explicitly declares which packs it needs
//! 2. **Structural** — resolver matches fact prefixes to pack metadata
//! 3. **Semantic** — huddle matches outcome description to pack capabilities
//! 4. **Learned** — prior calibration from execution history predicts pack needs
//!
//! Resolution runs after admission, before planning. The output is an
//! `IntentBinding` that tells the runtime which agents to register
//! with the Converge engine.

use serde::{Deserialize, Serialize};

// ── Intent Binding ─────────────────────────────────────────────────

/// The output of intent resolution. Tells the runtime what to wire up.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntentBinding {
    /// Which domain packs to register with the engine.
    pub packs: Vec<PackRequirement>,
    /// Which capabilities the intent needs (OCR, web, vision, etc.).
    pub capabilities: Vec<CapabilityRequirement>,
    /// Additional invariants to enforce beyond pack defaults.
    pub invariants: Vec<String>,
    /// How the binding was resolved.
    pub resolution: ResolutionTrace,
}

/// A domain pack needed by the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRequirement {
    pub pack_name: String,
    pub reason: String,
    pub confidence: f64,
    pub source: ResolutionLevel,
}

/// A capability needed by the intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequirement {
    pub capability: String,
    pub reason: String,
    pub confidence: f64,
    pub source: ResolutionLevel,
}

/// Which resolution level produced the binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionLevel {
    /// Intent explicitly declared its packs.
    Declarative,
    /// Resolver matched fact prefixes to pack metadata.
    Structural,
    /// Huddle matched outcome to pack descriptions.
    Semantic,
    /// Prior calibration predicted from execution history.
    Learned,
}

/// How the resolution was performed — for traceability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionTrace {
    pub levels_attempted: Vec<ResolutionLevel>,
    pub levels_contributed: Vec<ResolutionLevel>,
    /// Number of prior episodes consulted (level 4).
    pub prior_episodes_consulted: usize,
    /// Confidence that the binding is complete.
    pub completeness_confidence: f64,
}

// ── Declarative Binding (Level 1) ──────────────────────────────────

/// Builder for declaring an intent's resource needs explicitly.
/// This is what apps use today.
///
/// ```rust,ignore
/// let binding = DeclarativeBinding::new()
///     .pack("customers", "lead qualification workflow")
///     .pack("linkedin_research", "enrich with LinkedIn data")
///     .capability("web", "capture company website")
///     .invariant("lead_has_source")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct DeclarativeBinding {
    packs: Vec<PackRequirement>,
    capabilities: Vec<CapabilityRequirement>,
    invariants: Vec<String>,
}

impl DeclarativeBinding {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn pack(mut self, name: impl Into<String>, reason: impl Into<String>) -> Self {
        self.packs.push(PackRequirement {
            pack_name: name.into(),
            reason: reason.into(),
            confidence: 1.0,
            source: ResolutionLevel::Declarative,
        });
        self
    }

    #[must_use]
    pub fn capability(mut self, name: impl Into<String>, reason: impl Into<String>) -> Self {
        self.capabilities.push(CapabilityRequirement {
            capability: name.into(),
            reason: reason.into(),
            confidence: 1.0,
            source: ResolutionLevel::Declarative,
        });
        self
    }

    #[must_use]
    pub fn invariant(mut self, name: impl Into<String>) -> Self {
        self.invariants.push(name.into());
        self
    }

    #[must_use]
    pub fn build(self) -> IntentBinding {
        IntentBinding {
            packs: self.packs,
            capabilities: self.capabilities,
            invariants: self.invariants,
            resolution: ResolutionTrace {
                levels_attempted: vec![ResolutionLevel::Declarative],
                levels_contributed: vec![ResolutionLevel::Declarative],
                prior_episodes_consulted: 0,
                completeness_confidence: 1.0,
            },
        }
    }
}

// ── Resolution Trait ───────────────────────────────────────────────

/// Resolves an intent to its resource binding.
///
/// Implementations exist for each level. The runtime chains them:
/// declarative first, then structural fills gaps, semantic adds
/// uncertain matches, learned adjusts confidences from history.
pub trait IntentResolver: Send + Sync {
    fn level(&self) -> ResolutionLevel;
    fn resolve(&self, intent: &super::IntentPacket, current: &IntentBinding) -> IntentBinding;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarative_binding_builds_correctly() {
        let binding = DeclarativeBinding::new()
            .pack("customers", "lead qualification")
            .pack("linkedin_research", "enrich leads")
            .capability("web", "capture company page")
            .invariant("lead_has_source")
            .build();

        assert_eq!(binding.packs.len(), 2);
        assert_eq!(binding.capabilities.len(), 1);
        assert_eq!(binding.invariants.len(), 1);
        assert_eq!(binding.packs[0].pack_name, "customers");
        assert_eq!(binding.packs[0].source, ResolutionLevel::Declarative);
        assert!((binding.resolution.completeness_confidence - 1.0).abs() < f64::EPSILON);
    }
}
