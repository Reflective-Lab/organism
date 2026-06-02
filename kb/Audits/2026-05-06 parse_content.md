---
tags: [audit, conformance, 3.8]
source: llm
date: 2026-05-06
audit-target: parse_content adoption at convergence boundary
status: applied where typed; documented where future work
---
# Audit — `parse_content` adoption at the convergence boundary

## Mandate

3.8 declaration item 4: *"Strong types before tests. If a string or number has semantics, make the type system carry that meaning before adding property tests."* And conformance sweep item: *"Use `Fact::parse_content()` / `ProposedFact::parse_content()` at the convergence boundary; no string parsing."*

The actual API in 3.8.0 is `parse_json_content::<T>()` on `ContextFact` and `ProposedFact`. Same semantics as the declaration's stated `parse_content` — just the JSON-specific name.

## Findings

30 manual JSON parse sites across the workspace. Two categories:

### Category A — typed deserialization (5 sites, migrated)

Sites where `serde_json::from_str::<ConcreteType>(fact.content())` deserializes into a defined struct. Migrating to `fact.parse_json_content::<ConcreteType>()` is purely cosmetic but signals intent: "this is the typed-boundary path, not arbitrary JSON traversal."

Migrated:

| Site | Type | Before | After |
|---|---|---|---|
| `runtime/src/huddle.rs:420` | `Disagreement` | `serde_json::from_str::<Disagreement>(fact.content())` | `fact.parse_json_content::<Disagreement>()` |
| `runtime/src/huddle.rs:429` | `Disagreement` | same | same |
| `runtime/src/huddle.rs:517` | `Vote` | `serde_json::from_str::<Vote>(fact.content())` | `fact.parse_json_content::<Vote>()` |
| `runtime/src/huddle.rs:526` | `Vote` | same | same |
| `runtime/src/huddle.rs:619, 673, 650, 1056` | `ConsensusOutcome`, `DisagreementMap` (test code) | `serde_json::from_str(...).unwrap()` | `fact.parse_json_content().unwrap()` |

### Category B — untyped `Value` parses (25 sites, deferred)

The bulk of parse sites deserialize into `serde_json::Value` and then traverse properties dynamically (`v["amount"].as_f64()`, `v.get("type").and_then(...)`). Migrating these to `parse_json_content::<Value>()` would be an empty rename — same string-keyed traversal, same lack of type safety.

**The real improvement requires defining typed structs first.** That is a much larger refactor than this audit's scope.

Sites by crate:

| Crate | Site | Current shape (anonymous schema) |
|---|---|---|
| `adversarial/src/agents.rs` | 4 sites | Plan annotations: `{annotation: {assumptions: [...], evidence: [...], impacts: [...], risks: [...]}}` |
| `simulation/src/{outcome,operational,causal,cost,policy}.rs` | 5 sites | Same plan-annotation shape as adversarial |
| `learning/src/adapter.rs` | 3 sites | Hypotheses/evaluations content with `description`, `category`, `is_infra_failure` flags |
| `learning/src/prior_agent.rs` | 1 site | Prior calibration seeds: `{type: "prior_calibration", calibration: {...}}` |
| `planning/src/dd.rs` | 4 sites | DD facts: `{title, url, content, provider, query}`, `{type, calibration}`, etc. |
| `planning/src/kb.rs` | 4 sites | Signal/proposal/evaluation facts written to KB pages |
| `planning/src/suggestor.rs` | 1 site | `{confidence: f64}` extraction from hypothesis content |

These are organized into roughly four anonymous schemas:

1. **`PlanAnnotation`** — `{annotation: {impacts, costs, risks, assumptions, evidence}}` — used in adversarial + all simulation agents.
2. **`PriorCalibrationSeed`** — `{type: "prior_calibration", calibration: PriorCalibration}` — used in learning.
3. **`DdSignalContent`** — `{title, url, content, provider, query}` — used in DD planning.
4. **`HypothesisPayload`** — `{description, category, confidence, is_infra_failure}` — used in adversarial output, learning input.

## Recommended future work

Define those four shapes as concrete `#[derive(Serialize, Deserialize)]` structs in the natural home crate, then migrate the 25 sites to `fact.parse_json_content::<Shape>()`. Each migration is local to its crate; they can be done independently.

Suggested home crates:
- `PlanAnnotation` → `organism-pack` (shared between adversarial and all simulation agents)
- `PriorCalibrationSeed` → `organism-learning` (already has `PriorCalibration`)
- `DdSignalContent` → `organism-planning::dd`
- `HypothesisPayload` → `organism-pack` (shared between adversarial output and learning input)

Each schema-to-struct migration is independently shippable and unblocks property tests. The 3.8 declaration's "strong types before tests" rule applies to each.

## What this audit did NOT do

- Did **not** define the four schemas as structs. That's deliberate scope-cut: defining them changes the JSON wire format consumers see and warrants its own design pass per schema.
- Did **not** touch sites where the parse target is already serde_json::Value-shaped intentionally (e.g., debugging passes, content-type-inference logic).

## Verification

- `cargo test --workspace`: ✓ all green after the 5 migrations.
- No regression in test count or behavior; the migration is byte-equivalent.

## Cross-references

- `~/dev/reflective/bedrock-platform/converge/crates/pack/src/fact.rs` — `parse_json_content` definition
- `kb/Concepts/Bidirectional ExperienceStore.md` — already uses typed deserialization for the user variants; serves as the pattern for future schema-to-struct work
- 3.8 declaration item 4 (in `~/dev/reflective/bedrock-platform/converge/`)
