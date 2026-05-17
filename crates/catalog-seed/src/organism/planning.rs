//! Descriptors for `organism-planning::*` agents.

use converge_kernel::ContextKey;
use converge_kernel::formation::{SuggestorCapability, SuggestorRole};
use converge_provider::{CostClass, LatencyClass};
use organism_catalog::{CatalogSuggestorDescriptor, LoopContribution};

use crate::{EntrySpec, entry};

#[must_use]
pub fn descriptors() -> Vec<CatalogSuggestorDescriptor> {
    vec![
        huddle_seed(),
        hypothesis_tracker(),
        breadth_research(),
        depth_research(),
        fact_extractor(),
        gap_detector(),
        contradiction_finder(),
        synthesis(),
    ]
}

#[must_use]
pub fn huddle_seed() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-huddle-seed",
        role: SuggestorRole::Planning,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Hypotheses, ContextKey::Strategies],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["planning", "huddle", "seeding"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Seed a deliberation huddle with initial hypotheses derived from the intent.",
        use_when: "When a huddle needs starting material from the root intent before round 1.",
        examples: vec![
            "what are the obvious first hypotheses to consider",
            "seed the deliberation with three starting angles",
        ],
        loop_contributions: vec![LoopContribution::Propose],
        produces: vec!["organism.planning.huddle-seed"],
    })
}

#[must_use]
pub fn hypothesis_tracker() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-hypothesis-tracker",
        role: SuggestorRole::Meta,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Diagnostic],
        reads: vec![ContextKey::Hypotheses, ContextKey::Evaluations],
        domain_tags: vec!["planning", "tracking", "telemetry"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Track hypothesis evolution across deliberation rounds for audit and convergence checks.",
        use_when: "When you need to observe how hypotheses change across a huddle.",
        examples: vec![
            "which hypotheses survived three rounds",
            "show me hypotheses that pivoted significantly",
        ],
        loop_contributions: vec![LoopContribution::Observe],
        produces: vec!["organism.planning.hypothesis-trace"],
    })
}

#[must_use]
pub fn breadth_research() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-breadth-research",
        role: SuggestorRole::Signal,
        capabilities: vec![
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::LlmReasoning,
        ],
        output_keys: vec![ContextKey::Signals, ContextKey::Hypotheses],
        reads: vec![ContextKey::Seeds],
        domain_tags: vec!["planning", "research", "breadth", "due-diligence"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Broad research sweep — wide net over many angles, shallow per angle.",
        use_when: "When mapping the space of possibilities before going deep.",
        examples: vec![
            "what are all the angles on this question",
            "give me a 20-perspective scan before we narrow",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Propose],
        produces: vec!["organism.planning.breadth-research"],
    })
}

#[must_use]
pub fn depth_research() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-depth-research",
        role: SuggestorRole::Signal,
        capabilities: vec![
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::LlmReasoning,
        ],
        output_keys: vec![ContextKey::Signals, ContextKey::Hypotheses],
        reads: vec![ContextKey::Seeds, ContextKey::Hypotheses],
        domain_tags: vec!["planning", "research", "depth", "due-diligence"],
        cost: CostClass::High,
        latency: LatencyClass::Batch,
        summary: "Deep research — pick one angle and go far down it.",
        use_when: "After breadth research has surfaced the angles worth investing in.",
        examples: vec![
            "go deep on the regulatory exposure of option B",
            "exhaust the literature on this one mechanism",
        ],
        loop_contributions: vec![LoopContribution::Retrieve],
        produces: vec!["organism.planning.depth-research"],
    })
}

#[must_use]
pub fn fact_extractor() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-fact-extractor",
        role: SuggestorRole::Analysis,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Hypotheses],
        reads: vec![ContextKey::Signals],
        domain_tags: vec!["planning", "extraction", "structuring"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Extract structured facts from unstructured research signals.",
        use_when: "When research text needs to be turned into typed facts the engine can act on.",
        examples: vec![
            "extract dates and counterparties from these documents",
            "structure these findings into typed claims",
        ],
        loop_contributions: vec![LoopContribution::Retrieve, LoopContribution::Synthesize],
        produces: vec!["organism.planning.fact-extraction"],
    })
}

#[must_use]
pub fn gap_detector() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-gap-detector",
        role: SuggestorRole::Analysis,
        capabilities: vec![SuggestorCapability::Analytics],
        output_keys: vec![ContextKey::Diagnostic],
        reads: vec![ContextKey::Hypotheses, ContextKey::Signals],
        domain_tags: vec!["planning", "gaps", "coverage", "diligence"],
        cost: CostClass::Low,
        latency: LatencyClass::Interactive,
        summary: "Identify gaps — questions raised that have no supporting evidence yet.",
        use_when: "Before promoting a decision, check that every claim is grounded.",
        examples: vec![
            "what claims are still unsupported",
            "which questions did we raise and not answer",
        ],
        loop_contributions: vec![LoopContribution::Validate, LoopContribution::Observe],
        produces: vec!["organism.planning.gap-detection"],
    })
}

#[must_use]
pub fn contradiction_finder() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-contradiction-finder",
        role: SuggestorRole::Analysis,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Disagreements, ContextKey::Diagnostic],
        reads: vec![ContextKey::Hypotheses, ContextKey::Signals],
        domain_tags: vec!["planning", "contradiction", "consistency"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Find contradictions between facts, hypotheses, and evidence.",
        use_when: "When the body of evidence is large and internal inconsistencies are likely.",
        examples: vec![
            "do any of these claims contradict each other",
            "flag inconsistencies between source A and source B",
        ],
        loop_contributions: vec![LoopContribution::Validate, LoopContribution::Challenge],
        produces: vec!["organism.planning.contradiction"],
    })
}

#[must_use]
pub fn synthesis() -> CatalogSuggestorDescriptor {
    entry(EntrySpec {
        id: "organism-synthesis",
        role: SuggestorRole::Synthesis,
        capabilities: vec![SuggestorCapability::LlmReasoning],
        output_keys: vec![ContextKey::Proposals],
        reads: vec![
            ContextKey::Hypotheses,
            ContextKey::Evaluations,
            ContextKey::Constraints,
        ],
        domain_tags: vec!["planning", "synthesis", "decision"],
        cost: CostClass::Medium,
        latency: LatencyClass::Interactive,
        summary: "Synthesize a final proposal from accumulated hypotheses, evaluations, and constraints.",
        use_when: "At the end of deliberation when the engine needs a single coherent recommendation.",
        examples: vec![
            "give me the recommended decision now",
            "synthesize what we've learned into one proposal",
        ],
        loop_contributions: vec![LoopContribution::Synthesize],
        produces: vec!["organism.planning.synthesis"],
    })
}
