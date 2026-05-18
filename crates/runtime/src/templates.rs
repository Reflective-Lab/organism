//! Five named formation templates keyed by [`ProblemClass`].
//!
//! Each template declares the roles and capabilities a formation needs to
//! solve one *kind* of problem. Concrete Suggestor IDs are NOT baked in —
//! the [`crate::compiler::FormationCompiler`] resolves roles to descriptors at
//! compile time via the catalogs the host passes in. That's how a template
//! survives swapping out a particular skeptic implementation without
//! disturbing the routing.
//!
//! See `kb/Concepts/Formation.md` (organism) and the converge-model
//! `FormationTemplate` docs for how the metadata interacts with the
//! resolver/compiler.

use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateMetadata, StaticFormationTemplate,
    SuggestorCapability, SuggestorRole,
};
use organism_intent::problem::ProblemClass;
use serde::{Deserialize, Serialize};

/// Coarse cost class for a formation template. Used by [`crate::FormationGuru`]
/// to bias selection toward cheaper templates when match quality is comparable.
///
/// This is an organism-side hint layered on top of converge-model's
/// [`FormationTemplate`], which doesn't itself carry cost metadata. Real
/// calibration would come from learning-episode win/cost data; until then,
/// these are the standard templates' best-guess profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostHint {
    /// Cheap to run — small LLM context, no expensive retrieval.
    Low,
    /// Average LLM + tooling cost.
    Medium,
    /// Expensive — broad retrieval, many LLM rounds, or HITL.
    High,
}

impl CostHint {
    /// Higher = cheaper. Used by the guru's composite score.
    #[must_use]
    pub fn cheapness_bonus(self) -> i32 {
        match self {
            Self::Low => 2,
            Self::Medium => 1,
            Self::High => 0,
        }
    }
}

/// Best-guess cost class for the standard templates. Unknown ids default to
/// [`CostHint::Medium`].
///
/// `organism-research` (open-ended retrieval) and `organism-diligence`
/// (multi-source + HITL) are [`CostHint::High`]; everything else, including
/// unknown ids, is [`CostHint::Medium`].
#[must_use]
pub fn cost_hint_for(template_id: &str) -> CostHint {
    match template_id {
        "organism-research" | "organism-diligence" => CostHint::High,
        _ => CostHint::Medium,
    }
}

/// Template id used by every standard organism template — keep this stable;
/// downstream consumers may match on it.
///
/// `Incident` falls back to `organism-decision` and `Strategy` falls back to
/// `organism-research` (no dedicated templates yet).
pub fn template_id_for(class: ProblemClass) -> &'static str {
    match class {
        ProblemClass::Decision | ProblemClass::Incident => "organism-decision",
        ProblemClass::Research | ProblemClass::Strategy => "organism-research",
        ProblemClass::Evaluation => "organism-evaluation",
        ProblemClass::Planning => "organism-planning",
        ProblemClass::Diligence => "organism-diligence",
    }
}

/// Build the catalog of all five standard organism formation templates.
///
/// Hosts compose this with their domain-specific catalogs (e.g.
/// `vendor_selection_formation_catalog`) by adding more templates onto the
/// returned catalog.
#[must_use]
pub fn standard_formation_catalog() -> FormationCatalog {
    FormationCatalog::new()
        .with_template(decision_formation())
        .with_template(research_formation())
        .with_template(evaluation_formation())
        .with_template(planning_formation())
        .with_template(diligence_formation())
}

/// Decision: pick one option from a candidate set under stated authority and
/// constraints. LLM reasoner proposes; constraint checker gates; economic
/// skeptic challenges cost assumptions; synthesizer commits the choice.
///
/// Also routes [`ProblemClass::Incident`] intents — `template_id_for`
/// documents Incident as falling back to `organism-decision`. The
/// `incident` keyword keeps that fallback visible in catalog metadata
/// rather than hidden in `FormationGuru` control flow.
#[must_use]
pub fn decision_formation() -> FormationTemplate {
    let metadata = FormationTemplateMetadata::new(
        "organism-decision".to_string(),
        "Pick one option under authority + constraint review".to_string(),
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Constraint,
            SuggestorRole::Evaluation,
            SuggestorRole::Synthesis,
        ],
    )
    .with_keyword("decision")
    .with_keyword("decide")
    .with_keyword("select")
    .with_keyword("approve")
    .with_keyword("choose")
    .with_keyword("incident")
    .with_entity("approval")
    .with_entity("option")
    .with_required_capability(SuggestorCapability::LlmReasoning)
    .with_required_capability(SuggestorCapability::PolicyEnforcement)
    .with_required_capability(SuggestorCapability::Analytics);
    FormationTemplate::static_template(StaticFormationTemplate::new(metadata))
}

/// Research: open-ended fact gathering. LLM reasoner explores, knowledge
/// retrieval surfaces evidence, synthesizer summarizes.
///
/// Also routes [`ProblemClass::Strategy`] intents — `template_id_for`
/// documents Strategy as falling back to `organism-research`. The
/// `strategy` keyword keeps that fallback visible in catalog metadata
/// rather than hidden in `FormationGuru` control flow.
#[must_use]
pub fn research_formation() -> FormationTemplate {
    let metadata = FormationTemplateMetadata::new(
        "organism-research".to_string(),
        "Open-ended fact-finding with synthesis".to_string(),
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Analysis,
            SuggestorRole::Synthesis,
        ],
    )
    .with_keyword("research")
    .with_keyword("investigate")
    .with_keyword("explore")
    .with_keyword("discover")
    .with_keyword("study")
    .with_keyword("strategy")
    .with_entity("topic")
    .with_entity("evidence")
    .with_required_capability(SuggestorCapability::LlmReasoning)
    .with_required_capability(SuggestorCapability::KnowledgeRetrieval);
    FormationTemplate::static_template(StaticFormationTemplate::new(metadata))
}

/// Evaluation: score and rank candidates against criteria. LLM reasoner
/// extracts criteria; analytics scores; economic skeptic challenges weight
/// choices; synthesizer ranks.
#[must_use]
pub fn evaluation_formation() -> FormationTemplate {
    let metadata = FormationTemplateMetadata::new(
        "organism-evaluation".to_string(),
        "Score and rank candidates against criteria".to_string(),
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Evaluation,
            SuggestorRole::Synthesis,
        ],
    )
    .with_keyword("evaluation")
    .with_keyword("evaluate")
    .with_keyword("assess")
    .with_keyword("rank")
    .with_keyword("score")
    .with_keyword("compare")
    .with_entity("candidate")
    .with_entity("rubric")
    .with_required_capability(SuggestorCapability::LlmReasoning)
    .with_required_capability(SuggestorCapability::Analytics)
    .with_required_capability(SuggestorCapability::PolicyEnforcement);
    FormationTemplate::static_template(StaticFormationTemplate::new(metadata))
}

/// Planning: forward-looking sequencing. LLM reasoner drafts; assumption
/// breaker challenges; constraint checker gates feasibility; synthesizer
/// builds the schedule.
#[must_use]
pub fn planning_formation() -> FormationTemplate {
    let metadata = FormationTemplateMetadata::new(
        "organism-planning".to_string(),
        "Sequence work over time with feasibility checks".to_string(),
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Planning,
            SuggestorRole::Constraint,
            SuggestorRole::Synthesis,
        ],
    )
    .with_keyword("planning")
    .with_keyword("plan")
    .with_keyword("schedule")
    .with_keyword("design")
    .with_keyword("sequence")
    .with_keyword("rollout")
    .with_entity("milestone")
    .with_entity("dependency")
    .with_required_capability(SuggestorCapability::LlmReasoning)
    .with_required_capability(SuggestorCapability::Analytics)
    .with_required_capability(SuggestorCapability::PolicyEnforcement);
    FormationTemplate::static_template(StaticFormationTemplate::new(metadata))
}

/// Diligence: adversarial fact-gathering with a verdict. LLM reasoner
/// hypothesizes; knowledge retrieval surfaces evidence; assumption breaker,
/// constraint checker, and economic skeptic stress-test the verdict; HITL
/// gates the recommendation.
#[must_use]
pub fn diligence_formation() -> FormationTemplate {
    let metadata = FormationTemplateMetadata::new(
        "organism-diligence".to_string(),
        "Adversarial verification ending in a defended verdict".to_string(),
        vec![
            SuggestorRole::Signal,
            SuggestorRole::Evaluation,
            SuggestorRole::Constraint,
            SuggestorRole::Synthesis,
        ],
    )
    .with_keyword("diligence")
    .with_keyword("vet")
    .with_keyword("audit")
    .with_keyword("verify")
    .with_keyword("validate")
    .with_keyword("qualify")
    .with_entity("target")
    .with_entity("claim")
    .with_entity("evidence")
    .with_required_capability(SuggestorCapability::LlmReasoning)
    .with_required_capability(SuggestorCapability::KnowledgeRetrieval)
    .with_required_capability(SuggestorCapability::PolicyEnforcement)
    .with_required_capability(SuggestorCapability::HumanInTheLoop);
    FormationTemplate::static_template(StaticFormationTemplate::new(metadata))
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::formation::FormationTemplateQuery;

    #[test]
    fn standard_catalog_has_five_templates() {
        let catalog = standard_formation_catalog();
        let templates: Vec<_> = catalog.into_iter().collect();
        assert_eq!(templates.len(), 5);
    }

    #[test]
    fn decision_keyword_resolves_decision_template() {
        let catalog = standard_formation_catalog();
        let query = FormationTemplateQuery::new()
            .with_keyword("decide")
            .with_required_capability(SuggestorCapability::LlmReasoning);
        let template = catalog
            .top_match(&query)
            .expect("decision query matches a template");
        assert_eq!(template.id(), "organism-decision");
    }

    #[test]
    fn research_keyword_resolves_research_template() {
        let catalog = standard_formation_catalog();
        let query = FormationTemplateQuery::new()
            .with_keyword("research")
            .with_required_capability(SuggestorCapability::KnowledgeRetrieval);
        let template = catalog
            .top_match(&query)
            .expect("research query matches a template");
        assert_eq!(template.id(), "organism-research");
    }

    #[test]
    fn evaluation_keyword_resolves_evaluation_template() {
        let catalog = standard_formation_catalog();
        let query = FormationTemplateQuery::new()
            .with_keyword("evaluate")
            .with_required_capability(SuggestorCapability::Analytics);
        let template = catalog
            .top_match(&query)
            .expect("evaluation query matches a template");
        assert_eq!(template.id(), "organism-evaluation");
    }

    #[test]
    fn planning_keyword_resolves_planning_template() {
        let catalog = standard_formation_catalog();
        let query = FormationTemplateQuery::new()
            .with_keyword("plan")
            .with_required_capability(SuggestorCapability::LlmReasoning);
        let template = catalog
            .top_match(&query)
            .expect("planning query matches a template");
        assert_eq!(template.id(), "organism-planning");
    }

    #[test]
    fn diligence_keyword_resolves_diligence_template() {
        let catalog = standard_formation_catalog();
        let query = FormationTemplateQuery::new()
            .with_keyword("vet")
            .with_required_capability(SuggestorCapability::HumanInTheLoop);
        let template = catalog
            .top_match(&query)
            .expect("diligence query matches a template");
        assert_eq!(template.id(), "organism-diligence");
    }

    #[test]
    fn cost_hints_are_assigned_for_all_standard_templates() {
        for class in [
            ProblemClass::Decision,
            ProblemClass::Research,
            ProblemClass::Evaluation,
            ProblemClass::Planning,
            ProblemClass::Diligence,
        ] {
            let id = template_id_for(class);
            // Just exercise the lookup — every standard id should resolve.
            let _ = cost_hint_for(id);
        }
    }

    #[test]
    fn unknown_template_id_defaults_to_medium_cost() {
        assert_eq!(cost_hint_for("not-a-real-template"), CostHint::Medium);
    }

    #[test]
    fn cheaper_templates_score_higher() {
        assert!(CostHint::Low.cheapness_bonus() > CostHint::Medium.cheapness_bonus());
        assert!(CostHint::Medium.cheapness_bonus() > CostHint::High.cheapness_bonus());
    }

    #[test]
    fn template_id_for_problem_class_is_stable() {
        assert_eq!(template_id_for(ProblemClass::Decision), "organism-decision");
        assert_eq!(template_id_for(ProblemClass::Research), "organism-research");
        assert_eq!(
            template_id_for(ProblemClass::Evaluation),
            "organism-evaluation"
        );
        assert_eq!(template_id_for(ProblemClass::Planning), "organism-planning");
        assert_eq!(
            template_id_for(ProblemClass::Diligence),
            "organism-diligence"
        );
        // Incident and Strategy fall back to existing templates.
        assert_eq!(template_id_for(ProblemClass::Incident), "organism-decision");
        assert_eq!(template_id_for(ProblemClass::Strategy), "organism-research");
    }
}
