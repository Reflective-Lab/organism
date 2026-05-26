use serde::{Deserialize, Serialize};

/// Desired termination behavior for an Organism formation.
///
/// This is intent-level guidance. Formation runtimes may support only a subset
/// of criteria; unsupported custom signals should be surfaced as admission or
/// compilation diagnostics rather than ignored silently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConvergenceCriteria {
    /// Use the selected formation shape's default convergence behavior.
    PlatformDefault,
    /// Return after at most this many rounds, even when convergence is partial.
    MaxRounds { rounds: u32 },
    /// Converged when confidence remains stable for this many consecutive
    /// cycles or rounds.
    ConfidenceStableFor { cycles: u32 },
    /// Converged when the selected formation's member-agreement rule passes.
    ConsensusAmongMembers,
    /// Caller-supplied convergence signals interpreted by the formation
    /// compiler or runtime.
    Custom { signals: Vec<ConvergenceSignal> },
}

/// One caller-supplied convergence signal.
///
/// `ConvergenceCriteria` is the intent contract. Existing planning
/// `ConvergenceSignals` remains the runtime observation snapshot used by
/// topology-transition rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConvergenceSignal {
    pub name: String,
    pub description: String,
}

impl ConvergenceSignal {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criteria_serializes_with_stable_kind_tags() {
        let criteria = ConvergenceCriteria::ConfidenceStableFor { cycles: 3 };

        let json = serde_json::to_string(&criteria).unwrap();

        assert_eq!(json, r#"{"kind":"confidence_stable_for","cycles":3}"#);
        let back: ConvergenceCriteria = serde_json::from_str(&json).unwrap();
        assert_eq!(back, criteria);
    }

    #[test]
    fn custom_criteria_carries_named_signals() {
        let criteria = ConvergenceCriteria::Custom {
            signals: vec![ConvergenceSignal::new(
                "confidence_delta",
                "stop when confidence delta is below threshold",
            )],
        };

        let json = serde_json::to_value(&criteria).unwrap();

        assert_eq!(json["kind"], "custom");
        assert_eq!(json["signals"][0]["name"], "confidence_delta");
    }
}
