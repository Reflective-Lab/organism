---
tags: [architecture]
---
# Two-Sided Capabilities

Organism has two kinds of reusable capabilities that sit alongside the planning loop. Both are consumed by apps (Wolfgang, Outcome Workbench, etc.) and orchestrated by the planning loop when multi-model reasoning is needed.

## Provider-Shaped Capabilities

Data acquisition from the world. API adapters that produce observations.

**Crate:** `intelligence`

| Module | What it does |
|---|---|
| OCR | Document understanding (Tesseract, Apple Vision, Mistral, DeepSeek, LightOn) |
| Vision | Scene understanding (Claude, GPT-4o, Gemini, Pixtral) |
| Web | URL capture and metadata extraction |
| Social | Normalized social profile extraction |
| LinkedIn | Professional network research |
| Patent | IP landscape, competitive intelligence |
| Billing | Stripe ACP integration |

**Pattern:** Each module defines a provider trait (e.g., `OcrProvider`, `VisionDescriber`), results wrapped in `Observation<T>` with provenance metadata. Implementations are feature-gated.

**Future:** `organism-notes` (vault, ingestion, cleanup, enrichment, indexing) will follow this pattern.

## Pack-Shaped Capabilities

Organizational workflow patterns. Agents and invariants that encode how organizations operate.

**Crate:** `domain`

| Pack | Lifecycle |
|---|---|
| knowledge | Signal → Hypothesis → Experiment → Decision → Canonical |
| customers | Lead → Enrich → Score → Route → Propose → Close → Handoff |
| people | Hire → Identity → Access → Onboard → Pay → Offboard |
| legal | Contract → Review → Sign → Execute |
| autonomous_org | Policy → Enforce → Approve → Budget → Delegate |
| ... | 9 more organizational packs |

**Pattern:** Each pack declares agents (with fact prefixes, dependencies, target keys) and invariants (with severity classes). When wired to Converge, agents implement `Suggestor` from `converge-pack`.

**Blueprints** compose multiple packs into end-to-end workflows: lead-to-cash, hire-to-retire, procure-to-pay, etc.

## Relationship to Converge Domain

| Layer | Packs | Character |
|---|---|---|
| converge-domain | trust, money, delivery, data_metrics | Foundational state machines |
| organism-domain | customers, people, legal, knowledge, ... | Organizational workflows |

Organism packs build on Converge foundational packs. A `customers::deal_closer` produces facts that feed into converge-domain's `money::invoice_creator`. Blueprints orchestrate this composition.

## Relationship to the Planning Loop

The planning loop (intent → adversarial → simulation → learning) **orchestrates** both capability types:

- "Read this document" → intelligence OCR
- "Should we close this deal?" → domain customers + adversarial review
- "What's the expected conversion value?" → simulation swarm

Apps can also consume capabilities **directly**, without the planning loop, for simple use cases.

See also: [[Architecture/Crate Map]], [[Architecture/Converge Contract]]
