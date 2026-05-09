//! Fuzzy reasoner — graded reasoning over linguistic variables.
//!
//! Wraps `prism::fuzzy::FuzzyInferenceEngine` (Mamdani) so a fuzzy rulebook
//! can participate in a huddle alongside LLM, constraint, ML, and causal
//! reasoners. Output memberships and activated rules become structured
//! `Impact` annotations on the produced `Plan`; the activated-rule trace is
//! preserved in the rationale string so a reviewer can see which rules drove
//! the proposal.

use std::collections::BTreeMap;

use anyhow::Result;
use async_trait::async_trait;
use converge_pack::gate::{ObjectiveSpec, ProblemSpec};
use organism_intent::IntentPacket;
use prism::fuzzy::{FuzzyInferenceEngine, FuzzyInferenceInput, FuzzyRule, LinguisticVariable};

use crate::{Impact, Plan, PlanContribution, Reasoner, ReasoningSystem};

/// A fuzzy reasoner participating in planning huddles.
///
/// Inputs are extracted from `IntentPacket::context` by variable name —
/// the context is expected to be a JSON object with one numeric field per
/// linguistic variable. Variables without a matching field are skipped;
/// a rule whose antecedent references a missing variable will not fire.
pub struct FuzzyReasoner {
    name: String,
    variables: Vec<LinguisticVariable>,
    rules: Vec<FuzzyRule>,
}

impl FuzzyReasoner {
    pub fn new(
        name: impl Into<String>,
        variables: Vec<LinguisticVariable>,
        rules: Vec<FuzzyRule>,
    ) -> Self {
        Self {
            name: name.into(),
            variables,
            rules,
        }
    }

    fn extract_inputs(&self, context: &serde_json::Value) -> BTreeMap<String, f64> {
        let mut inputs = BTreeMap::new();
        if let Some(obj) = context.as_object() {
            for var in &self.variables {
                if let Some(value) = obj.get(&var.name).and_then(serde_json::Value::as_f64)
                    && value.is_finite()
                {
                    inputs.insert(var.name.clone(), value);
                }
            }
        }
        inputs
    }

    fn problem_spec(&self) -> Result<ProblemSpec> {
        ProblemSpec::builder(format!("fuzzy-reasoner:{}", self.name), "fuzzy-reasoning")
            .objective(ObjectiveSpec::maximize("activation_strength"))
            .build()
            .map_err(|e| anyhow::anyhow!("fuzzy reasoner could not build ProblemSpec: {e}"))
    }

    fn build_input(&self, context: &serde_json::Value) -> FuzzyInferenceInput {
        FuzzyInferenceInput {
            inputs: self.extract_inputs(context),
            variables: self.variables.clone(),
            rules: self.rules.clone(),
        }
    }
}

#[async_trait]
impl Reasoner for FuzzyReasoner {
    fn name(&self) -> &str {
        &self.name
    }

    fn system_type(&self) -> ReasoningSystem {
        ReasoningSystem::FuzzyReasoning
    }

    async fn propose(&self, intent: &IntentPacket) -> Result<Plan> {
        let input = self.build_input(&intent.context);
        let spec = self.problem_spec()?;

        let (output, _report) = FuzzyInferenceEngine
            .solve(&input, &spec)
            .map_err(|e| anyhow::anyhow!("fuzzy inference failed: {e}"))?;

        let impacts: Vec<Impact> = output
            .memberships
            .iter()
            .filter(|(_, strength)| **strength > 0.0)
            .map(|(consequent, strength)| Impact {
                description: format!("output {consequent}"),
                confidence: *strength,
            })
            .collect();

        let rationale = if output.activated_rules.is_empty() {
            format!(
                "fuzzy: no rules fired against intent context (evaluated {} total)",
                output.total_rules
            )
        } else {
            let trace = output
                .activated_rules
                .iter()
                .map(|r| format!("{} fires {} at {:.3}", r.id, r.consequent, r.strength))
                .collect::<Vec<_>>()
                .join("; ");
            format!("fuzzy: {trace}")
        };

        let mut plan = Plan::new(intent, rationale);
        plan.annotation.impacts = impacts;
        plan.contributor = ReasoningSystem::FuzzyReasoning;
        Ok(plan)
    }

    fn contribute(&self, context: &serde_json::Value) -> PlanContribution {
        let input = self.build_input(context);
        let suggestions = match self.problem_spec() {
            Ok(spec) => match FuzzyInferenceEngine.solve(&input, &spec) {
                Ok((output, _)) => output
                    .activated_rules
                    .into_iter()
                    .map(|r| format!("{} fires {} at {:.3}", r.id, r.consequent, r.strength))
                    .collect(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        };

        PlanContribution {
            system: ReasoningSystem::FuzzyReasoning,
            suggestions,
            constraints: vec![],
            risks: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use prism::fuzzy::{FuzzyConsequent, FuzzyExpression, FuzzyRule, FuzzySet, MembershipFunction};

    use super::*;

    fn rulebook() -> (Vec<LinguisticVariable>, Vec<FuzzyRule>) {
        // trust ∈ [0,1]: low/high shoulders.
        // urgency ∈ [0,1]: low/high shoulders.
        // priority output: low/high (linguistic), shouldered.
        // Rule: if trust is high and urgency is high → priority is high.
        let variables = vec![
            LinguisticVariable {
                name: "trust".into(),
                sets: vec![
                    FuzzySet {
                        name: "low".into(),
                        function: MembershipFunction::LeftShoulder {
                            start: 0.3,
                            end: 0.6,
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
                name: "urgency".into(),
                sets: vec![FuzzySet {
                    name: "high".into(),
                    function: MembershipFunction::RightShoulder {
                        start: 0.4,
                        end: 0.8,
                    },
                }],
            },
            LinguisticVariable {
                name: "priority".into(),
                sets: vec![FuzzySet {
                    name: "high".into(),
                    function: MembershipFunction::RightShoulder {
                        start: 0.5,
                        end: 1.0,
                    },
                }],
            },
        ];

        let rules = vec![FuzzyRule {
            id: Some("trust-urgency-priority".into()),
            when: FuzzyExpression::And {
                terms: vec![
                    FuzzyExpression::Is {
                        variable: "trust".into(),
                        set: "high".into(),
                    },
                    FuzzyExpression::Is {
                        variable: "urgency".into(),
                        set: "high".into(),
                    },
                ],
            },
            then: FuzzyConsequent {
                variable: "priority".into(),
                set: "high".into(),
            },
            weight: None,
        }];

        (variables, rules)
    }

    #[tokio::test]
    async fn fuzzy_reasoner_proposes_plan_with_graded_impact() {
        // trust = 0.7 (high membership = (0.7-0.4)/(0.8-0.4) = 0.75)
        // urgency = 0.8 (high membership = clamp((0.8-0.4)/(0.8-0.4), 0, 1) = 1.0)
        // antecedent = min(0.75, 1.0) = 0.75
        // priority.high fires at 0.75
        let (variables, rules) = rulebook();
        let reasoner = FuzzyReasoner::new("urgency-priority", variables, rules);

        let intent = IntentPacket::new("decide priority", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "trust": 0.7, "urgency": 0.8 }));

        let plan = reasoner.propose(&intent).await.unwrap();
        assert_eq!(plan.contributor, ReasoningSystem::FuzzyReasoning);
        assert_eq!(plan.annotation.impacts.len(), 1);
        let impact = &plan.annotation.impacts[0];
        assert!(impact.description.contains("priority.high"));
        assert!(
            (impact.confidence - 0.75).abs() < 1e-9,
            "priority.high confidence should be 0.75, got {}",
            impact.confidence
        );
        assert!(plan.rationale.contains("trust-urgency-priority"));
    }

    #[tokio::test]
    async fn fuzzy_reasoner_no_rules_fired_produces_empty_impacts() {
        // trust = 0.1 (low), urgency = 0.1 (no match for high)
        // antecedent strength = 0 → no firings
        let (variables, rules) = rulebook();
        let reasoner = FuzzyReasoner::new("low-trust", variables, rules);

        let intent = IntentPacket::new("decide priority", Utc::now() + chrono::Duration::hours(1))
            .with_context(serde_json::json!({ "trust": 0.1, "urgency": 0.1 }));

        let plan = reasoner.propose(&intent).await.unwrap();
        assert!(plan.annotation.impacts.is_empty());
        assert!(plan.rationale.contains("no rules fired"));
    }

    #[test]
    fn contribute_returns_activated_rule_summaries() {
        let (variables, rules) = rulebook();
        let reasoner = FuzzyReasoner::new("contrib-test", variables, rules);
        let context = serde_json::json!({ "trust": 0.7, "urgency": 0.8 });
        let contribution = reasoner.contribute(&context);
        assert_eq!(contribution.system, ReasoningSystem::FuzzyReasoning);
        assert_eq!(contribution.suggestions.len(), 1);
        assert!(contribution.suggestions[0].contains("trust-urgency-priority"));
    }
}
