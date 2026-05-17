//! Graded admission control via fuzzy inference.
//!
//! `DefaultAdmissionController` does cheap, hard-edged checks
//! (missing-capability, expired, irreversible-without-authority). This
//! controller is for the *uncertain middle* — when feasibility is a matter
//! of degree on signals like "how much capacity is currently free", "how
//! aligned is this with prior intent", "how confident are we in the
//! authority scope". One fuzzy rulebook per `FeasibilityDimension`; the
//! kind with the highest membership wins.
//!
//! Caller convention: each rulebook produces memberships in a single
//! output linguistic variable whose set names match `FeasibilityKind`
//! variants (snake_case): `feasible`, `feasible_with_constraints`,
//! `uncertain`, `infeasible`. Sets that don't match these names are
//! ignored. If no rules fire or no matching sets are present, the
//! dimension defaults to `Uncertain`.

use std::collections::BTreeMap;

use converge_pack::gate::{ObjectiveSpec, ProblemSpec};
use prism::fuzzy::{
    FuzzyInferenceEngine, FuzzyInferenceInput, FuzzyInferenceOutput, FuzzyRule, LinguisticVariable,
};

use crate::{
    AdmissionController, AdmissionResult, FeasibilityAssessment, FeasibilityDimension,
    FeasibilityKind, IntentPacket,
};

/// One fuzzy rulebook scoped to a single `FeasibilityDimension`.
pub struct DimensionRulebook {
    pub dimension: FeasibilityDimension,
    pub variables: Vec<LinguisticVariable>,
    pub rules: Vec<FuzzyRule>,
    /// Output linguistic variable whose set memberships drive the
    /// `FeasibilityKind` decision.
    pub output_variable: String,
}

impl DimensionRulebook {
    pub fn new(
        dimension: FeasibilityDimension,
        output_variable: impl Into<String>,
        variables: Vec<LinguisticVariable>,
        rules: Vec<FuzzyRule>,
    ) -> Self {
        Self {
            dimension,
            variables,
            rules,
            output_variable: output_variable.into(),
        }
    }
}

pub struct GradedAdmissionController {
    rulebooks: Vec<DimensionRulebook>,
}

impl GradedAdmissionController {
    pub fn new(rulebooks: Vec<DimensionRulebook>) -> Self {
        Self { rulebooks }
    }
}

impl AdmissionController for GradedAdmissionController {
    fn evaluate(&self, intent: &IntentPacket) -> AdmissionResult {
        let dimensions = self
            .rulebooks
            .iter()
            .map(|rb| evaluate_dimension(intent, rb))
            .collect();
        AdmissionResult::from_dimensions(dimensions)
    }
}

fn evaluate_dimension(
    intent: &IntentPacket,
    rulebook: &DimensionRulebook,
) -> FeasibilityAssessment {
    let inputs = extract_inputs(&intent.context, &rulebook.variables);
    let input = FuzzyInferenceInput {
        inputs,
        variables: rulebook.variables.clone(),
        rules: rulebook.rules.clone(),
    };
    let spec = match build_spec(rulebook.dimension) {
        Ok(s) => s,
        Err(e) => {
            return FeasibilityAssessment::uncertain(
                rulebook.dimension,
                format!("could not build problem spec: {e}"),
            );
        }
    };
    match FuzzyInferenceEngine.solve(&input, &spec) {
        Ok((output, _)) => {
            assess_from_output(rulebook.dimension, &rulebook.output_variable, &output)
        }
        Err(e) => FeasibilityAssessment::uncertain(
            rulebook.dimension,
            format!("fuzzy inference error: {e}"),
        ),
    }
}

fn extract_inputs(
    context: &serde_json::Value,
    variables: &[LinguisticVariable],
) -> BTreeMap<String, f64> {
    let mut inputs = BTreeMap::new();
    if let Some(obj) = context.as_object() {
        for var in variables {
            if let Some(v) = obj.get(&var.name).and_then(serde_json::Value::as_f64)
                && v.is_finite()
            {
                inputs.insert(var.name.clone(), v);
            }
        }
    }
    inputs
}

fn build_spec(dim: FeasibilityDimension) -> Result<ProblemSpec, String> {
    let kind = match dim {
        FeasibilityDimension::Capability => "capability",
        FeasibilityDimension::Context => "context",
        FeasibilityDimension::Resources => "resources",
        FeasibilityDimension::Authority => "authority",
    };
    ProblemSpec::builder(format!("graded-admission:{kind}"), "fuzzy-admission")
        .objective(ObjectiveSpec::maximize("feasibility_match"))
        .build()
        .map_err(|e| format!("{e}"))
}

fn assess_from_output(
    dim: FeasibilityDimension,
    output_var: &str,
    output: &FuzzyInferenceOutput,
) -> FeasibilityAssessment {
    let prefix = format!("{output_var}.");
    let memberships: BTreeMap<&str, f64> = output
        .memberships
        .iter()
        .filter_map(|(k, v): (&String, &f64)| k.strip_prefix(&prefix).map(|name| (name, *v)))
        .collect();

    let kind_candidates: &[(&str, FeasibilityKind)] = &[
        ("feasible", FeasibilityKind::Feasible),
        (
            "feasible_with_constraints",
            FeasibilityKind::FeasibleWithConstraints,
        ),
        ("uncertain", FeasibilityKind::Uncertain),
        ("infeasible", FeasibilityKind::Infeasible),
    ];

    let chosen = kind_candidates
        .iter()
        .filter_map(|(name, k)| memberships.get(name).map(|m| (*k, *m)))
        .filter(|(_, m): &(FeasibilityKind, f64)| *m > 0.0)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let (kind, top_membership) = chosen.unwrap_or((FeasibilityKind::Uncertain, 0.0));

    let trace = output
        .activated_rules
        .iter()
        .map(|r| format!("{} fires {} at {:.3}", r.id, r.consequent, r.strength))
        .collect::<Vec<_>>()
        .join("; ");

    let reason = if output.activated_rules.is_empty() {
        format!("{dim:?}: no fuzzy rules fired")
    } else {
        format!("{dim:?}: {kind:?} (membership {top_membership:.3}); rules: {trace}")
    };

    FeasibilityAssessment {
        dimension: dim,
        kind,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use prism::fuzzy::{FuzzyConsequent, FuzzyExpression, FuzzySet, MembershipFunction};

    use super::*;

    fn capacity_rulebook() -> DimensionRulebook {
        // Input: free_capacity ∈ [0,1].
        // High free capacity (right shoulder 0.4..0.8) → resources.feasible.
        // Low  free capacity (left  shoulder 0.2..0.5) → resources.infeasible.
        let variables = vec![
            LinguisticVariable {
                name: "free_capacity".into(),
                sets: vec![
                    FuzzySet {
                        name: "low".into(),
                        function: MembershipFunction::LeftShoulder {
                            start: 0.2,
                            end: 0.5,
                        },
                    },
                    FuzzySet {
                        name: "high".into(),
                        function: MembershipFunction::RightShoulder {
                            start: 0.4,
                            end: 0.8,
                        },
                    },
                ],
            },
            LinguisticVariable {
                name: "resources".into(),
                sets: vec![
                    FuzzySet {
                        name: "feasible".into(),
                        function: MembershipFunction::RightShoulder {
                            start: 0.5,
                            end: 1.0,
                        },
                    },
                    FuzzySet {
                        name: "infeasible".into(),
                        function: MembershipFunction::RightShoulder {
                            start: 0.5,
                            end: 1.0,
                        },
                    },
                ],
            },
        ];

        let rules = vec![
            FuzzyRule {
                id: Some("plenty".into()),
                when: FuzzyExpression::Is {
                    variable: "free_capacity".into(),
                    set: "high".into(),
                },
                then: FuzzyConsequent {
                    variable: "resources".into(),
                    set: "feasible".into(),
                },
                weight: None,
            },
            FuzzyRule {
                id: Some("starved".into()),
                when: FuzzyExpression::Is {
                    variable: "free_capacity".into(),
                    set: "low".into(),
                },
                then: FuzzyConsequent {
                    variable: "resources".into(),
                    set: "infeasible".into(),
                },
                weight: None,
            },
        ];

        DimensionRulebook::new(
            FeasibilityDimension::Resources,
            "resources",
            variables,
            rules,
        )
    }

    #[test]
    fn graded_admission_marks_feasible_when_capacity_is_high() {
        // free_capacity = 0.9 → high membership 1.0; low membership 0.
        // Plenty fires at 1.0 → resources.feasible = 1.0.
        // Starved doesn't fire.
        let controller = GradedAdmissionController::new(vec![capacity_rulebook()]);
        let intent = IntentPacket::new("run heavy job", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "free_capacity": 0.9 }));
        let result = controller.evaluate(&intent);
        assert!(result.feasible);
        let assessment = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Resources)
            .unwrap();
        assert_eq!(assessment.kind, FeasibilityKind::Feasible);
        assert!(assessment.reason.contains("plenty"));
    }

    #[test]
    fn graded_admission_marks_infeasible_when_capacity_is_low() {
        // free_capacity = 0.1 → low membership 1.0; high membership 0.
        // Starved fires at 1.0 → resources.infeasible = 1.0.
        let controller = GradedAdmissionController::new(vec![capacity_rulebook()]);
        let intent = IntentPacket::new("run heavy job", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "free_capacity": 0.1 }));
        let result = controller.evaluate(&intent);
        assert!(!result.feasible);
        let assessment = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Resources)
            .unwrap();
        assert_eq!(assessment.kind, FeasibilityKind::Infeasible);
        assert!(assessment.reason.contains("starved"));
        assert!(result.rejection_reason.is_some());
    }

    #[test]
    fn graded_admission_uncertain_when_no_rules_fire() {
        // free_capacity is missing entirely → no inputs → no rules fire.
        let controller = GradedAdmissionController::new(vec![capacity_rulebook()]);
        let intent = IntentPacket::new("run heavy job", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "unrelated": 0.5 }));
        let result = controller.evaluate(&intent);
        let assessment = result
            .dimensions
            .iter()
            .find(|d| d.dimension == FeasibilityDimension::Resources)
            .unwrap();
        assert_eq!(assessment.kind, FeasibilityKind::Uncertain);
    }
}
