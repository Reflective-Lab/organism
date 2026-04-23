//! Formations — teams of heterogeneous agents assembled to solve a problem.
//!
//! A Formation is the unit of work that Organism hands to Converge.
//! It contains a team of Suggestors (which may be LLMs, optimizers,
//! policy gates, analytics, knowledge retrieval, schedulers, or any other
//! agent type) plus the seed Context they
//! operate on.

use converge_kernel::{
    AgentEffect, Budget, Context, ContextKey, ContextState, ConvergeResult, Engine,
    ExperienceEventObserver, Suggestor,
};
use converge_pack::ProposalId;
use std::sync::Arc;

/// Wrapper that implements `Suggestor` for a boxed trait object.
/// Needed because converge-pack does not provide a blanket impl.
struct BoxedAgent(Box<dyn Suggestor>);

#[async_trait::async_trait]
impl Suggestor for BoxedAgent {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn dependencies(&self) -> &[ContextKey] {
        self.0.dependencies()
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        self.0.accepts(ctx)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        self.0.execute(ctx).await
    }
}

/// A team of agents assembled by Organism to run in a Converge Engine.
///
/// Formations are hypotheses: "this team, with these seeds, will converge
/// on a good answer." Organism may run multiple formations concurrently
/// and pick the winner.
pub struct Formation {
    /// Human-readable label for logging and learning.
    pub label: String,
    /// The agents in this team, ready to register on an Engine.
    agents: Vec<Box<dyn Suggestor>>,
    /// Initial external inputs to stage before running.
    seeds: Vec<Seed>,
    /// Execution budget for this formation's run.
    pub budget: Budget,
}

/// A seed input to stage into the Context before the Engine runs.
pub struct Seed {
    pub key: ContextKey,
    pub id: ProposalId,
    pub content: String,
    pub provenance: String,
}

/// Result of running a Formation in a Converge Engine.
pub struct FormationResult {
    /// The label of the formation that produced this result.
    pub label: String,
    /// The governed Converge result.
    pub converge_result: ConvergeResult,
}

impl Formation {
    /// Create an empty formation with a human-readable label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            agents: Vec::new(),
            seeds: Vec::new(),
            budget: Budget::default(),
        }
    }

    /// Add a heterogeneous agent to the team.
    pub fn agent(mut self, suggestor: impl Suggestor + 'static) -> Self {
        self.agents.push(Box::new(suggestor));
        self
    }

    /// Add a boxed agent to the team.
    pub fn agent_boxed(mut self, suggestor: Box<dyn Suggestor>) -> Self {
        self.agents.push(suggestor);
        self
    }

    /// Stage an initial input with explicit provenance.
    pub fn seed(
        mut self,
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: impl Into<String>,
        provenance: impl Into<String>,
    ) -> Self {
        self.seeds.push(Seed {
            key,
            id: id.into(),
            content: content.into(),
            provenance: provenance.into(),
        });
        self
    }

    /// Set the execution budget.
    pub fn with_budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// Run this formation in a fresh Converge Engine.
    ///
    /// This is the honest execution boundary: Organism assembles the team,
    /// Converge runs it. Agents propose, the engine promotes, and the
    /// returned result is governed by Converge.
    pub async fn run(self) -> Result<FormationResult, FormationError> {
        self.run_observed(None).await
    }

    /// Run this formation with a run-scoped experience observer.
    ///
    /// Organism should use this with `FormationExperienceObserver` when it needs
    /// tenant/correlation metadata on Converge experience envelopes.
    pub async fn run_with_event_observer(
        self,
        observer: Arc<dyn ExperienceEventObserver>,
    ) -> Result<FormationResult, FormationError> {
        self.run_observed(Some(observer)).await
    }

    async fn run_observed(
        self,
        observer: Option<Arc<dyn ExperienceEventObserver>>,
    ) -> Result<FormationResult, FormationError> {
        let mut engine = Engine::with_budget(self.budget);
        if let Some(observer) = observer {
            engine.set_event_observer(observer);
        }

        // Register all agents
        for agent in self.agents {
            engine.register_suggestor(BoxedAgent(agent));
        }

        // Build seed context through the public input path.
        let mut context = ContextState::new();
        for seed in &self.seeds {
            context
                .add_input_with_provenance(
                    seed.key,
                    seed.id.clone(),
                    &seed.content,
                    &seed.provenance,
                )
                .map_err(|e| FormationError::ConvergenceFailed(e.to_string()))?;
        }

        // Run convergence
        let converge_result = engine
            .run(context)
            .await
            .map_err(|e| FormationError::ConvergenceFailed(e.to_string()))?;

        Ok(FormationResult {
            label: self.label,
            converge_result,
        })
    }
}

/// Builder helpers for standard organism agent teams.
impl Formation {
    /// Add the standard simulation swarm (all 5 dimensions) with default configs.
    pub fn with_simulation_swarm(self) -> Self {
        use organism_simulation::{
            CausalSimulationAgent, CostSimulationAgent, OperationalSimulationAgent,
            OutcomeSimulationAgent, PolicySimulationAgent,
        };

        self.agent(OutcomeSimulationAgent::default_config())
            .agent(CostSimulationAgent::default_config())
            .agent(PolicySimulationAgent::default_config())
            .agent(CausalSimulationAgent::default_config())
            .agent(OperationalSimulationAgent::default_config())
    }

    /// Add the standard adversarial team with default configs.
    pub fn with_adversarial_team(self) -> Self {
        use organism_adversarial::{
            AssumptionBreakerAgent, ConstraintCheckerAgent, EconomicSkepticAgent,
            OperationalSkepticAgent,
        };

        self.agent(AssumptionBreakerAgent::new())
            .agent(ConstraintCheckerAgent::default_config())
            .agent(EconomicSkepticAgent::default_config())
            .agent(OperationalSkepticAgent::default_config())
    }

    /// Add the planning prior agent for learning feedback.
    pub fn with_learning_priors(self) -> Self {
        use organism_learning::PlanningPriorAgent;

        self.agent(PlanningPriorAgent::new())
    }

    /// Full Stage 2 pipeline: priors → adversarial → simulation.
    pub fn with_stress_test_pipeline(self) -> Self {
        self.with_learning_priors()
            .with_adversarial_team()
            .with_simulation_swarm()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FormationError {
    #[error("convergence failed: {0}")]
    ConvergenceFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SEED_DEPENDENCIES: &[ContextKey] = &[ContextKey::Seeds];

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("runtime")
    }

    struct SeedObserver;

    #[async_trait::async_trait]
    impl Suggestor for SeedObserver {
        fn name(&self) -> &'static str {
            "seed-observer"
        }

        fn dependencies(&self) -> &[ContextKey] {
            SEED_DEPENDENCIES
        }

        fn accepts(&self, ctx: &dyn Context) -> bool {
            ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
        }

        async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
            let seed = &ctx.get(ContextKey::Seeds)[0];
            AgentEffect::with_proposal(converge_kernel::ProposedFact::new(
                ContextKey::Hypotheses,
                format!("observed-{}", seed.id),
                format!("observed {}", seed.content),
                self.name(),
            ))
        }
    }

    #[tokio::test]
    async fn formation_promotes_valid_seed_before_agent_loop() {
        let result = Formation::new("valid-seed")
            .agent(SeedObserver)
            .seed(
                ContextKey::Seeds,
                "seed-1",
                "seed content",
                "external-request",
            )
            .run()
            .await
            .expect("formation should converge");

        assert!(result.converge_result.converged);
        assert!(!result.converge_result.context.has_pending_proposals());

        let seeds = result.converge_result.context.get(ContextKey::Seeds);
        let hypotheses = result.converge_result.context.get(ContextKey::Hypotheses);

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].id, "seed-1");
        assert_eq!(seeds[0].content, "seed content");
        assert_eq!(hypotheses.len(), 1);
        assert_eq!(hypotheses[0].id, "observed-seed-1");
        assert_eq!(hypotheses[0].content, "observed seed content");
    }

    #[tokio::test]
    async fn formation_rejects_invalid_seed_before_agent_can_observe_it() {
        let result = Formation::new("invalid-seed")
            .agent(SeedObserver)
            .seed(ContextKey::Seeds, "seed-1", "   \t\n  ", "external-request")
            .run()
            .await
            .expect("formation should converge");

        assert!(result.converge_result.converged);
        assert!(!result.converge_result.context.has(ContextKey::Seeds));
        assert!(!result.converge_result.context.has(ContextKey::Hypotheses));
        assert!(!result.converge_result.context.has_pending_proposals());
    }

    #[test]
    fn formation_rejects_conflicting_seed_ids_before_engine_run() {
        let result = rt().block_on(
            Formation::new("conflict")
                .seed(ContextKey::Seeds, "seed-1", "version A", "user")
                .seed(ContextKey::Seeds, "seed-1", "version B", "user")
                .run(),
        );

        match result {
            Err(FormationError::ConvergenceFailed(message)) => {
                assert!(message.contains("conflict detected for fact 'seed-1'"));
            }
            Ok(_) => panic!("conflicting seeds must fail"),
        }
    }

    proptest! {
        #[test]
        fn formation_roundtrips_valid_seed_inputs(
            id in "[a-z0-9][a-z0-9-]{0,15}",
            content in "[A-Za-z0-9][A-Za-z0-9 _-]{0,31}",
            provenance in "[a-z][a-z0-9-]{2,15}",
        ) {
            let result = rt()
                .block_on(
                    Formation::new("prop-valid")
                        .agent(SeedObserver)
                        .seed(ContextKey::Seeds, id.clone(), content.clone(), provenance)
                        .run(),
                )
                .expect("formation should converge");

            let seeds = result.converge_result.context.get(ContextKey::Seeds);
            let hypotheses = result.converge_result.context.get(ContextKey::Hypotheses);

            prop_assert_eq!(seeds.len(), 1);
            prop_assert_eq!(&seeds[0].id, &id);
            prop_assert_eq!(&seeds[0].content, &content);
            prop_assert_eq!(hypotheses.len(), 1);
            prop_assert_eq!(&hypotheses[0].content, &format!("observed {content}"));
            prop_assert!(!result.converge_result.context.has_pending_proposals());
        }

        #[test]
        fn formation_never_promotes_whitespace_only_seed_content(
            id in "[a-z0-9][a-z0-9-]{0,15}",
            content in "[ \\t\\n]{1,12}",
            provenance in "[a-z][a-z0-9-]{2,15}",
        ) {
            let result = rt()
                .block_on(
                    Formation::new("prop-invalid")
                        .agent(SeedObserver)
                        .seed(ContextKey::Seeds, id, content, provenance)
                        .run(),
                )
                .expect("formation should converge");

            prop_assert!(!result.converge_result.context.has(ContextKey::Seeds));
            prop_assert!(!result.converge_result.context.has(ContextKey::Hypotheses));
            prop_assert!(!result.converge_result.context.has_pending_proposals());
        }
    }
}
