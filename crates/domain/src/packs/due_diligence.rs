//! Due Diligence pack — convergent research, fact extraction, gap detection,
//! contradiction finding, and synthesis for investment or procurement decisions.
//!
//! Born from monterro (PE portfolio intelligence) and hackathon (governance demo).
//! The pattern is: seed research strategies → search wide and deep → extract facts
//! → detect gaps and contradictions → synthesize when stable.
//!
//! Fact prefixes: `signal:`, `hypothesis:`, `contradiction:`, `strategy:gap:`,
//! `synthesis:`

use crate::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta, PackProfile};

pub const AGENTS: &[AgentMeta] = &[
    AgentMeta {
        name: "breadth_researcher",
        dependencies: &[ContextKey::Strategies],
        fact_prefix: "signal:",
        target_key: ContextKey::Signals,
        description: "Wide entity discovery — searches for company, market, and competitive context",
    },
    AgentMeta {
        name: "depth_researcher",
        dependencies: &[ContextKey::Strategies],
        fact_prefix: "signal:",
        target_key: ContextKey::Signals,
        description: "Deep content reading — searches for architecture, financials, and ownership evidence",
    },
    AgentMeta {
        name: "fact_extractor",
        dependencies: &[ContextKey::Signals],
        fact_prefix: "hypothesis:",
        target_key: ContextKey::Hypotheses,
        description: "LLM reads raw signals and extracts tagged, sourced factual claims",
    },
    AgentMeta {
        name: "gap_detector",
        dependencies: &[ContextKey::Hypotheses],
        fact_prefix: "strategy:gap:",
        target_key: ContextKey::Strategies,
        description: "Reviews accumulated facts, identifies critical gaps, injects follow-up strategies",
    },
    AgentMeta {
        name: "contradiction_finder",
        dependencies: &[ContextKey::Hypotheses],
        fact_prefix: "contradiction:",
        target_key: ContextKey::Evaluations,
        description: "Detects conflicting claims across sources and flags them for human review",
    },
    AgentMeta {
        name: "synthesis",
        dependencies: &[ContextKey::Hypotheses, ContextKey::Evaluations],
        fact_prefix: "synthesis:",
        target_key: ContextKey::Proposals,
        description: "Produces final analysis when hypotheses stabilize — executive summary, recommendation, risk factors",
    },
];

pub const INVARIANTS: &[InvariantMeta] = &[
    InvariantMeta {
        name: "hypothesis_has_source",
        class: InvariantClass::Structural,
        description: "Every hypothesis must cite a signal source",
    },
    InvariantMeta {
        name: "contradictions_flagged",
        class: InvariantClass::Semantic,
        description: "Contradicting hypotheses must produce evaluation facts before convergence",
    },
    InvariantMeta {
        name: "synthesis_requires_coverage",
        class: InvariantClass::Acceptance,
        description: "Synthesis cannot converge without minimum category coverage (product, market, competition, technology, financials)",
    },
];

pub const PROFILE: PackProfile = PackProfile {
    entities: &[
        "company",
        "product",
        "competitor",
        "investor",
        "market",
        "technology",
        "risk",
    ],
    required_capabilities: &["web", "llm"],
    uses_llm: true,
    requires_hitl: true,
    handles_irreversible: false,
    keywords: &[
        "due diligence",
        "diligence",
        "research",
        "analysis",
        "investment",
        "portfolio",
        "assessment",
        "fact extraction",
        "contradiction",
    ],
};
