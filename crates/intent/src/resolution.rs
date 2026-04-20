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

    #[test]
    fn declarative_binding_empty() {
        let binding = DeclarativeBinding::new().build();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
        assert_eq!(
            binding.resolution.levels_attempted,
            vec![ResolutionLevel::Declarative]
        );
        assert_eq!(
            binding.resolution.levels_contributed,
            vec![ResolutionLevel::Declarative]
        );
        assert_eq!(binding.resolution.prior_episodes_consulted, 0);
    }

    #[test]
    fn declarative_binding_pack_confidence_is_one() {
        let binding = DeclarativeBinding::new().pack("test", "reason").build();
        assert!((binding.packs[0].confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn declarative_binding_capability_confidence_is_one() {
        let binding = DeclarativeBinding::new()
            .capability("ocr", "doc processing")
            .build();
        assert!((binding.capabilities[0].confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn declarative_binding_multiple_invariants() {
        let binding = DeclarativeBinding::new()
            .invariant("inv_a")
            .invariant("inv_b")
            .invariant("inv_c")
            .build();
        assert_eq!(binding.invariants, vec!["inv_a", "inv_b", "inv_c"]);
    }

    #[test]
    fn declarative_binding_default() {
        let binding = DeclarativeBinding::default();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
    }

    #[test]
    fn intent_binding_default() {
        let binding = IntentBinding::default();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
        assert!(binding.resolution.levels_attempted.is_empty());
        assert!(binding.resolution.levels_contributed.is_empty());
        assert_eq!(binding.resolution.prior_episodes_consulted, 0);
        assert!((binding.resolution.completeness_confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resolution_trace_default() {
        let trace = ResolutionTrace::default();
        assert!(trace.levels_attempted.is_empty());
        assert!(trace.levels_contributed.is_empty());
        assert_eq!(trace.prior_episodes_consulted, 0);
        assert!((trace.completeness_confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resolution_level_all_variants_distinct() {
        let variants = [
            ResolutionLevel::Declarative,
            ResolutionLevel::Structural,
            ResolutionLevel::Semantic,
            ResolutionLevel::Learned,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b);
            }
        }
    }

    #[test]
    fn resolution_level_serde_roundtrip() {
        for level in [
            ResolutionLevel::Declarative,
            ResolutionLevel::Structural,
            ResolutionLevel::Semantic,
            ResolutionLevel::Learned,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let back: ResolutionLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, back);
        }
    }

    #[test]
    fn resolution_level_snake_case() {
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Declarative).unwrap(),
            "\"declarative\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Structural).unwrap(),
            "\"structural\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Semantic).unwrap(),
            "\"semantic\""
        );
        assert_eq!(
            serde_json::to_string(&ResolutionLevel::Learned).unwrap(),
            "\"learned\""
        );
    }

    #[test]
    fn pack_requirement_serde_roundtrip() {
        let req = PackRequirement {
            pack_name: "customers".into(),
            reason: "lead workflow".into(),
            confidence: 0.85,
            source: ResolutionLevel::Structural,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: PackRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pack_name, "customers");
        assert_eq!(back.reason, "lead workflow");
        assert!((back.confidence - 0.85).abs() < f64::EPSILON);
        assert_eq!(back.source, ResolutionLevel::Structural);
    }

    #[test]
    fn capability_requirement_serde_roundtrip() {
        let req = CapabilityRequirement {
            capability: "vision".into(),
            reason: "document scanning".into(),
            confidence: 0.7,
            source: ResolutionLevel::Semantic,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: CapabilityRequirement = serde_json::from_str(&json).unwrap();
        assert_eq!(back.capability, "vision");
        assert_eq!(back.source, ResolutionLevel::Semantic);
    }

    #[test]
    fn intent_binding_serde_roundtrip() {
        let binding = DeclarativeBinding::new()
            .pack("dd", "due diligence")
            .capability("web", "scraping")
            .invariant("hypothesis_has_source")
            .build();

        let json = serde_json::to_string(&binding).unwrap();
        let back: IntentBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.packs.len(), 1);
        assert_eq!(back.capabilities.len(), 1);
        assert_eq!(back.invariants, vec!["hypothesis_has_source"]);
        assert_eq!(
            back.resolution.levels_attempted,
            vec![ResolutionLevel::Declarative]
        );
    }

    #[test]
    fn resolution_trace_serde_roundtrip() {
        let trace = ResolutionTrace {
            levels_attempted: vec![ResolutionLevel::Declarative, ResolutionLevel::Structural],
            levels_contributed: vec![ResolutionLevel::Declarative],
            prior_episodes_consulted: 42,
            completeness_confidence: 0.95,
        };
        let json = serde_json::to_string(&trace).unwrap();
        let back: ResolutionTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(back.levels_attempted.len(), 2);
        assert_eq!(back.levels_contributed.len(), 1);
        assert_eq!(back.prior_episodes_consulted, 42);
        assert!((back.completeness_confidence - 0.95).abs() < f64::EPSILON);
    }
}
