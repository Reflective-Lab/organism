---
tags: [architecture, strategy]
---
# Remember Organism When

This page exists because new requirements will keep getting discovered in
`../*PROJECTS*`.

That is fine.

Prototype and pressure-test flows in apps first. Then consolidate them back into
Organism when they stop being product-specific and start looking like reusable
Layer 2 capability.

```
Apps → Organism → Converge
UX      reason      authority
```

If a thing sits between human intent and Converge's commit boundary, there is a
good chance it belongs in Organism.

## Put It In Organism When

- it interprets intent, decomposes work, or narrows scope before commit
- it plans, debates, challenges, simulates, or learns
- it is reusable across more than one app, workflow, or organization-shaped use case
- it expresses organizational workflow logic rather than product UX
- it should be available through `organism-pack`, `organism-runtime`, `organism-domain`, `organism-intelligence`, or `organism-notes`
- it is an app-discovered flow that has stabilized into a reusable organizational pattern

## Organism Owns These Packs

Organism owns the organizational workflow packs in `organism-domain`:

- `knowledge`
- `customers`
- `people`
- `legal`
- `performance`
- `autonomous_org`
- `growth_marketing`
- `product_engineering`
- `ops_support`
- `procurement`
- `partnerships`
- `virtual_teams`
- `linkedin_research`
- `reskilling`
- `due_diligence` (born from monterro + hackathon consolidation)

Organism also owns cross-pack blueprints when the flow is reusable:

- `lead_to_cash`
- `hire_to_retire`
- `procure_to_pay`
- `issue_to_resolution`
- `idea_to_launch`
- `campaign_to_revenue`
- `partner_to_value`
- `patent_research`
- `diligence_to_decision` (DueDiligence + Legal + Knowledge)

Rule:
- if it is a reusable organizational workflow with its own agents, invariants,
  fact prefixes, or cross-pack handoffs, it probably belongs in an Organism pack
  or blueprint

## What Does Not Belong In Organism

- product-specific screens, routes, APIs, jobs, and UX flows
- account-specific workflow tweaks, tenant policy overlays, and customer configuration
- one-off prompts, presentation formatting, report layouts, and brand voice
- app-local persistence models, caches, graph projections, and operational glue
- temporary experiments that have not yet proven reusable
- anything whose main purpose is kernel authority, fact promotion, or commit enforcement
- foundational state machines that any Converge consumer would need even without Organism

Organism is not the product layer, and it is not the kernel.

## What Belongs In Apps On Top Of Organism And Converge

Apps should own the product:

- UI, UX, APIs, CLI entrypoints, webhooks, jobs, and schedulers
- customer- or product-specific prompt framing
- feature flags, experiments, onboarding flows, and packaging
- app-specific integrations, persistence, graph models, and reporting
- domain workflows that are still being discovered and are not yet stable enough
  to generalize
- composition of Organism and Converge into a product experience

Good rule:
- start in the app when you are still learning
- move down into Organism when the logic becomes reusable

## What Belongs In Converge

Converge owns the general-purpose commit kernel:

- the convergence axioms
- the commit boundary and promotion gate
- authority, policy, and authorization primitives
- traceability, audit, and semantic truth handling
- foundational pack semantics and pack authoring contracts
- general kernel capabilities such as provider routing, storage, knowledge,
  experience, analytics, optimization, tools, and remote protocol

Converge also owns foundational packs that are not specifically about
organizational reasoning, for example:

- trust
- money
- delivery
- data and metrics

Good rule:
- if it must remain true for every system using Converge, it belongs in Converge
- if it is specific to organizational reasoning before commit, it belongs in Organism

## Live Example: Monterro → Organism (2026-04)

Monterro's convergent due diligence flow was built app-first:

1. **App-discovered patterns:** Five hand-rolled suggestors (breadth search, depth
   search, fact extraction, gap detection, synthesis), a hand-rolled budget tracker,
   and a planning seed that converted Organism plans into Converge strategy facts.
2. **Hackathon duplicated them:** The governance hackathon independently built the
   same patterns — budget, planning seed, gap detection, synthesis — with different
   domain payloads but identical structure.
3. **Consolidated to Organism:** The reusable patterns moved into `organism-planning`:
   - `HuddleSeedSuggestor` — seeds Organism plans as Converge strategy facts
   - `GapDetectorSuggestor` — debate-as-suggestor, reactive gap detection
   - `StabilitySuggestor` — fires when a context key stabilizes for N cycles
   - `SharedBudget` — cross-suggestor resource tracking with named limits
4. **Next step:** The five DD suggestors themselves (search, extract, detect, synthesize)
   move into `organism-domain/src/packs/due_diligence.rs` as a proper domain pack
   with generic callbacks for search and LLM. Monterro and hackathon become thin
   wiring layers that inject their backends.

This is the intended lifecycle: **app → pattern → organism crate → multiple apps reuse**.

## Consolidation Test

When something emerges from an app, ask:

1. Is this reusable across multiple apps or organizational workflows?
2. Is it below product UX but above Converge authority?
3. Does it encode planning, adversarial review, simulation, learning, or organizational pack logic?
4. Would another app want the same thing through a stable crate surface?
5. Is it more than a one-off prompt or local integration?

If the answer is mostly yes, move it into Organism.

## Practical Mapping

- reusable planning semantics: `organism-pack`
- reusable runtime embedding, resolution, readiness: `organism-runtime`
- reusable organizational workflows: `organism-domain`
- reusable world-facing capability adapters: `organism-intelligence`
- reusable vault and note lifecycle capability: `organism-notes`
- product experience and app-specific orchestration: app repo
- authority, truth, promotion, and foundational kernel infrastructure: Converge

See also: [[Philosophy/Relationship to Converge]], [[Architecture/API Surfaces]], [[Architecture/Crate Map]]
