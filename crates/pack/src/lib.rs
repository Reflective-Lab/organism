//! # Organism Pack
//!
//! The public contract for Organism's planning loop.
//! One import — the full pipeline from intent to learning.
//!
//! ```text
//! IntentPacket → Admission → Plan → Challenge → Simulate → Learn → Commit
//! ```
//!
//! # Quick start
//!
//! ```rust,ignore
//! use organism_pack::*;
//!
//! // 1. Create an intent
//! let intent = IntentPacket::new("Approve $2,500 expense", expires);
//!
//! // 2. Check admission (4 dimensions)
//! let admission = my_controller.evaluate(&intent);
//!
//! // 3. Plan (multi-model huddle)
//! let plan = Plan::new(&intent, "route to 3 approvers");
//!
//! // 4. Challenge (5 skepticism kinds)
//! let challenge = Challenge::new(
//!     SkepticismKind::EconomicSkepticism,
//!     plan.id,
//!     "entertainment spend is high",
//!     Severity::Warning,
//! );
//!
//! // 5. Simulate (5 dimensions)
//! let result = DimensionResult {
//!     dimension: SimulationDimension::Cost,
//!     passed: true,
//!     confidence: 0.95,
//!     findings: vec!["within budget".into()],
//!     samples: vec![],
//! };
//!
//! // 6. Learn from outcomes
//! let lesson = Lesson {
//!     insight: "score 0.88 → approved".into(),
//!     context: "expense approval".into(),
//!     confidence: 0.9,
//!     planning_adjustment: "none".into(),
//! };
//! ```

// ── Intent ─────────────────────────────────────────────────────────
// The input: what the organization wants to achieve.

pub use organism_intent::{
    // Admission
    AdmissionController,
    AdmissionResult,
    ExpiryAction,
    FeasibilityAssessment,
    FeasibilityDimension,
    FeasibilityKind,
    ForbiddenAction,
    IntentError,
    IntentNode,
    IntentPacket,
    Reversibility,
    admission::DefaultAdmissionController,
    // Resolution
    resolution::{
        CapabilityRequirement, DeclarativeBinding, IntentBinding, IntentResolver, PackRequirement,
        ResolutionLevel, ResolutionTrace,
    },
};

// ── Planning ───────────────────────────────────────────────────────
// How the system reasons about the intent.

pub use organism_planning::{
    CollaborationCharter, CollaborationDiscipline, CollaborationMember, CollaborationRole,
    CollaborationTopology, CollaborationValidationError, ConsensusRule, CostEstimate, Impact,
    Likelihood, Plan, PlanAnnotation, PlanBundle, PlanContribution, PlanStep, Reasoner,
    ReasoningSystem, Risk, RiskImpact, TeamFormation, TeamFormationMode, TurnCadence,
    charter_derivation::{
        DerivationRationale, DerivedCharter, IntentComplexity, derive_charter,
        derive_charter_with_priors,
    },
    dd::{
        BreadthResearchSuggestor, ContradictionFinderSuggestor, DdError, DdFactSummary, DdHooks,
        DdLlm, DdSearch, DepthResearchSuggestor, FactExtractorSuggestor, FailoverDdLlm,
        FailoverDdSearch, GapDetectorSuggestor, HookPatterns, SearchHit, SynthesisSuggestor,
        consolidate_dd_hypotheses, extract_hooks_from_facts,
    },
    kb::{
        HubCategory, KbConfig, RootPageDef, sanitize_filename, slugify, update_root_pages,
        write_dd_to_vault, write_or_append_hub,
    },
    shape_hypothesis::{
        ShapeCalibration, ShapeCandidate, ShapeCompetition, ShapeMetric, ShapeObservation,
        calibrate_shape, classify_problem, generate_candidates, score_observation, select_winner,
    },
    suggestor::{HuddleSeedSuggestor, NamedPlan, SharedBudget},
    topology_transition::{
        CharterAdjustments, ConvergenceSignals, TransitionDecision, TransitionRule,
        TransitionTrigger, apply_adjustments, default_transition_rules, evaluate_transitions,
    },
};

// ── Adversarial ────────────────────────────────────────────────────
// Adversarial agents are Suggestors — they participate in the convergence loop.
// These types are the vocabulary for what adversarial agents produce.

pub use organism_adversarial::{
    AdversarialSignal, AdversarialVerdict, AgentId, Challenge, Complexity, Finding, Severity,
    SkepticismKind,
};

// ── Simulation ─────────────────────────────────────────────────────
// Simulation agents are Suggestors — they stress-test plans inside the loop.
// These types describe simulation results and configuration.

pub use organism_simulation::{
    DimensionResult, OutcomeSimulationAgent, OutcomeSimulator, OutcomeSimulatorConfig,
    RiskLikelihood, Sample, SimulationDimension, SimulationRecommendation, SimulationReport,
    SimulationResult, SimulationVerdict,
};

// ── Learning ───────────────────────────────────────────────────────
// Calibrate priors from execution outcomes.

pub use organism_learning::{
    AdversarialContext, ErrorDimension, LearningEpisode, LearningSignal, Lesson, PredictionError,
    PriorCalibration, SignalKind,
    adapter::{
        build_episode, build_episode_from_run, calibrate_priors, extract_signals,
        extract_signals_from_run, has_infra_failure,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn intent_packet_roundtrip() {
        let expires = Utc::now() + Duration::hours(1);
        let intent = IntentPacket::new("ship feature X", expires);
        let json = serde_json::to_string(&intent).unwrap();
        let back: IntentPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(back.outcome, "ship feature X");
        assert_eq!(back.reversibility, Reversibility::Reversible);
        assert_eq!(back.expiry_action, ExpiryAction::Halt);
    }

    #[test]
    fn intent_packet_builder_chain() {
        let expires = Utc::now() + Duration::hours(2);
        let intent = IntentPacket::new("approve expense", expires)
            .with_context(serde_json::json!({"amount": 2500}))
            .with_authority(vec!["finance".into()])
            .with_reversibility(Reversibility::Irreversible)
            .with_expiry_action(ExpiryAction::Escalate);

        assert_eq!(intent.outcome, "approve expense");
        assert_eq!(intent.context["amount"], 2500);
        assert_eq!(intent.authority, vec!["finance"]);
        assert_eq!(intent.reversibility, Reversibility::Irreversible);
        assert_eq!(intent.expiry_action, ExpiryAction::Escalate);
    }

    #[test]
    fn intent_packet_expired_detection() {
        let past = Utc::now() - Duration::seconds(10);
        let intent = IntentPacket::new("too late", past);
        assert!(intent.is_expired(Utc::now()));

        let future = Utc::now() + Duration::hours(1);
        let intent2 = IntentPacket::new("still valid", future);
        assert!(!intent2.is_expired(Utc::now()));
    }

    #[test]
    fn reversibility_all_variants_serialize() {
        for variant in [
            Reversibility::Reversible,
            Reversibility::Partial,
            Reversibility::Irreversible,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: Reversibility = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn expiry_action_all_variants_serialize() {
        for variant in [
            ExpiryAction::Halt,
            ExpiryAction::Escalate,
            ExpiryAction::CompleteAndHalt,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: ExpiryAction = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
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
    }

    #[test]
    fn declarative_binding_full() {
        let binding = DeclarativeBinding::new()
            .pack("customers", "qualification")
            .pack("knowledge", "enrichment")
            .capability("web", "scraping")
            .capability("ocr", "documents")
            .invariant("lead_has_source")
            .invariant("claim_has_provenance")
            .build();

        assert_eq!(binding.packs.len(), 2);
        assert_eq!(binding.capabilities.len(), 2);
        assert_eq!(binding.invariants.len(), 2);
        assert!((binding.packs[0].confidence - 1.0).abs() < f64::EPSILON);
        assert_eq!(binding.packs[1].pack_name, "knowledge");
        assert_eq!(binding.capabilities[0].capability, "web");
    }

    #[test]
    fn intent_binding_default() {
        let binding = IntentBinding::default();
        assert!(binding.packs.is_empty());
        assert!(binding.capabilities.is_empty());
        assert!(binding.invariants.is_empty());
        assert_eq!(binding.resolution.prior_episodes_consulted, 0);
        assert!((binding.resolution.completeness_confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resolution_level_serde() {
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
    fn resolution_trace_default() {
        let trace = ResolutionTrace::default();
        assert!(trace.levels_attempted.is_empty());
        assert!(trace.levels_contributed.is_empty());
        assert_eq!(trace.prior_episodes_consulted, 0);
    }

    #[test]
    fn intent_binding_serde_roundtrip() {
        let binding = DeclarativeBinding::new()
            .pack("test_pack", "test reason")
            .capability("web", "needed for scraping")
            .invariant("my_invariant")
            .build();

        let json = serde_json::to_string(&binding).unwrap();
        let back: IntentBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back.packs.len(), 1);
        assert_eq!(back.capabilities.len(), 1);
        assert_eq!(back.invariants, vec!["my_invariant"]);
    }

    #[test]
    fn intent_node_leaf() {
        let expires = Utc::now() + Duration::hours(1);
        let intent = IntentPacket::new("leaf task", expires);
        let node = IntentNode::leaf(intent);
        assert!(node.is_leaf());
        assert!(node.children.is_empty());
        assert_eq!(node.intent.outcome, "leaf task");
    }

    #[test]
    fn feasibility_dimension_serde() {
        for dim in [
            FeasibilityDimension::Capability,
            FeasibilityDimension::Context,
            FeasibilityDimension::Resources,
            FeasibilityDimension::Authority,
        ] {
            let json = serde_json::to_string(&dim).unwrap();
            let back: FeasibilityDimension = serde_json::from_str(&json).unwrap();
            assert_eq!(dim, back);
        }
    }

    #[test]
    fn feasibility_kind_serde() {
        for kind in [
            FeasibilityKind::Feasible,
            FeasibilityKind::FeasibleWithConstraints,
            FeasibilityKind::Uncertain,
            FeasibilityKind::Infeasible,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: FeasibilityKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn admission_result_serde_roundtrip() {
        let result = AdmissionResult {
            feasible: true,
            dimensions: vec![FeasibilityAssessment {
                dimension: FeasibilityDimension::Capability,
                kind: FeasibilityKind::Feasible,
                reason: "all good".into(),
            }],
            rejection_reason: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AdmissionResult = serde_json::from_str(&json).unwrap();
        assert!(back.feasible);
        assert_eq!(back.dimensions.len(), 1);
        assert!(back.rejection_reason.is_none());
    }

    #[test]
    fn forbidden_action_serde_roundtrip() {
        let fa = ForbiddenAction {
            action: "delete_all".into(),
            reason: "too destructive".into(),
        };
        let json = serde_json::to_string(&fa).unwrap();
        let back: ForbiddenAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, "delete_all");
        assert_eq!(back.reason, "too destructive");
    }

    #[test]
    fn challenge_fields() {
        let challenge = Challenge::new(
            SkepticismKind::EconomicSkepticism,
            uuid::Uuid::new_v4(),
            "spend too high",
            Severity::Warning,
        );
        assert_eq!(challenge.kind, SkepticismKind::EconomicSkepticism);
        assert_eq!(challenge.severity, Severity::Warning);
        assert_eq!(challenge.description, "spend too high");
    }

    #[test]
    fn challenge_is_blocking() {
        let blocker = Challenge::new(
            SkepticismKind::ConstraintChecking,
            uuid::Uuid::new_v4(),
            "hard stop",
            Severity::Blocker,
        );
        let warning = Challenge::new(
            SkepticismKind::ConstraintChecking,
            uuid::Uuid::new_v4(),
            "soft",
            Severity::Warning,
        );
        assert!(blocker.is_blocking());
        assert!(!warning.is_blocking());
    }

    #[test]
    fn severity_all_variants_accessible() {
        let variants = [Severity::Advisory, Severity::Warning, Severity::Blocker];
        assert_eq!(variants.len(), 3);
    }

    #[test]
    fn skepticism_kind_all_variants_accessible() {
        let variants = [
            SkepticismKind::AssumptionBreaking,
            SkepticismKind::ConstraintChecking,
            SkepticismKind::CausalSkepticism,
            SkepticismKind::EconomicSkepticism,
            SkepticismKind::OperationalSkepticism,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn simulation_dimension_all_variants_accessible() {
        let variants = [
            SimulationDimension::Outcome,
            SimulationDimension::Cost,
            SimulationDimension::Policy,
            SimulationDimension::Causal,
            SimulationDimension::Operational,
        ];
        assert_eq!(variants.len(), 5);
    }

    #[test]
    fn dimension_result_construction() {
        let result = DimensionResult {
            dimension: SimulationDimension::Cost,
            passed: true,
            confidence: 0.95,
            findings: vec!["within budget".into()],
            samples: vec![],
        };
        assert!(result.passed);
        assert!((result.confidence - 0.95).abs() < f64::EPSILON);
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn lesson_construction() {
        let lesson = Lesson {
            insight: "score 0.88 → approved".into(),
            context: "expense approval".into(),
            confidence: 0.9,
            planning_adjustment: "none".into(),
        };
        assert_eq!(lesson.insight, "score 0.88 → approved");
        assert!((lesson.confidence - 0.9).abs() < f64::EPSILON);
    }
}
