//! Prism-backed fuzzy inference as a normal Converge Suggestor.
//!
//! This adapter keeps the work loop in Organism and the fuzzy math in Prism.
//! Apps provide domain variables, input extraction, proposal ids, and payload
//! projection; the adapter handles deterministic in-loop execution.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use converge_pack::gate::{ObjectiveSpec, ProblemSpec};
use converge_pack::{
    AgentEffect, Context, ContextKey, FactPayload, ProposedFact, Provenance, ProvenanceSource,
    Suggestor, TextPayload,
};
use prism::fuzzy::{
    ActivatedRule, FuzzyInferenceEngine, FuzzyInferenceInput, FuzzyInferenceOutput, FuzzyRule,
    LinguisticVariable,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::provenance::ORGANISM_PLANNING_PROVENANCE;

type InputExtractor =
    Arc<dyn Fn(&dyn Context) -> Result<BTreeMap<String, f64>, FuzzySuggestorError> + Send + Sync>;
type PayloadBuilder<P> =
    Arc<dyn Fn(&FuzzyInferenceTrace) -> Result<P, FuzzySuggestorError> + Send + Sync>;
type ProposalIdBuilder = Arc<dyn Fn(&FuzzyInferenceTrace) -> String + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzyInferenceTrace {
    pub inputs: BTreeMap<String, f64>,
    pub input_memberships: BTreeMap<String, BTreeMap<String, f64>>,
    pub memberships: BTreeMap<String, f64>,
    pub activated_rules: Vec<FuzzyRuleActivationTrace>,
    pub confidence: f64,
    pub total_rules: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzyRuleActivationTrace {
    pub id: String,
    pub antecedent_strength: f64,
    pub weight: f64,
    pub strength: f64,
    pub consequent: String,
}

pub struct FuzzyInferenceSuggestor<P>
where
    P: FactPayload + PartialEq,
{
    name: String,
    provenance: Provenance,
    dependencies: Vec<ContextKey>,
    output_key: ContextKey,
    diagnostic_key: ContextKey,
    proposal_id_prefix: String,
    variables: Vec<LinguisticVariable>,
    rules: Vec<FuzzyRule>,
    input_extractor: InputExtractor,
    payload_builder: PayloadBuilder<P>,
    proposal_id_builder: ProposalIdBuilder,
}

impl<P> FuzzyInferenceSuggestor<P>
where
    P: FactPayload + PartialEq + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new<I, B, Id>(
        name: impl Into<String>,
        variables: Vec<LinguisticVariable>,
        rules: Vec<FuzzyRule>,
        input_extractor: I,
        payload_builder: B,
        proposal_id_builder: Id,
    ) -> Self
    where
        I: Fn(&dyn Context) -> Result<BTreeMap<String, f64>, FuzzySuggestorError>
            + Send
            + Sync
            + 'static,
        B: Fn(&FuzzyInferenceTrace) -> Result<P, FuzzySuggestorError> + Send + Sync + 'static,
        Id: Fn(&FuzzyInferenceTrace) -> String + Send + Sync + 'static,
    {
        let name = name.into();
        Self {
            proposal_id_prefix: format!("organism.fuzzy.{name}"),
            name,
            provenance: ORGANISM_PLANNING_PROVENANCE.provenance(),
            dependencies: vec![ContextKey::Signals],
            output_key: ContextKey::Evaluations,
            diagnostic_key: ContextKey::Diagnostic,
            variables,
            rules,
            input_extractor: Arc::new(input_extractor),
            payload_builder: Arc::new(payload_builder),
            proposal_id_builder: Arc::new(proposal_id_builder),
        }
    }

    #[must_use]
    pub fn with_dependencies(mut self, dependencies: Vec<ContextKey>) -> Self {
        self.dependencies = dependencies;
        self
    }

    #[must_use]
    pub fn with_output_key(mut self, output_key: ContextKey) -> Self {
        self.output_key = output_key;
        self
    }

    #[must_use]
    pub fn with_diagnostic_key(mut self, diagnostic_key: ContextKey) -> Self {
        self.diagnostic_key = diagnostic_key;
        self
    }

    #[must_use]
    pub fn with_provenance(mut self, provenance: impl Into<Provenance>) -> Self {
        self.provenance = provenance.into();
        self
    }

    #[must_use]
    pub fn with_proposal_id_prefix(mut self, proposal_id_prefix: impl Into<String>) -> Self {
        self.proposal_id_prefix = proposal_id_prefix.into();
        self
    }

    pub fn infer_trace(
        &self,
        inputs: BTreeMap<String, f64>,
    ) -> Result<FuzzyInferenceTrace, FuzzySuggestorError> {
        if self.rules.is_empty() {
            return self.membership_only_trace(inputs);
        }
        let spec = self.problem_spec()?;
        let input = FuzzyInferenceInput {
            inputs: inputs.clone(),
            variables: self.variables.clone(),
            rules: self.rules.clone(),
        };
        let (output, _report) = FuzzyInferenceEngine
            .solve(&input, &spec)
            .map_err(|error| FuzzySuggestorError::InferenceFailed(error.to_string()))?;
        Ok(FuzzyInferenceTrace::from_output(inputs, output))
    }

    fn membership_only_trace(
        &self,
        inputs: BTreeMap<String, f64>,
    ) -> Result<FuzzyInferenceTrace, FuzzySuggestorError> {
        let mut input_memberships = BTreeMap::new();
        let mut confidence = 0.0_f64;
        for variable in &self.variables {
            let Some(value) = inputs.get(&variable.name).copied() else {
                continue;
            };
            let mut memberships = BTreeMap::new();
            for set in &variable.sets {
                set.function
                    .validate()
                    .map_err(|error| FuzzySuggestorError::InferenceFailed(error.to_string()))?;
                let degree = set.function.evaluate(value).value();
                confidence = confidence.max(degree);
                memberships.insert(set.name.clone(), degree);
            }
            input_memberships.insert(variable.name.clone(), memberships);
        }

        Ok(FuzzyInferenceTrace {
            inputs,
            input_memberships,
            memberships: BTreeMap::new(),
            activated_rules: Vec::new(),
            confidence,
            total_rules: 0,
        })
    }

    fn problem_spec(&self) -> Result<ProblemSpec, FuzzySuggestorError> {
        ProblemSpec::builder(
            format!("fuzzy-suggestor:{}", self.name),
            "fuzzy-inference-suggestor",
        )
        .objective(ObjectiveSpec::maximize("activation_strength"))
        .build()
        .map_err(|error| FuzzySuggestorError::InferenceFailed(error.to_string()))
    }

    fn diagnostic(&self, error: &FuzzySuggestorError) -> AgentEffect {
        let id_prefix = self.proposal_id_prefix.trim_end_matches('.');
        AgentEffect::with_proposal(ProposedFact::new(
            self.diagnostic_key,
            format!("{id_prefix}.diagnostic"),
            TextPayload::new(error.to_string()),
            self.provenance.clone(),
        ))
    }
}

impl FuzzyInferenceTrace {
    fn from_output(inputs: BTreeMap<String, f64>, output: FuzzyInferenceOutput) -> Self {
        Self {
            inputs,
            input_memberships: output
                .input_memberships
                .into_iter()
                .map(|(variable, memberships)| {
                    (
                        variable,
                        memberships
                            .into_iter()
                            .map(|(set, degree)| (set, degree.value()))
                            .collect(),
                    )
                })
                .collect(),
            memberships: output
                .memberships
                .into_iter()
                .map(|(consequent, degree)| (consequent, degree.value()))
                .collect(),
            activated_rules: output
                .activated_rules
                .into_iter()
                .map(FuzzyRuleActivationTrace::from)
                .collect(),
            confidence: output.confidence.value(),
            total_rules: output.total_rules,
        }
    }
}

impl From<ActivatedRule> for FuzzyRuleActivationTrace {
    fn from(rule: ActivatedRule) -> Self {
        Self {
            id: rule.id,
            antecedent_strength: rule.antecedent_strength.value(),
            weight: rule.weight.value(),
            strength: rule.strength.value(),
            consequent: rule.consequent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FuzzySuggestorError {
    #[error("fuzzy input extraction failed: {0}")]
    InputExtractionFailed(String),
    #[error("fuzzy inference failed: {0}")]
    InferenceFailed(String),
    #[error("fuzzy payload projection failed: {0}")]
    PayloadProjectionFailed(String),
}

impl FuzzySuggestorError {
    #[must_use]
    pub fn input(message: impl fmt::Display) -> Self {
        Self::InputExtractionFailed(message.to_string())
    }

    #[must_use]
    pub fn payload(message: impl fmt::Display) -> Self {
        Self::PayloadProjectionFailed(message.to_string())
    }
}

#[async_trait::async_trait]
impl<P> Suggestor for FuzzyInferenceSuggestor<P>
where
    P: FactPayload + PartialEq + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &self.dependencies
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        self.dependencies.iter().all(|key| ctx.has(*key))
            && !ctx
                .get(self.output_key)
                .iter()
                .any(|fact| fact.id().as_str().starts_with(&self.proposal_id_prefix))
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let inputs = match (self.input_extractor)(ctx) {
            Ok(inputs) => inputs,
            Err(error) => return self.diagnostic(&error),
        };
        let trace = match self.infer_trace(inputs) {
            Ok(trace) => trace,
            Err(error) => return self.diagnostic(&error),
        };
        let payload = match (self.payload_builder)(&trace) {
            Ok(payload) => payload,
            Err(error) => return self.diagnostic(&error),
        };
        let proposal_id = (self.proposal_id_builder)(&trace);

        AgentEffect::with_proposal(ProposedFact::new(
            self.output_key,
            proposal_id,
            payload,
            self.provenance.clone(),
        ))
    }

    fn provenance(&self) -> Provenance {
        self.provenance.clone()
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(v*s + r) - v = fuzzy variables, s = sets per variable, r = rules")
    }
}

#[cfg(test)]
mod tests {
    use converge_kernel::{ContextState, Engine};
    use prism::fuzzy::{FuzzyConsequent, FuzzyExpression, FuzzyRule, FuzzySet, MembershipFunction};

    use super::*;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestFuzzyPayload {
        trace: FuzzyInferenceTrace,
    }

    impl FactPayload for TestFuzzyPayload {
        const FAMILY: &'static str = "organism.test.fuzzy";
        const VERSION: u16 = 1;
    }

    fn risk_rulebook() -> (Vec<LinguisticVariable>, Vec<FuzzyRule>) {
        let variables = vec![
            LinguisticVariable {
                name: "risk".to_string(),
                sets: vec![FuzzySet {
                    name: "high".to_string(),
                    function: MembershipFunction::RightShoulder {
                        start: 0.4,
                        end: 0.8,
                    },
                }],
            },
            LinguisticVariable {
                name: "review_urgency".to_string(),
                sets: vec![FuzzySet {
                    name: "advisable".to_string(),
                    function: MembershipFunction::RightShoulder {
                        start: 0.3,
                        end: 0.7,
                    },
                }],
            },
        ];
        let rules = vec![FuzzyRule {
            id: Some("high-risk-review".to_string()),
            when: FuzzyExpression::Is {
                variable: "risk".to_string(),
                set: "high".to_string(),
            },
            then: FuzzyConsequent {
                variable: "review_urgency".to_string(),
                set: "advisable".to_string(),
            },
            weight: None,
        }];
        (variables, rules)
    }

    fn risk_suggestor() -> FuzzyInferenceSuggestor<TestFuzzyPayload> {
        let (variables, rules) = risk_rulebook();
        FuzzyInferenceSuggestor::new(
            "risk-review",
            variables,
            rules,
            |ctx| {
                let value = ctx
                    .get(ContextKey::Signals)
                    .iter()
                    .filter_map(|fact| fact.text())
                    .find_map(|text| text.strip_prefix("risk="))
                    .ok_or_else(|| FuzzySuggestorError::input("missing risk signal"))?;
                let risk = value.parse::<f64>().map_err(FuzzySuggestorError::input)?;
                Ok(BTreeMap::from([("risk".to_string(), risk)]))
            },
            |trace| {
                Ok(TestFuzzyPayload {
                    trace: trace.clone(),
                })
            },
            |_| "test.fuzzy.risk-review".to_string(),
        )
        .with_proposal_id_prefix("test.fuzzy.")
    }

    #[tokio::test]
    async fn fuzzy_inference_suggestor_runs_inside_converge_engine() {
        let mut engine = Engine::default();
        engine.register_suggestor(risk_suggestor());
        let mut ctx = ContextState::default();
        ctx.add_input_with_provenance(ContextKey::Signals, "risk-signal", "risk=0.7", "test")
            .expect("input should stage");

        let result = engine.run(ctx).await.expect("engine should run");

        assert!(result.converged);
        let evaluations = result.context.get(ContextKey::Evaluations);
        let fact = evaluations
            .iter()
            .find(|fact| fact.id().as_str() == "test.fuzzy.risk-review")
            .expect("fuzzy proposal promoted");
        let payload = fact
            .require_payload::<TestFuzzyPayload>()
            .expect("typed fuzzy payload");
        assert_eq!(payload.trace.total_rules, 1);
        assert_eq!(payload.trace.activated_rules[0].id, "high-risk-review");
        assert!((payload.trace.input_memberships["risk"]["high"] - 0.75).abs() < 1e-9);
    }

    #[tokio::test]
    async fn fuzzy_inference_suggestor_emits_diagnostic_when_input_is_missing() {
        let mut engine = Engine::default();
        engine.register_suggestor(risk_suggestor());
        let mut ctx = ContextState::default();
        ctx.add_input_with_provenance(ContextKey::Signals, "other-signal", "other=0.7", "test")
            .expect("input should stage");

        let result = engine.run(ctx).await.expect("engine should run");

        assert!(result.converged);
        assert!(result.context.get(ContextKey::Evaluations).is_empty());
        assert!(
            result
                .context
                .get(ContextKey::Diagnostic)
                .iter()
                .any(|fact| fact
                    .text()
                    .is_some_and(|text| text.contains("missing risk")))
        );
    }
}
