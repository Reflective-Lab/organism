//! Registry — the runtime's catalog of available packs, capabilities, and invariants.
//!
//! Intent resolution queries the registry to find what's available.
//! Apps register what they've wired up; the resolver searches across it.

use organism_domain::pack::{AgentMeta, ContextKey, InvariantClass, InvariantMeta, PackProfile};
use organism_intent::resolution::{
    IntentBinding, IntentResolver, PackRequirement, ResolutionLevel,
};

// ── Registered Pack ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RegisteredPack {
    pub name: String,
    pub description: String,
    pub fact_prefixes: Vec<String>,
    pub agent_names: Vec<String>,
    pub invariant_names: Vec<String>,
    pub agent_count: usize,
    pub invariant_count: usize,
    pub context_keys_read: Vec<ContextKey>,
    pub context_keys_written: Vec<ContextKey>,
    pub has_acceptance_invariants: bool,
    pub profile: PackProfile,
}

#[derive(Debug, Clone)]
pub struct RegisteredCapability {
    pub name: String,
    pub description: String,
}

// ── Registry ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Registry {
    packs: Vec<RegisteredPack>,
    capabilities: Vec<RegisteredCapability>,
}

impl Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a domain pack with full metadata and profile.
    pub fn register_pack_with_profile(
        &mut self,
        name: impl Into<String>,
        agents: &[AgentMeta],
        invariants: &[InvariantMeta],
        profile: &PackProfile,
    ) {
        let name = name.into();
        let fact_prefixes: Vec<String> = agents
            .iter()
            .map(|a| a.fact_prefix.to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let agent_names = agents.iter().map(|a| a.name.to_string()).collect();
        let invariant_names = invariants.iter().map(|i| i.name.to_string()).collect();
        let description = agents
            .iter()
            .map(|a| a.description)
            .collect::<Vec<_>>()
            .join("; ");

        let context_keys_read: Vec<ContextKey> = agents
            .iter()
            .flat_map(|a| a.dependencies.iter().copied())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let context_keys_written: Vec<ContextKey> = agents
            .iter()
            .map(|a| a.target_key)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let has_acceptance_invariants = invariants
            .iter()
            .any(|i| i.class == InvariantClass::Acceptance);

        self.packs.push(RegisteredPack {
            name,
            description,
            fact_prefixes,
            agent_names,
            invariant_names,
            agent_count: agents.len(),
            invariant_count: invariants.len(),
            context_keys_read,
            context_keys_written,
            has_acceptance_invariants,
            profile: profile.clone(),
        });
    }

    /// Register a domain pack (without profile — uses default).
    pub fn register_pack(
        &mut self,
        name: impl Into<String>,
        agents: &[AgentMeta],
        invariants: &[InvariantMeta],
    ) {
        self.register_pack_with_profile(name, agents, invariants, &PackProfile::default());
    }

    /// Register a pack directly from a raw struct.
    pub fn register_pack_raw(&mut self, pack: RegisteredPack) {
        self.packs.push(pack);
    }

    /// Register an available capability.
    pub fn register_capability(&mut self, name: impl Into<String>, description: impl Into<String>) {
        self.capabilities.push(RegisteredCapability {
            name: name.into(),
            description: description.into(),
        });
    }

    #[must_use]
    pub fn packs_for_prefix(&self, prefix: &str) -> Vec<&RegisteredPack> {
        self.packs
            .iter()
            .filter(|p| p.fact_prefixes.iter().any(|fp| fp == prefix))
            .collect()
    }

    #[must_use]
    pub fn packs_for_prefixes(&self, prefixes: &[&str]) -> Vec<&RegisteredPack> {
        self.packs
            .iter()
            .filter(|p| {
                p.fact_prefixes
                    .iter()
                    .any(|fp| prefixes.contains(&fp.as_str()))
            })
            .collect()
    }

    #[must_use]
    pub fn packs_for_entity(&self, entity: &str) -> Vec<&RegisteredPack> {
        let entity = entity.to_lowercase();
        self.packs
            .iter()
            .filter(|p| p.profile.entities.iter().any(|e| *e == entity))
            .collect()
    }

    #[must_use]
    pub fn packs_for_keyword(&self, keyword: &str) -> Vec<&RegisteredPack> {
        let keyword = keyword.to_lowercase();
        self.packs
            .iter()
            .filter(|p| p.profile.keywords.iter().any(|k| *k == keyword))
            .collect()
    }

    #[must_use]
    pub fn packs_writing_key(&self, key: ContextKey) -> Vec<&RegisteredPack> {
        self.packs
            .iter()
            .filter(|p| p.context_keys_written.contains(&key))
            .collect()
    }

    #[must_use]
    pub fn packs_handling_irreversible(&self) -> Vec<&RegisteredPack> {
        self.packs
            .iter()
            .filter(|p| p.profile.handles_irreversible && p.has_acceptance_invariants)
            .collect()
    }

    #[must_use]
    pub fn packs_requiring_capability(&self, capability: &str) -> Vec<&RegisteredPack> {
        self.packs
            .iter()
            .filter(|p| p.profile.required_capabilities.contains(&capability))
            .collect()
    }

    #[must_use]
    pub fn search_packs(&self, query: &str) -> Vec<&RegisteredPack> {
        let query = query.to_lowercase();
        self.packs
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.description.to_lowercase().contains(&query)
                    || p.profile.keywords.iter().any(|k| k.contains(&query))
                    || p.profile.entities.iter().any(|e| e.contains(&query))
            })
            .collect()
    }

    #[must_use]
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.iter().any(|c| c.name == name)
    }

    #[must_use]
    pub fn search_capabilities(&self, query: &str) -> Vec<&RegisteredCapability> {
        let query = query.to_lowercase();
        self.capabilities
            .iter()
            .filter(|c| {
                c.name.to_lowercase().contains(&query)
                    || c.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    #[must_use]
    pub fn packs(&self) -> &[RegisteredPack] {
        &self.packs
    }

    #[must_use]
    pub fn capabilities(&self) -> &[RegisteredCapability] {
        &self.capabilities
    }
}

// ── Structural Resolver (multi-dimension) ──────────────────────────

/// Level 2 resolver with 10 matching dimensions:
///
/// 1. Fact prefix matching
/// 2. Constraint → invariant matching
/// 3. Context key flow matching
/// 4. Domain entity matching
/// 5. Keyword matching
/// 6. Reversibility matching
/// 7. Forbidden action filtering
/// 8. Capability affinity
/// 9. HITL requirement matching
/// 10. Blueprint expansion (TODO)
pub struct StructuralResolver<'a> {
    registry: &'a Registry,
}

const STRONG_CONTEXT_FLOW_ANCHOR_CONFIDENCE: f64 = 0.75;

impl<'a> StructuralResolver<'a> {
    #[must_use]
    pub fn new(registry: &'a Registry) -> Self {
        Self { registry }
    }
}

#[allow(clippy::too_many_lines)]
impl IntentResolver for StructuralResolver<'_> {
    fn level(&self) -> ResolutionLevel {
        ResolutionLevel::Structural
    }

    fn resolve(
        &self,
        intent: &organism_intent::IntentPacket,
        current: &IntentBinding,
    ) -> IntentBinding {
        let mut binding = current.clone();
        let already_bound: std::collections::HashSet<String> =
            binding.packs.iter().map(|p| p.pack_name.clone()).collect();

        let mut matched: Vec<(String, String, f64)> = Vec::new(); // (pack, reason, confidence)

        // ── Dimension 1: Fact prefix matching ──────────────────────
        let prefixes = extract_fact_prefixes(&intent.context);
        for prefix in &prefixes {
            for pack in self.registry.packs_for_prefix(prefix) {
                if !already_bound.contains(&pack.name) {
                    matched.push((
                        pack.name.clone(),
                        format!("fact prefix '{prefix}' → pack '{}'", pack.name),
                        0.9,
                    ));
                }
            }
        }

        // ── Dimension 2: Constraint → invariant matching ───────────
        for constraint in &intent.constraints {
            for pack in &self.registry.packs {
                if pack.invariant_names.iter().any(|i| constraint.contains(i))
                    && !already_bound.contains(&pack.name)
                {
                    matched.push((
                        pack.name.clone(),
                        format!("constraint '{constraint}' → invariant in '{}'", pack.name),
                        0.85,
                    ));
                }
            }
        }

        // ── Dimension 4: Domain entity matching ────────────────────
        let entities = extract_entities(&intent.outcome);
        for entity in &entities {
            for pack in self.registry.packs_for_entity(entity) {
                if !already_bound.contains(&pack.name) {
                    matched.push((
                        pack.name.clone(),
                        format!("entity '{entity}' → pack '{}'", pack.name),
                        0.75,
                    ));
                }
            }
        }

        // ── Dimension 5: Keyword matching ──────────────────────────
        let keywords = extract_keywords(&intent.outcome);
        for keyword in &keywords {
            for pack in self.registry.packs_for_keyword(keyword) {
                if !already_bound.contains(&pack.name) {
                    matched.push((
                        pack.name.clone(),
                        format!("keyword '{keyword}' → pack '{}'", pack.name),
                        0.65,
                    ));
                }
            }
        }

        // ── Dimension 6: Reversibility matching ────────────────────
        if intent.reversibility == organism_intent::Reversibility::Irreversible {
            for pack in self.registry.packs_handling_irreversible() {
                if !already_bound.contains(&pack.name) {
                    matched.push((
                        pack.name.clone(),
                        format!(
                            "irreversible intent → pack '{}' has Acceptance invariants",
                            pack.name
                        ),
                        0.7,
                    ));
                }
            }
        }

        // ── Dimension 3: Context key flow matching ─────────────────
        // Context keys are useful only when they extend a stronger anchor.
        // Avoid global fan-out like "all packs writing Evaluations".
        let needed_keys = extract_context_keys(&intent.context);
        let anchored_pack_names = already_bound
            .iter()
            .cloned()
            .chain(
                matched
                    .iter()
                    .filter(|(_, _, confidence)| {
                        *confidence >= STRONG_CONTEXT_FLOW_ANCHOR_CONFIDENCE
                    })
                    .map(|(pack_name, _, _)| pack_name.clone()),
            )
            .collect::<std::collections::HashSet<_>>();
        let anchored_packs = anchored_pack_names
            .iter()
            .filter_map(|pack_name| {
                self.registry
                    .packs
                    .iter()
                    .find(|pack| &pack.name == pack_name)
            })
            .collect::<Vec<_>>();

        let anchor_written_keys = anchored_packs
            .iter()
            .flat_map(|pack| pack.context_keys_written.iter().copied())
            .filter(|key| needed_keys.contains(key))
            .collect::<std::collections::HashSet<_>>();

        for pack in &anchored_packs {
            for key in pack
                .context_keys_written
                .iter()
                .filter(|key| needed_keys.contains(key))
            {
                if !already_bound.contains(&pack.name) {
                    matched.push((
                        pack.name.clone(),
                        format!(
                            "anchored context flow → pack '{}' writes needed {key:?}",
                            pack.name
                        ),
                        0.78,
                    ));
                }
            }
        }

        if needed_keys.len() >= 2 && !anchor_written_keys.is_empty() {
            for pack in &self.registry.packs {
                if already_bound.contains(&pack.name) || anchored_pack_names.contains(&pack.name) {
                    continue;
                }

                let Some(read_key) = pack
                    .context_keys_read
                    .iter()
                    .copied()
                    .find(|key| anchor_written_keys.contains(key))
                else {
                    continue;
                };
                let Some(write_key) = pack
                    .context_keys_written
                    .iter()
                    .copied()
                    .find(|key| needed_keys.contains(key) && *key != read_key)
                else {
                    continue;
                };

                matched.push((
                    pack.name.clone(),
                    format!("context flow bridge {read_key:?} → {write_key:?} from anchored packs"),
                    0.72,
                ));
            }
        }

        // ── Dimension 7: Forbidden action filtering ────────────────
        let forbidden_keywords: Vec<String> = intent
            .forbidden
            .iter()
            .map(|f| f.action.to_lowercase())
            .collect();
        matched.retain(|(pack_name, _, _)| {
            if let Some(pack) = self.registry.packs.iter().find(|p| &p.name == pack_name) {
                !pack.profile.keywords.iter().any(|k| {
                    forbidden_keywords
                        .iter()
                        .any(|f| f.contains(k) || k.contains(f.as_str()))
                })
            } else {
                true
            }
        });

        // ── Dimension 8: Capability affinity ───────────────────────
        // If ANY bound pack (declarative or matched) requires capabilities, add them.
        let mut needed_capabilities: Vec<(String, String)> = Vec::new();
        let all_pack_names: Vec<String> = already_bound
            .iter()
            .chain(matched.iter().map(|(name, _, _)| name))
            .cloned()
            .collect();
        for pack_name in &all_pack_names {
            if let Some(pack) = self.registry.packs.iter().find(|p| &p.name == pack_name) {
                for cap in pack.profile.required_capabilities {
                    if !binding.capabilities.iter().any(|c| c.capability == *cap)
                        && !needed_capabilities.iter().any(|(c, _)| c == *cap)
                    {
                        needed_capabilities.push((
                            (*cap).to_string(),
                            format!("required by pack '{pack_name}'"),
                        ));
                    }
                }
            }
        }

        // Deduplicate: keep highest confidence per pack
        let mut best: std::collections::HashMap<String, (String, f64)> =
            std::collections::HashMap::new();
        for (pack_name, reason, confidence) in matched {
            let entry = best
                .entry(pack_name.clone())
                .or_insert((reason.clone(), 0.0));
            if confidence > entry.1 {
                *entry = (reason, confidence);
            }
        }

        for (pack_name, (reason, confidence)) in best {
            binding.packs.push(PackRequirement {
                pack_name,
                reason,
                confidence,
                source: ResolutionLevel::Structural,
            });
        }

        for (cap, reason) in needed_capabilities {
            binding
                .capabilities
                .push(organism_intent::resolution::CapabilityRequirement {
                    capability: cap,
                    reason,
                    confidence: 0.85,
                    source: ResolutionLevel::Structural,
                });
        }

        if !binding
            .resolution
            .levels_attempted
            .contains(&ResolutionLevel::Structural)
        {
            binding
                .resolution
                .levels_attempted
                .push(ResolutionLevel::Structural);
            binding
                .resolution
                .levels_contributed
                .push(ResolutionLevel::Structural);
        }

        binding
    }
}

// ── Extraction helpers ─────────────────────────────────────────────

fn extract_fact_prefixes(context: &serde_json::Value) -> Vec<String> {
    let mut prefixes = Vec::new();
    collect_prefixes(context, &mut prefixes);
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn collect_prefixes(value: &serde_json::Value, prefixes: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => {
            if let Some(colon) = s.find(':') {
                let candidate = &s[..=colon];
                if candidate.len() >= 3
                    && candidate.chars().next().is_some_and(char::is_alphabetic)
                {
                    prefixes.push(candidate.to_string());
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_prefixes(v, prefixes);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, v) in map {
                if key.ends_with(':') {
                    prefixes.push(key.clone());
                }
                collect_prefixes(v, prefixes);
            }
        }
        _ => {}
    }
}

fn extract_context_keys(context: &serde_json::Value) -> Vec<ContextKey> {
    let mut keys = Vec::new();
    // Only match explicit ContextKey references in structured fields,
    // not incidental word matches in free text.
    if let Some(obj) = context.as_object() {
        for key in obj.keys() {
            let k = key.to_lowercase();
            if k == "evaluations" || k == "evaluation" {
                keys.push(ContextKey::Evaluations);
            }
            if k == "strategies" || k == "strategy" {
                keys.push(ContextKey::Strategies);
            }
            if k == "proposals" || k == "proposal" {
                keys.push(ContextKey::Proposals);
            }
            if k == "constraints" || k == "constraint" {
                keys.push(ContextKey::Constraints);
            }
            if k == "signals" || k == "signal" {
                keys.push(ContextKey::Signals);
            }
        }
    }
    keys.dedup();
    keys
}

fn extract_entities(outcome: &str) -> Vec<String> {
    let outcome = outcome.to_lowercase();
    let known_entities = [
        "lead",
        "vendor",
        "contract",
        "employee",
        "expense",
        "deal",
        "partner",
        "subscription",
        "asset",
        "ticket",
        "campaign",
        "feature",
        "release",
        "incident",
        "policy",
        "approval",
        "budget",
        "team",
        "persona",
        "skill",
        "credential",
        "patent",
        "signal",
        "hypothesis",
        "experiment",
    ];
    known_entities
        .iter()
        .filter(|e| outcome.contains(**e))
        .map(|e| (*e).to_string())
        .collect()
}

const STOP_WORDS: &[&str] = &[
    "this", "that", "with", "from", "have", "your", "into", "about", "would", "there", "their",
    "they", "them", "when", "what", "where", "which", "shall", "could", "should", "will", "been",
    "does", "done", "each", "every", "some", "than", "then", "also", "just", "only", "most",
    "such", "very", "more", "over", "under", "after", "before", "between", "through", "during",
    "without", "within", "along", "across", "against", "around", "upon", "onto", "produce",
    "process", "create", "update", "manage", "handle", "ensure", "track", "check", "verify",
    "analyze", "review", "prepare", "complete",
];

fn extract_keywords(outcome: &str) -> Vec<String> {
    outcome
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() >= 4)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty() && !STOP_WORDS.contains(&w.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use organism_domain::packs;
    use organism_intent::resolution::DeclarativeBinding;
    use organism_intent::{ForbiddenAction, IntentPacket, Reversibility};

    fn full_registry() -> Registry {
        let mut r = Registry::new();
        r.register_pack_with_profile(
            "customers",
            packs::customers::AGENTS,
            packs::customers::INVARIANTS,
            &packs::customers::PROFILE,
        );
        r.register_pack_with_profile(
            "legal",
            packs::legal::AGENTS,
            packs::legal::INVARIANTS,
            &packs::legal::PROFILE,
        );
        r.register_pack_with_profile(
            "autonomous_org",
            packs::autonomous_org::AGENTS,
            packs::autonomous_org::INVARIANTS,
            &packs::autonomous_org::PROFILE,
        );
        r.register_pack_with_profile(
            "partnerships",
            packs::partnerships::AGENTS,
            packs::partnerships::INVARIANTS,
            &packs::partnerships::PROFILE,
        );
        r.register_pack_with_profile(
            "people",
            packs::people::AGENTS,
            packs::people::INVARIANTS,
            &packs::people::PROFILE,
        );
        r.register_pack_with_profile(
            "procurement",
            packs::procurement::AGENTS,
            packs::procurement::INVARIANTS,
            &packs::procurement::PROFILE,
        );
        r.register_pack_with_profile(
            "linkedin_research",
            packs::linkedin_research::AGENTS,
            packs::linkedin_research::INVARIANTS,
            &packs::linkedin_research::PROFILE,
        );
        r.register_pack_with_profile(
            "knowledge",
            packs::knowledge::AGENTS,
            packs::knowledge::INVARIANTS,
            &packs::knowledge::PROFILE,
        );
        r.register_pack_with_profile(
            "growth_marketing",
            packs::growth_marketing::AGENTS,
            packs::growth_marketing::INVARIANTS,
            &packs::growth_marketing::PROFILE,
        );
        r.register_pack_with_profile(
            "ops_support",
            packs::ops_support::AGENTS,
            packs::ops_support::INVARIANTS,
            &packs::ops_support::PROFILE,
        );
        r.register_pack_with_profile(
            "performance",
            packs::performance::AGENTS,
            packs::performance::INVARIANTS,
            &packs::performance::PROFILE,
        );
        r.register_pack_with_profile(
            "product_engineering",
            packs::product_engineering::AGENTS,
            packs::product_engineering::INVARIANTS,
            &packs::product_engineering::PROFILE,
        );
        r.register_pack_with_profile(
            "virtual_teams",
            packs::virtual_teams::AGENTS,
            packs::virtual_teams::INVARIANTS,
            &packs::virtual_teams::PROFILE,
        );
        r.register_pack_with_profile(
            "reskilling",
            packs::reskilling::AGENTS,
            packs::reskilling::INVARIANTS,
            &packs::reskilling::PROFILE,
        );
        r.register_capability("web", "URL capture and metadata extraction");
        r.register_capability("ocr", "Document understanding");
        r.register_capability("linkedin", "Professional network research");
        r.register_capability("social", "Social profile extraction");
        r
    }

    fn intent(outcome: &str) -> IntentPacket {
        IntentPacket::new(outcome, chrono::Utc::now() + chrono::Duration::hours(1))
    }

    // ── Dimension 1: Fact prefix ───────────────────────────────────

    #[test]
    fn dim1_fact_prefix_matches_pack() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let i = intent("process lead").with_context(serde_json::json!({ "ref": "lead:abc-123" }));
        let binding = resolver.resolve(&i, &IntentBinding::default());
        assert!(
            binding.packs.iter().any(|p| p.pack_name == "customers"),
            "should match customers from lead: prefix"
        );
    }

    // ── Dimension 2: Constraint → invariant ────────────────────────

    #[test]
    fn dim2_constraint_matches_invariant() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let mut i = intent("execute contract");
        i.constraints = vec!["signature_required".into()];
        let binding = resolver.resolve(&i, &IntentBinding::default());
        assert!(
            binding.packs.iter().any(|p| p.pack_name == "legal"),
            "should match legal from signature_required constraint"
        );
    }

    // ── Dimension 3: Context key flow ─────────────────────────────

    #[test]
    fn dim3_context_keys_without_anchor_do_not_globally_fan_out() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let i = intent("aggregate findings into recommendation").with_context(serde_json::json!({
            "evaluations": ["score:a", "score:b"],
            "strategies": "final recommendation needed"
        }));
        let binding = resolver.resolve(&i, &IntentBinding::default());

        assert!(
            !binding
                .packs
                .iter()
                .any(|pack| pack.reason.contains("context flow")),
            "context keys alone should not add context-flow matches"
        );
    }

    #[test]
    fn dim3_context_keys_extend_only_anchored_flow() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let i = intent("aggregate vendor scores into strategy").with_context(serde_json::json!({
            "evaluations": ["price:vendor-a", "compliance:vendor-a"],
            "strategies": "final recommendation needed"
        }));
        let binding = resolver.resolve(&i, &IntentBinding::default());
        let pack_names = binding
            .packs
            .iter()
            .map(|pack| pack.pack_name.as_str())
            .collect::<std::collections::HashSet<_>>();

        assert!(
            pack_names.contains("procurement"),
            "vendor entity should anchor procurement"
        );
        assert!(
            pack_names.contains("partnerships"),
            "vendor entity should anchor partnerships"
        );
        assert!(
            pack_names.contains("linkedin_research"),
            "anchored Evaluations → Strategies flow should add linkedin_research"
        );
        assert!(
            pack_names.contains("knowledge"),
            "anchored Evaluations → Strategies flow should add knowledge"
        );
        assert!(
            pack_names.contains("reskilling"),
            "anchored Evaluations → Strategies flow should add reskilling"
        );
        assert!(
            !pack_names.contains("ops_support"),
            "unanchored packs writing Evaluations should not be added"
        );
        assert!(
            !pack_names.contains("virtual_teams"),
            "unanchored packs writing Evaluations should not be added"
        );
        let weak_keyword_matches = binding
            .packs
            .iter()
            .filter(|pack| {
                pack.pack_name == "product_engineering" || pack.pack_name == "performance"
            })
            .collect::<Vec<_>>();
        assert!(
            weak_keyword_matches
                .iter()
                .all(|pack| !pack.reason.contains("context flow")),
            "weak keyword matches must not be upgraded into context-flow matches"
        );
    }

    // ── Dimension 4: Domain entity ─────────────────────────────────

    #[test]
    fn dim4_entity_matches_pack() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let i = intent("evaluate this vendor for compliance");
        let binding = resolver.resolve(&i, &IntentBinding::default());
        assert!(
            binding.packs.iter().any(|p| p.pack_name == "partnerships"),
            "should match partnerships from 'vendor' entity"
        );
    }

    // ── Dimension 5: Keyword ───────────────────────────────────────

    #[test]
    fn dim5_keyword_matches_pack() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let i = intent("plan the next marketing campaign for Q3");
        let binding = resolver.resolve(&i, &IntentBinding::default());
        assert!(
            binding
                .packs
                .iter()
                .any(|p| p.pack_name == "growth_marketing"),
            "should match growth_marketing from 'campaign' keyword"
        );
    }

    #[test]
    fn dim5_keyword_does_not_match_pack_descriptions() {
        let r = full_registry();
        assert!(
            r.packs_for_keyword("strategy").is_empty(),
            "description substrings should not count as keyword matches"
        );
        assert!(
            r.packs_for_keyword("aggregate").is_empty(),
            "description substrings should not count as keyword matches"
        );
    }

    // ── Dimension 6: Reversibility ─────────────────────────────────

    #[test]
    fn dim6_irreversible_adds_governance_packs() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let i = intent("terminate employee access").with_reversibility(Reversibility::Irreversible);
        let binding = resolver.resolve(&i, &IntentBinding::default());
        let governance_packs: Vec<_> = binding
            .packs
            .iter()
            .filter(|p| p.reason.contains("irreversible"))
            .collect();
        assert!(
            !governance_packs.is_empty(),
            "irreversible intent should add governance packs"
        );
    }

    // ── Dimension 7: Forbidden filtering ───────────────────────────

    #[test]
    fn dim7_forbidden_actions_filter_packs() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        let mut i = intent("research this lead but no linkedin outreach");
        i.forbidden = vec![ForbiddenAction {
            action: "linkedin".into(),
            reason: "not authorized for external contact".into(),
        }];
        i = i.with_context(serde_json::json!({ "ref": "lead:abc" }));
        let binding = resolver.resolve(&i, &IntentBinding::default());
        assert!(
            !binding
                .packs
                .iter()
                .any(|p| p.pack_name == "linkedin_research"),
            "linkedin_research should be filtered out by forbidden action"
        );
    }

    // ── Dimension 8: Capability affinity ───────────────────────────

    #[test]
    fn dim8_pack_adds_required_capabilities() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        // linkedin_research requires linkedin + web + social capabilities
        let binding = DeclarativeBinding::new()
            .pack("linkedin_research", "research leads")
            .build();
        let binding = resolver.resolve(&intent("research leads"), &binding);
        let cap_names: Vec<_> = binding
            .capabilities
            .iter()
            .map(|c| c.capability.as_str())
            .collect();
        assert!(
            cap_names.contains(&"linkedin"),
            "should add linkedin capability"
        );
        assert!(cap_names.contains(&"web"), "should add web capability");
        assert!(
            cap_names.contains(&"social"),
            "should add social capability"
        );
    }

    // ── Deduplication ──────────────────────────────────────────────

    #[test]
    fn deduplicates_packs_keeping_highest_confidence() {
        let r = full_registry();
        let resolver = StructuralResolver::new(&r);
        // Intent that matches customers via BOTH prefix and entity
        let i = intent("qualify this lead for the pipeline")
            .with_context(serde_json::json!({ "ref": "lead:abc" }));
        let binding = resolver.resolve(&i, &IntentBinding::default());
        let customer_matches: Vec<_> = binding
            .packs
            .iter()
            .filter(|p| p.pack_name == "customers")
            .collect();
        assert_eq!(customer_matches.len(), 1, "should deduplicate to one entry");
        assert!(
            customer_matches[0].confidence >= 0.75,
            "should keep highest confidence match"
        );
    }
}
