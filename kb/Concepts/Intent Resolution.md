---
tags: [concepts]
---
# Intent Resolution

How Organism maps an intent to the resources needed for convergence.

```
Gherkin feature / natural language
    ↓
IntentPacket
    ↓
Admission (4 feasibility dimensions)
    ↓
Resolution (4 levels)
    ↓
IntentBinding { packs, capabilities, invariants }
    ↓
Engine configuration → Converge
```

## The Problem

An intent says WHAT should become true. But the Converge engine needs to know WHICH agents to register, WHICH capabilities to wire, WHICH invariants to enforce. Someone has to bridge that gap.

That someone is intent resolution.

## The Four Levels

### Level 1: Declarative

The app explicitly declares what it needs. This is the starting point — no intelligence, just configuration.

```gherkin
Feature: qualify-inbound-lead

  Given an inbound lead with company and contact data
  When the lead scoring pipeline runs
  Then the lead has an ICP fit score
  And the lead is routed to the appropriate sales owner
```

The app binds this to packs and capabilities:

```rust
use organism_pack::*;

let binding = DeclarativeBinding::new()
    .pack("customers", "lead enrichment, scoring, routing")
    .pack("linkedin_research", "enrich with professional network data")
    .capability("web", "capture company website for ICP signals")
    .capability("social", "extract LinkedIn profile metadata")
    .invariant("lead_has_source")
    .invariant("evidence_requires_provenance")
    .build();
```

**Who does it:** The app developer, at design time.
**Confidence:** 1.0 — the developer knows what they need.
**Limitation:** Manual, brittle, doesn't adapt.

### Level 2: Structural

The resolver inspects the intent's fact prefixes and context keys to find matching packs automatically. This is deterministic — no LLM, no guessing.

```rust
/// Structural resolver: match fact prefixes in the intent context
/// to packs that produce or consume those prefixes.
impl IntentResolver for StructuralResolver {
    fn level(&self) -> ResolutionLevel { ResolutionLevel::Structural }

    fn resolve(&self, intent: &IntentPacket, current: &IntentBinding) -> IntentBinding {
        let mut binding = current.clone();

        // Parse the intent context for fact references
        let context = &intent.context;
        let mentioned_prefixes = extract_fact_prefixes(context);

        // Match against pack metadata
        for pack in ALL_PACKS {
            for agent in pack.agents {
                if mentioned_prefixes.contains(agent.fact_prefix) {
                    binding.packs.push(PackRequirement {
                        pack_name: pack.name.into(),
                        reason: format!(
                            "agent {} produces facts with prefix {}",
                            agent.name, agent.fact_prefix
                        ),
                        confidence: 0.85,
                        source: ResolutionLevel::Structural,
                    });
                    break;
                }
            }
        }

        binding
    }
}
```

**Example:** Intent context mentions `"lead:"` facts → structural resolver finds `customers` pack (whose agents produce `lead:` prefixed facts). Intent mentions `"contract:"` → resolver adds `legal` pack.

**Who does it:** Organism, at runtime.
**Confidence:** 0.85 — fact prefix matching is deterministic but may over-include.
**Limitation:** Only works when the intent context contains structured fact references.

### Level 3: Semantic

The huddle (multi-model reasoning) matches the intent's natural language outcome against pack descriptions and capability summaries. This uses LLM reasoning.

```rust
/// Semantic resolver: use the huddle to match intent outcome
/// to relevant packs by meaning, not just structure.
impl IntentResolver for SemanticResolver {
    fn level(&self) -> ResolutionLevel { ResolutionLevel::Semantic }

    fn resolve(&self, intent: &IntentPacket, current: &IntentBinding) -> IntentBinding {
        let mut binding = current.clone();

        // Build a prompt from the intent outcome and available packs
        let pack_catalog = ALL_PACKS.iter()
            .map(|p| format!("{}: {}", p.name, p.description))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Given this intent:\n\n\"{}\"\n\n\
             Which of these packs are needed?\n\n{}\n\n\
             Return pack names and confidence (0.0-1.0).",
            intent.outcome, pack_catalog
        );

        // Huddle produces pack recommendations with confidence
        let recommendations = self.huddle.reason(&prompt);

        for rec in recommendations {
            // Don't add packs already bound by level 1 or 2
            if !binding.packs.iter().any(|p| p.pack_name == rec.pack_name) {
                binding.packs.push(PackRequirement {
                    pack_name: rec.pack_name,
                    reason: rec.rationale,
                    confidence: rec.confidence,
                    source: ResolutionLevel::Semantic,
                });
            }
        }

        // Also identify capability needs from the outcome
        let capability_prompt = format!(
            "Does this intent need any of these capabilities?\n\
             OCR, Vision, Web capture, Social extraction, LinkedIn, Patent search\n\n\
             Intent: \"{}\"",
            intent.outcome
        );

        let capabilities = self.huddle.reason(&capability_prompt);
        for cap in capabilities {
            binding.capabilities.push(CapabilityRequirement {
                capability: cap.name,
                reason: cap.rationale,
                confidence: cap.confidence,
                source: ResolutionLevel::Semantic,
            });
        }

        binding
    }
}
```

**Example:** Intent: "evaluate whether this vendor meets our compliance requirements" → semantic resolver matches `partnerships` (vendor assessment), `autonomous_org` (policy enforcement), and capability `web` (capture vendor website for compliance docs).

**Who does it:** Organism's huddle, at runtime. Uses LLM reasoning via Converge providers.
**Confidence:** 0.5–0.9 — depends on outcome clarity and pack catalog coverage.
**Limitation:** Non-deterministic, costs tokens, needs adversarial review of the resolution itself.

### Level 4: Learned

Prior calibration from execution history adjusts pack selection confidence. The system observes which packs were actually needed for similar intents and updates its priors.

```rust
/// Learned resolver: adjust confidence based on execution history.
/// Consults LearningEpisode and PriorCalibration records.
impl IntentResolver for LearnedResolver {
    fn level(&self) -> ResolutionLevel { ResolutionLevel::Learned }

    fn resolve(&self, intent: &IntentPacket, current: &IntentBinding) -> IntentBinding {
        let mut binding = current.clone();

        // Find similar past intents by outcome embedding
        let similar_episodes = self.episode_store
            .find_similar(&intent.outcome, 10);

        for episode in &similar_episodes {
            // Which packs did similar intents actually use?
            let used_packs = episode.actual_packs_used();
            let predicted_packs: Vec<_> = binding.packs.iter()
                .map(|p| p.pack_name.clone())
                .collect();

            // Add packs that were historically needed but not yet bound
            for pack in &used_packs {
                if !predicted_packs.contains(pack) {
                    binding.packs.push(PackRequirement {
                        pack_name: pack.clone(),
                        reason: format!(
                            "historically needed for similar intents ({} prior episodes)",
                            similar_episodes.len()
                        ),
                        confidence: episode.outcome_confidence * 0.8,
                        source: ResolutionLevel::Learned,
                    });
                }
            }

            // Adjust confidence of existing bindings based on history
            for pack in &mut binding.packs {
                if let Some(prior) = self.calibrations.get(&pack.pack_name) {
                    // Bayesian update: blend declared confidence with historical
                    pack.confidence = (pack.confidence + prior.posterior_confidence) / 2.0;
                }
            }
        }

        binding.resolution.prior_episodes_consulted = similar_episodes.len();
        binding
    }
}
```

**Example:** First time "approve entertainment expense" runs, level 1 binds `autonomous_org` + `procurement`. After 50 runs, the system learns that `compliance` pack was added manually in 80% of cases where amount > $1,000. Level 4 now automatically suggests `compliance` with confidence 0.64 for similar intents.

**Who does it:** Organism's learning system, using `LearningEpisode` and `PriorCalibration`.
**Confidence:** Starts low, compounds over time.
**Limitation:** Cold start — needs execution history. This is the moat.

## Resolution Chain

The runtime chains all four levels. Each level fills gaps left by the previous one:

```
Level 1 (declarative)  → baseline binding, confidence 1.0
Level 2 (structural)   → fill gaps from fact prefix matching, confidence 0.85
Level 3 (semantic)     → fill remaining gaps from LLM reasoning, confidence 0.5–0.9
Level 4 (learned)      → adjust all confidences from execution history
```

The final `IntentBinding` carries a `ResolutionTrace` showing which levels contributed, how many prior episodes were consulted, and overall completeness confidence.

```rust
// Resolution output
IntentBinding {
    packs: [
        PackRequirement { pack_name: "customers", confidence: 1.0,  source: Declarative },
        PackRequirement { pack_name: "legal",     confidence: 0.85, source: Structural },
        PackRequirement { pack_name: "compliance", confidence: 0.72, source: Learned },
    ],
    capabilities: [
        CapabilityRequirement { capability: "web", confidence: 0.8, source: Semantic },
    ],
    invariants: ["lead_has_source", "signature_required"],
    resolution: ResolutionTrace {
        levels_attempted: [Declarative, Structural, Semantic, Learned],
        levels_contributed: [Declarative, Structural, Semantic, Learned],
        prior_episodes_consulted: 47,
        completeness_confidence: 0.91,
    },
}
```

## Adversarial Review of Bindings

The binding itself can be challenged. An adversarial agent might say:

- **ConstraintChecking:** "binding includes `legal` pack but intent has no contract-related keywords — over-provisioning"
- **AssumptionBreaking:** "binding assumes `customers` pack but this is an internal HR intent"
- **EconomicSkepticism:** "5 packs for a simple expense — unnecessary compute cost"

This is the organism differentiator: even the resource allocation is stress-tested.

## The Flywheel

```
More intents → more episodes → better level 4 → fewer manual bindings
→ faster resolution → more intents processed → more episodes
```

This is the moat from the strategy doc: "the more organizations run on Organism, the better Organism gets at planning for organizations like them."

See also: [[Concepts/Intent Pipeline]], [[Concepts/Organizational Learning]], [[Architecture/Two-Sided Capabilities]]
