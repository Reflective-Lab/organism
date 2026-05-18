//! FormationGuru — picks formation templates from a catalog given an
//! [`IntentPacket`] and the host's available capabilities.
//!
//! The guru is the *pre-loop* selection path: it classifies the intent, queries
//! a [`FormationCatalog`] for matching templates, and returns a primary plus
//! up to two alternates with a [`SelectionTrace`] explaining the choice.
//! Running an actual tournament across the candidates happens one level up in
//! [`crate::Runtime`]'s auto mode — the guru is concerned only with *which*
//! templates are worth running.
//!
//! Pair the guru with [`crate::classifier::ProblemClassifierSuggestor`] when
//! you want the in-loop refinement: the guru picks based on the IntentPacket
//! up front; the classifier keeps the chosen formation's working context
//! aware of the problem class as new seeds and signals arrive.

use converge_kernel::formation::{
    FormationCatalog, FormationTemplate, FormationTemplateQuery, SuggestorCapability,
};
use organism_intent::IntentPacket;
use organism_intent::problem::{ProblemClass, ProblemClassification, classify};
use serde::{Deserialize, Serialize};

use crate::templates::{CostHint, cost_hint_for};

/// Result of the guru's selection over an intent + capability inventory.
#[derive(Debug, Clone)]
pub struct GuruSelection<'cat> {
    /// Best-matching template — what the runtime should run first.
    pub primary: &'cat FormationTemplate,
    /// Up to two next-best templates. Auto-tournament mode runs all of these
    /// alongside `primary` and picks the convergent winner.
    pub alternates: Vec<&'cat FormationTemplate>,
    /// The classification computed from the IntentPacket.
    pub classification: ProblemClassification,
    /// Auditable record of what was queried and why this template won. Safe
    /// to log, render in UIs, and serialize for replay.
    pub trace: SelectionTrace,
}

/// Auditable record of a guru selection decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionTrace {
    pub problem_class: ProblemClass,
    /// Keywords that fired in the classifier (drawn from the intent text).
    pub matched_keywords: Vec<String>,
    /// True if the classifier hit no keywords and used its default class.
    pub defaulted: bool,
    /// The keywords sent to the catalog query.
    pub query_keywords: Vec<String>,
    /// The capabilities sent to the catalog query.
    pub query_capabilities: Vec<SuggestorCapability>,
    /// Template ids the catalog returned, in final rank order (after the
    /// guru's composite re-rank). For component scores per id, see
    /// [`Self::scores`].
    pub considered: Vec<String>,
    /// Per-candidate score breakdown, in the same order as
    /// [`Self::considered`]. Auditable record of *why* the guru ranked the
    /// templates the way it did — catalog rank, capability surplus, cost.
    pub scores: Vec<CandidateScore>,
    /// Id of the chosen primary template.
    pub primary_id: String,
    /// One-line reason for why this template won.
    pub primary_reason: String,
}

/// Component-by-component score breakdown for one candidate template. Use
/// the components to debug ranking surprises ("why did template X beat Y?").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateScore {
    pub template_id: String,
    /// Position in the catalog's keyword/entity match ranking. 0 = best
    /// match. Inverted into a bonus by the composite formula.
    pub catalog_rank: usize,
    /// How many of the host's capabilities the template does NOT require.
    /// Larger = template is "underspending" the host = preferable, all
    /// else equal. (A diligence template that needs HITL is fine on a
    /// HITL host but won't be picked over a decision template that runs
    /// on a no-HITL host.)
    pub capability_surplus: usize,
    /// Organism's cost hint for this template. Cheaper = bonus.
    pub cost_hint: CostHint,
    /// The integer the composite formula produced — higher wins.
    pub composite: i32,
}

/// Why the guru couldn't pick a formation.
#[derive(Debug, thiserror::Error)]
pub enum GuruError {
    /// The catalog had no template matching the classified problem under the
    /// available capabilities. Means the host's catalog is incomplete for
    /// this kind of work.
    #[error("no formation template matches problem class {class} with available capabilities")]
    NoMatch { class: ProblemClass },
}

/// Picks formations from a [`FormationCatalog`].
pub struct FormationGuru<'a> {
    catalog: &'a FormationCatalog,
}

impl<'a> FormationGuru<'a> {
    #[must_use]
    pub fn new(catalog: &'a FormationCatalog) -> Self {
        Self { catalog }
    }

    /// Pick a primary formation and up to two alternates for `intent`,
    /// constrained by the host's `available_capabilities`. The guru returns
    /// references into the catalog — keep the catalog alive while the
    /// selection is in use.
    ///
    /// # Errors
    ///
    /// [`GuruError::NoMatch`] if the catalog has no template that satisfies
    /// the classified problem under the supplied capabilities.
    pub fn select(
        &self,
        intent: &IntentPacket,
        available_capabilities: &[SuggestorCapability],
    ) -> Result<GuruSelection<'a>, GuruError> {
        let classification = classify(intent);
        let query_keywords = query_keywords_for(&classification);
        // Build a keyword-only query. Capability matching here would invert
        // the semantics — `with_required_capability` says "template must
        // require this", which is not the same as "host has this".
        // We post-filter by `available_capabilities` once we have matches.
        let mut query = FormationTemplateQuery::new();
        for kw in &query_keywords {
            query = query.with_keyword(kw.clone());
        }

        let raw_matches = self.catalog.matches(&query);
        let filtered: Vec<&FormationTemplate> = raw_matches
            .into_iter()
            .filter(|t| host_satisfies(t, available_capabilities))
            .collect();
        if filtered.is_empty() {
            return Err(GuruError::NoMatch {
                class: classification.class,
            });
        }

        // Composite scoring: catalog rank dominates, surplus + cost break
        // ties (and can flip near-ties). Catalog rank is the dominant signal
        // because it reflects the keyword/entity match the user actually
        // expressed; surplus + cost are tiebreakers, not primary signals.
        let mut scored: Vec<(CandidateScore, &FormationTemplate)> = filtered
            .iter()
            .enumerate()
            .map(|(rank, template)| {
                let surplus = capability_surplus(template, available_capabilities);
                let cost = cost_hint_for(template.id());
                let catalog_bonus =
                    i32::try_from(filtered.len().saturating_sub(rank)).unwrap_or(i32::MAX);
                let composite = catalog_bonus * 10
                    + i32::try_from(surplus).unwrap_or(0)
                    + cost.cheapness_bonus();
                (
                    CandidateScore {
                        template_id: template.id().to_owned(),
                        catalog_rank: rank,
                        capability_surplus: surplus,
                        cost_hint: cost,
                        composite,
                    },
                    *template,
                )
            })
            .collect();

        // Higher composite wins; stable on ties (preserves catalog order).
        scored.sort_by(|a, b| b.0.composite.cmp(&a.0.composite));

        let primary = scored[0].1;
        let alternates: Vec<&FormationTemplate> =
            scored.iter().skip(1).take(2).map(|(_, t)| *t).collect();

        let considered: Vec<String> = scored.iter().map(|(s, _)| s.template_id.clone()).collect();
        let candidate_scores: Vec<CandidateScore> = scored.iter().map(|(s, _)| s.clone()).collect();

        let primary_reason = if classification.defaulted {
            format!(
                "no problem-class keywords matched; defaulted to {} and picked {} (composite {})",
                classification.class,
                primary.id(),
                candidate_scores[0].composite,
            )
        } else {
            format!(
                "{} matched {} → top template {} (composite {}, surplus {}, cost {:?})",
                classification.class,
                classification
                    .matched_keywords
                    .first()
                    .map_or("(no kw)", String::as_str),
                primary.id(),
                candidate_scores[0].composite,
                candidate_scores[0].capability_surplus,
                candidate_scores[0].cost_hint,
            )
        };

        let trace = SelectionTrace {
            problem_class: classification.class,
            matched_keywords: classification.matched_keywords.clone(),
            defaulted: classification.defaulted,
            query_keywords,
            query_capabilities: available_capabilities.to_vec(),
            considered,
            scores: candidate_scores,
            primary_id: primary.id().to_owned(),
            primary_reason,
        };

        Ok(GuruSelection {
            primary,
            alternates,
            classification,
            trace,
        })
    }
}

/// How many of the host's available capabilities the template does NOT
/// require. Bigger = template is underspending the host's resources.
fn capability_surplus(template: &FormationTemplate, available: &[SuggestorCapability]) -> usize {
    let required = &template.metadata().required_capabilities;
    available
        .iter()
        .filter(|cap| !required.contains(cap))
        .count()
}

/// Build the catalog query keywords for a classification. We send the
/// matched keywords (when any) plus the class name itself, since the
/// standard templates tag themselves with `decision`/`research`/etc. as
/// keywords.
fn query_keywords_for(classification: &ProblemClassification) -> Vec<String> {
    let mut keywords = vec![classification.class.as_str().to_owned()];
    for kw in &classification.matched_keywords {
        if !keywords.contains(kw) {
            keywords.push(kw.clone());
        }
    }
    keywords
}

/// True iff every capability the template requires is in the host's
/// available capabilities. Templates declaring capabilities the host can't
/// supply are filtered out before being returned to the caller.
fn host_satisfies(template: &FormationTemplate, available: &[SuggestorCapability]) -> bool {
    template
        .metadata()
        .required_capabilities
        .iter()
        .all(|cap| available.contains(cap))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::standard_formation_catalog;
    use chrono::{Duration, Utc};

    fn intent(outcome: &str) -> IntentPacket {
        IntentPacket::new(outcome, Utc::now() + Duration::hours(1))
    }

    fn caps() -> Vec<SuggestorCapability> {
        vec![
            SuggestorCapability::LlmReasoning,
            SuggestorCapability::Analytics,
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::HumanInTheLoop,
        ]
    }

    #[test]
    fn picks_decision_for_decision_intent() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(&intent("decide which vendor to approve"), &caps())
            .expect("decision intent matches");
        assert_eq!(selection.primary.id(), "organism-decision");
        assert_eq!(selection.classification.class, ProblemClass::Decision);
        assert_eq!(selection.trace.primary_id, "organism-decision");
    }

    #[test]
    fn picks_diligence_for_vetting_intent() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(&intent("vet the acquisition target end-to-end"), &caps())
            .expect("diligence intent matches");
        assert_eq!(selection.primary.id(), "organism-diligence");
    }

    #[test]
    fn picks_research_for_open_ended_intent() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(&intent("research the competitive landscape"), &caps())
            .expect("research intent matches");
        assert_eq!(selection.primary.id(), "organism-research");
    }

    /// `template_id_for` documents Incident as falling back to
    /// `organism-decision`. The fallback must be honored through the
    /// guru's normal keyword-driven path — proved by the `incident`
    /// keyword on the decision template's metadata, not by hidden
    /// control flow in `FormationGuru`.
    #[test]
    fn picks_decision_for_incident_intent() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(
                &intent("respond to the production incident and stabilize the system"),
                &caps(),
            )
            .expect("incident intent must match a template");
        assert_eq!(selection.classification.class, ProblemClass::Incident);
        assert_eq!(selection.primary.id(), "organism-decision");
        // Trace shows keyword-driven routing, not a special-case fallback.
        assert!(!selection.trace.defaulted);
        assert!(
            selection
                .trace
                .matched_keywords
                .iter()
                .any(|k| k == "incident")
        );
    }

    /// `template_id_for` documents Strategy as falling back to
    /// `organism-research`. Same contract as Incident → Decision —
    /// visible as template metadata, not hidden control flow.
    #[test]
    fn picks_research_for_strategy_intent() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(
                &intent("set our 3-year strategy and define the long-term vision"),
                &caps(),
            )
            .expect("strategy intent must match a template");
        assert_eq!(selection.classification.class, ProblemClass::Strategy);
        assert_eq!(selection.primary.id(), "organism-research");
        assert!(!selection.trace.defaulted);
        assert!(
            selection
                .trace
                .matched_keywords
                .iter()
                .any(|k| k == "strategy")
        );
    }

    #[test]
    fn missing_capabilities_filters_template_out() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        // Diligence requires HumanInTheLoop. Strip it from capabilities and
        // the diligence template should not be returned.
        let limited = vec![
            SuggestorCapability::LlmReasoning,
            SuggestorCapability::Analytics,
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::KnowledgeRetrieval,
        ];
        let result = guru.select(&intent("vet the acquisition target"), &limited);
        // Either no match (most strict) OR a fallback template that doesn't
        // require HITL. The contract here is "diligence is NOT picked".
        if let Ok(selection) = result {
            assert_ne!(selection.primary.id(), "organism-diligence");
        }
    }

    #[test]
    fn defaulted_classification_records_defaulted_in_trace() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(&intent("doing the thing today"), &caps())
            .expect("default classification still matches a template");
        assert!(selection.trace.defaulted);
        assert_eq!(selection.classification.class, ProblemClass::Decision);
        assert!(selection.trace.primary_reason.contains("defaulted"));
    }

    #[test]
    fn alternates_capped_at_two() {
        let catalog = standard_formation_catalog();
        let guru = FormationGuru::new(&catalog);
        let selection = guru
            .select(&intent("decide and evaluate the proposal"), &caps())
            .expect("matches");
        assert!(selection.alternates.len() <= 2);
    }
}
