---
tags: [handoff, cross-repo, helms, organism-1.7.0, organism-1.8.0, organism-1.8.1, axiom-0.8.0]
source: human
date: 2026-05-07
target: helms
unblocks: helms's `truth-catalog` migration off hand-rolled IntentPackets; the
  tournament-of-templates orchestration that organism intentionally does
  not ship.
---

# Handoff to Helms — Organism 1.7.0 + 1.8.0 migration

When this lands in helms, two things happen: (1) helms stops hand-rolling
`IntentPacket`s and uses axiom's typed compiler; (2) helms gains the
"tournament-of-templates" orchestration that organism intentionally does
**not** ship. The split honours an architectural rule we converged on this
week:

> Organism = intelligent **selection** of pre-built suggestors. Compilation
> belongs in axiom. Orchestration with policy (cost ceilings, race vs.
> single-shot) belongs in helms.

---

## What changed upstream

### axiom-truth 0.8.0 (2026-05-07)

- New `intent` module: `axiom_truth::compile_intent(&TruthDocument) -> IntentPacket`
  and `compile_intent_from_source(&str) -> IntentPacket`. axiom now owns
  the bridge from Truth-shaped governance to organism's runtime contract.
- New dependency: `organism-pack` (axiom imports `IntentPacket`,
  `Reversibility`, `ForbiddenAction`, `ExpiryAction`).
- Architectural inversion: `axiom-truth → organism-pack`. organism no
  longer mentions Truth in any form.

### organism 1.7.0 (2026-05-07)

- **Removed:** `organism_intent::bridge` (along with `TruthInput` and the
  whole local Truth-shaped type family). `Runtime::resolve_and_admit_truth`
  is gone. `TruthAdmissionError::Bridge` variant is gone.
- **Added:** `Runtime::admit_intent(&IntentPacket, ...)` — the runtime
  takes a *pre-compiled* `IntentPacket` and stages it through Converge's
  typed admission boundary.
- **Added (Stage 3 selection):** `organism_intent::problem::ProblemClass`
  (7-class taxonomy + deterministic classifier),
  `organism_runtime::classifier::ProblemClassifierSuggestor` (in-loop
  refinement), `organism_runtime::templates::standard_formation_catalog()`
  (5 named templates: Decision / Research / Evaluation / Planning /
  Diligence), `organism_runtime::guru::FormationGuru::select` (returns
  primary + up to 2 alternates + `SelectionTrace`),
  `Runtime::select_formation` (auto-mode front half).

### organism 1.8.0 (2026-05-07, "Smarter selection")

- **Added:** capability-surplus + cost-aware composite scoring in
  `FormationGuru`. `SelectionTrace` exposes per-candidate `CandidateScore`
  breakdown (catalog rank, surplus, cost). New `templates::CostHint` enum
  and `cost_hint_for(template_id)` lookup.
- **Added:** `organism_intent::problem::ClassifierTiebreaker` Plug-Boundary
  trait. Async, no organism vendor adapter imports.
  `classify_with_tiebreaker(...)` consults the tiebreaker only when the
  keyword pass defaulted; degrades to deterministic default on error.
  `ProblemClassification.tiebroken: bool` surfaces whether the tiebreaker
  resolved the ambiguity.

  **Async surface — selection stays sync.** `Runtime::select_formation`
  and `FormationGuru::select` are deliberately **not** async — the guru
  classifies internally with the deterministic `classify()` and never
  takes a tiebreaker. Only the standalone helpers
  (`classify_with_tiebreaker`, `classify_text_with_tiebreaker`) are
  async. A helms `auto_run` that wants LLM tiebreaking has two shapes:
  (a) skip the tiebreaker and call `runtime.select_formation` directly
  (the cheap default), or (b) call `classify_with_tiebreaker` itself,
  then route via `template_id_for(class)` + a direct catalog lookup,
  bypassing the guru. Wrapping the guru is *not* an option in 1.8.x —
  if helms needs a tiebreaker-aware `select_formation_async`, surface it
  as a fresh organism deliverable, not a helms-side workaround.
- **Added:** `organism_runtime::stall::RoleStallSuggestor` — in-loop
  observation Suggestor that emits a `Diagnostic` recommendation when a
  watched role's ContextKey stays empty while convergence happens
  elsewhere. Idempotent. Observation only — host policy decides whether
  to act.

  **Consumer is helms's responsibility — not wired in 1.8.0.** The
  `Diagnostic` is dead telemetry until something acts on it. The
  recommended pattern: surface stall facts as
  `UserExperienceEvent::UserCorrection` (the runtime self-corrected
  mid-flight, see §"Audit trail integration") and feed them to
  Converge's experience store; long-term they become `PlanningPriorAgent`
  inputs. Re-selection on stall (drop the role, retry with an alternate
  template) is *not* in 1.8.0 — if helms wants it, that's a helms
  control-loop on top of the diagnostic emission, or a fresh organism
  deliverable depending on whether re-selection should be Suggestor- or
  orchestrator-driven.

### What organism intentionally does **not** ship

- A method that takes an intent, picks templates, compiles them all,
  instantiates them all, races them, and returns the winner. That's
  **orchestration with policy**, and it belongs here in helms.

---

## What helms does today (the path being deprecated)

`helms/crates/truth-catalog/src/organism.rs` builds an `IntentPacket`
directly out of in-repo `TruthDefinition` constants:

```rust
// crates/truth-catalog/src/organism.rs:41
fn build_binding(truth: TruthDefinition) -> Option<TruthOrganismBinding> {
    let (blueprint, intent, baseline, readiness) = organism_recipe(truth)?;
    let registry = organism_registry();
    let resolver = StructuralResolver::new(&registry);
    let binding = resolver.resolve(&intent, &baseline);
    // … readiness checks
}
```

`organism_recipe` constructs the `IntentPacket` field-by-field from the
`TruthDefinition` in helms's source tree. The Truth content that drives it
lives in helms, not in `.truths` files parsed at runtime.

This was always temporary — the comment thread in `Runtime::resolve_and_admit_truth`'s
old doc-block explicitly named it the "older `helms/truth-catalog/src/organism.rs`
path that the typed bridge replaces."

---

## The new flow helms should adopt

```rust
// 1. Parse the Truth source (axiom)
let truth   = axiom_truth::parse_truth_document(source)?;

// 2. Compile to IntentPacket (axiom)
let intent  = axiom_truth::compile_intent(&truth)?;
//    OR, one-shot from raw source:
//    let intent = axiom_truth::compile_intent_from_source(source)?;

// 3. Admit through the typed boundary (organism)
let receipt = runtime.admit_intent(&intent, actor, src, &mut ctx)?;

// 4. Pick a formation (organism — selection only)
let selection = runtime.select_formation(&intent, &catalog, &caps)?;

// 5. Compile + instantiate one or more candidates (organism mechanism;
//    helms decides how many)
let plan      = compiler.compile(req(&selection.primary), &compiler_cats)?;
let formation = executables.instantiate(&plan, seeds.clone())?;

// 6. Run a single formation OR race candidates — helms policy decides
let result    = formation.run().await?;
//    OR, racing primary + alternates:
//    let formations = build_all_candidates(&selection, …)?;
//    let tournament = FormationTournament::new(intent.id, plan_id, formations);
//    let result     = tournament.run().await?;
```

Note steps 5–6 are **the same building blocks organism already exposes** —
they just don't compose themselves. The orchestrator (helms) does.

---

## Migration sequence

A safe order, smallest steps first.

### Where helms is right now (verified 2026-05-07)

- `axiom-truth = { path = "../axiom", version = "0.8.1" }` ✓ already off
  the v0.6.0 git tag. Step 1 bump is partially done.
- `organism-{pack,runtime,intent,domain,intelligence,notes} = "1.7.0"`
  with path overrides ✓ at 1.7.0.
- Bump pending: 1.7.0 → 1.8.1 to pick up the smarter-selection surfaces.
  (1.8.1 is a docs-only patch on top of 1.8.0; the API surface is
  identical to 1.8.0. Pin to the patch so this handoff doc and the pin
  agree.)

### Steps

1. **Bump organism pins 1.7.0 → 1.8.1.** Trivial caret-relaxed; the path
   overrides resolve the local source either way. Confirms helms picks up
   `CandidateScore` / `CostHint` / `ClassifierTiebreaker` /
   `RoleStallSuggestor`. The 1.8.1 patch is docs-only on top of 1.8.0 —
   no API delta — so this collapses to a single version-string change.
2. **Replace one `organism_recipe` truth at a time.** For each existing
   `TruthDefinition`, write the Truth as a `.truths` source string (or read
   it from a file), pipe through `axiom_truth::compile_intent_from_source`,
   and assert the resulting `IntentPacket` matches what `organism_recipe`
   produced **per the equivalence rules below** (not literal `==`). Land
   tests proving equivalence before deleting the recipe.

   **Equivalence rules** — what counts as "the same packet":

   | Field | Rule |
   |---|---|
   | `id` | Excluded (always different — both sides mint UUIDs). |
   | `outcome` | Exact string. |
   | `expires` | RFC-3339 round-trip equality. Skip sub-second precision. |
   | `authority` | Compared as a set, not a list. |
   | `forbidden` | Compared as a set on `(action, reason)`. |
   | `constraints` | Compared as a set. |
   | `reversibility`, `expiry_action` | Exact. |
   | `context` (JSON) | Structural eq via `serde_json::Value` compare. |

   Helms should write a `intent_packet_equiv(a, b) -> Result<(), Diff>`
   helper that returns the first divergence with a human-readable diff.
   Tests assert `intent_packet_equiv(legacy, axiom_compiled).is_ok()`.

3. **Three likely escape-hatch failure modes during step 2** — pre-named
   so you don't hit them cold:

   a. **Synthesized forbidden actions.** `organism_recipe` may mint
      `ForbiddenAction`s that don't appear in the Truth source (e.g.,
      "approve_unverified_lead" hard-coded into the recipe). Fix:
      add the corresponding `Authority: Must Not:` line to the Truth
      source; equivalence test passes once the source carries them.

   b. **Stable list ordering.** `organism_recipe` produces deterministic
      list order; `compile_intent` produces whatever order the input
      gives. Fix: equivalence rules above already treat lists as sets
      where they're conceptually unordered. If a list must stay
      ordered, surface it during the migration test.

   c. **Default expiry / reversibility differs.** The recipe might
      hard-code a 90-day expiry; `compile_intent` defaults to 24h. Fix:
      the Truth source must specify `Authority: Expires:` explicitly
      whenever a default mismatch matters. Migration is the right time
      to make defaults explicit.
3. **Drop `organism_recipe` and `TruthDefinition`.** Once all truths are
   parsed via axiom, the in-repo definitions are dead weight.
4. **Adopt `Runtime::admit_intent` for the kernel staging step.** The
   in-repo readiness/probe wiring around `StructuralResolver` stays — it's
   already the "smart selection" surface helms wants. Just stop minting
   IntentPackets by hand.
5. **Add `select_formation` to the truth-catalog flow.** With axiom doing
   the parse and organism doing the admission + selection, helms's job
   shrinks to: register descriptors, choose the catalog, decide the seed
   set, drive `compile + instantiate + run`.
6. **(Optional, scope under "smart orchestration".)** Implement
   tournament-of-templates with cost guardrails. Spec below.

Each step is independently shippable; nothing in 1–5 requires (6) to
function.

---

## Tournament orchestration — the part organism doesn't ship

This belongs in helms because every choice is **policy**, not mechanism.

### The recipe

```rust
pub struct AutoRunOptions {
    /// Race the alternates? Default: false. Single-shot is the cheap path.
    pub race_alternates: bool,
    /// If racing, hard cap on candidates considered (incl. primary).
    pub max_candidates: usize,
    /// Skip alternates whose match score is below
    /// (primary_score * relative_cutoff).
    pub relative_cutoff: f64,
    /// Total budget across the race, per cost class. None = no cap.
    pub max_cost: Option<CostBudget>,
}
```

### Pseudocode

```rust
async fn auto_run(intent, catalog, ..opts) -> AutoRunResult {
    // Hard rule: irreversibles never race. See "Known sharp edges" §2.
    if intent.reversibility == Reversibility::Irreversible {
        return single_run_with_hitl(intent).await;
    }

    let selection = runtime.select_formation(&intent, &catalog, &caps)?;

    let candidates: Vec<&FormationTemplate> = if opts.race_alternates {
        // Defensive: clamp max_candidates to >= 1.
        let cap = opts.max_candidates.max(1);
        // Defensive: clamp relative_cutoff to (0.0, 1.0]. The primary must
        // never be filtered out by its own score.
        let cutoff = opts.relative_cutoff.clamp(0.0, 1.0);

        let mut v = vec![selection.primary];
        v.extend(selection.alternates.iter().take(cap - 1));

        // Use the catalog's per-candidate score (CandidateScore.composite,
        // 1.8.0). Filter alternates only — never the primary itself.
        let primary_score = selection.trace.scores[0].composite as f64;
        v = v.into_iter().enumerate().filter(|(idx, _)| {
            *idx == 0
                || (selection.trace.scores[*idx].composite as f64) >= primary_score * cutoff
        }).map(|(_, t)| t).collect();
        v
    } else {
        vec![selection.primary]
    };

    let mut formations = Vec::new();
    let mut excluded = Vec::new();
    for template in &candidates {
        match build_formation(template, intent, seeds.clone(), &compiler_cats, &executables) {
            Ok(formation)  => formations.push(formation),
            Err(reason)    => excluded.push((template.id().to_owned(), reason)),
        }
    }

    if formations.len() == 1 {
        return single_run(formations.pop().unwrap()).await;
    }
    let tournament = FormationTournament::new(intent.id, plan_id, formations);
    let result     = tournament.run().await?;
    Ok(AutoRunResult { selection: selection.trace, tournament: result, excluded })
}
```

### Cost guardrails — count-based, not per-step

Helms enforces cost ceilings via **candidate count** (`max_candidates`)
and **score cutoff** (`relative_cutoff`), not per-step LLM cost.

- `organism::FormationTournament` today does **not** expose per-step cost
  to the orchestrator. Per-Suggestor cost telemetry is a separate
  organism deliverable, **not** a 1.8.0 commitment.
- That means: don't draft helms code that assumes "cancel after $X
  spent." The available levers are coarser. Three guardrails:

  1. **Default to single-shot.** `race_alternates: false` is the sensible
     default. Auto-tournament is a power tool, not a free upgrade.
  2. **Skip near-duplicate alternates.** `relative_cutoff` filters
     alternates whose composite score is far below the primary. Use the
     `CandidateScore.composite` numbers from `SelectionTrace.scores`
     (1.8.0) to make this a real filter rather than guessing.
  3. **Surface partial failure.** A candidate that fails to compile or
     instantiate (no descriptor for a required role, etc.) should be
     **excluded from the race, not abort the run**. Return exclusions in
     `AutoRunResult` so audits can see why a candidate didn't compete.

`organism::FormationTournament` errors with `AllFailed` when every
formation crashes; "some compiled, some didn't" is a helms-side concern.

---

## Known sharp edges (read before committing to a step-6 ship date)

The migration is shippable through step 5 as written. Step 6 (auto-run
with tournament) has three architectural decisions that need to land
before helms can put a date on it.

### 1. Cost accounting — count-based, by design

The doc previously listed "cost ceilings on auto-tournaments" as both a
helms responsibility and an open question. Resolution: **helms enforces
count-based caps** (`max_candidates`, `relative_cutoff` against
`CandidateScore.composite`). Per-Suggestor cost observation is **not** a
1.8.0 commitment from organism. If helms needs spend-based caps later,
that becomes a fresh organism deliverable (probably an
`AgentEffect::with_cost_hint(...)` extension, scoped under "Stage 4
runtime maturity").

### 2. Tournaments forbidden for irreversible intents — full stop

Racing candidates means committing on a winner. If
`IntentPacket::reversibility == Reversibility::Irreversible`, racing
risks committing before a human has approved. **Hard rule: one and only
one execution path for irreversibles, even with cached HITL approval.**
The pseudocode above bails out before constructing candidates. This rule
should also be enforced **at the admission boundary** so it's not
bypassable from a non-tournament code path.

### 3. HITL gate placement — admission-boundary, not auto_run

The naive "put the gate inside auto_run before the tournament starts"
plan is bypassable: any path that calls `runtime.admit_intent` directly
skips the gate. Helms needs an ADR pinning where the gate lives. Two
viable shapes:

- **Single ingress.** Wrap `admit_intent` so every helms call to organism
  goes through `helms::admit_with_gates(intent, …)`. The wrapper checks
  reversibility + queries HITL approval, then calls
  `runtime.admit_intent`. Direct calls to `runtime.admit_intent` from
  helms are forbidden by linter / convention.

  *Tradeoff:* one chokepoint to audit; centralised enforcement is hard
  to bypass once the convention holds. **Cost:** couples HITL to the
  kernel admission boundary — every future organism admission entry
  point (e.g. an in-loop re-admit, a streaming admission API) has to
  preserve that wrapping or HITL silently leaks.

- **Pre-admission gate at the truth-catalog boundary.** Every Truth that
  becomes an `IntentPacket` flows through `truth-catalog`'s ingress;
  attach the HITL check there, before `compile_intent` is even called.

  *Tradeoff:* keeps the kernel pure — organism stays unaware of HITL,
  which preserves the layering. **Cost:** every new admission entry
  point has to remember the gate. The day helms grows a second ingress
  (a CLI tool, a webhook, a replay path), HITL is on the author of that
  entry point, not on the type system.

Helms picks one — eyes open on the cost, not just the centralisation
appeal. The losing option becomes deprecated. Either way, organism's
`Registry::packs_handling_irreversible` stops being "a hint" the moment
helms commits to enforcement somewhere.

### Audit trail integration (worth threading)

`AutoRunResult` carries `selection.trace` (per-candidate scores), the
tournament outcome, and `excluded` candidates. That data shape looks
exactly like the new `UserExperienceEvent` variants from the 2026-05-06
Converge handoff (`UserApprovalGranted`, `UserApprovalRejected`,
`UserOverrideIssued`, `UserCorrection`, `UserBoundaryAdjusted`).
Specifically:

- A guru selection rejected via cost cutoff → emit `UserOverrideIssued`
  with the alternate's id and the cutoff reason.
- A tournament where the alternate won → emit `UserCorrection` (the
  primary the system *wanted* lost to the alternate the system *also*
  proposed).
- An irreversible-intent path that hit HITL → emit `UserApprovalGranted`
  / `UserApprovalRejected` carrying the IntentPacket id.

Helms's `truth-catalog` is the audit layer; threading these events into
Converge's experience store closes the bidirectional learning loop and
makes future selections smarter (organism-learning's
`PlanningPriorAgent` already consumes these variants).

---

## Open architectural questions for helms

These are the genuine unknowns — items 3–4 above ("cost", "HITL gate")
have been promoted out of this list because they're *resolved* (not just
unanswered).

1. **Where does the host's `SuggestorDescriptorCatalog` live?** Today the
   atelier-showcase fixtures bootstrap one inline. Helms presumably wants
   a registered catalog spread across modules; pick a discoverable place
   (likely a new `helms-formation` crate alongside `truth-catalog`).
2. **Where does `ExecutableSuggestorCatalog` register factories?** Same
   answer. Provider handles (LLM clients, knowledge stores, Cedar policy
   engines) are real resources — they need a lifetime owner. Probably the
   helms application layer.

---

## Test strategy

- **Equivalence tests during step 2.** For each migrated `TruthDefinition`,
  assert the axiom-compiled `IntentPacket` is identical (modulo the random
  `IntentPacket::id`) to the legacy `organism_recipe` output. Drop the old
  recipe only when the equivalence holds.
- **Regression test for `select_formation`.** Pick three Truths spanning
  three problem classes; verify the guru returns the expected template
  ids. Atelier-showcase already has this exact test in
  `scenarios/truth-driven-formation` — copy/adapt for helms's catalog
  shape.
- **Tournament smoke test.** A single intent + 2 candidate templates
  built from mock descriptors that converge with different cycle counts;
  assert the higher-scoring formation wins. Use the existing
  `tournament::tests::ConvergingAgent` fixture as a model.

---

## What organism is committing to next (so helms can plan)

**Already shipped (1.8.0, 2026-05-07):**
- Capability-surplus + cost-aware template scoring (`CandidateScore`)
- LLM tiebreaker for the problem classifier (`ClassifierTiebreaker`)
- In-loop re-selection (Suggestor that flags stalled roles)

**Doc-only patch (1.8.1, 2026-05-07):**
- This handoff doc itself: async-surface callout, RoleStall consumer
  note, HITL ADR tradeoffs sketched per shape, per-role descriptor
  scoring promoted to a planning-input for helms's tournament design.
- No API delta vs 1.8.0; pin to 1.8.1 so the version on disk and the
  version of this doc agree.

**Deferred to a future cut:**
- Per-role descriptor scoring + per-role decisions in `SelectionTrace`.
  Touches `FormationCompiler` and `CompiledFormationPlan`. Splitting it
  out kept 1.8.0 backwards-compatible.

  **Planning input for helms tournament design.** If helms's tournament
  scoring assumes per-role decisions ("template X won because role Y's
  descriptor Z scored high"), redesign now around whole-template
  composites — only `CandidateScore.composite`, `catalog_rank`,
  `capability_surplus`, and `cost_hint` are exposed today. Don't build
  a per-role scoring workaround on the helms side: organism owns that
  surface and will likely ship it in 1.9.0, and a parallel helms
  implementation will need to be retired the moment organism does.

**Not committed (and should not be assumed):**
- Per-step Suggestor cost telemetry. If helms ends up needing
  spend-based caps, that's a fresh organism deliverable, not a hidden
  1.8.0 contract.

None of this changes the helms-side migration above; it just makes the
selection helms drives its tournament around smarter over time.

---

## Contact

Questions: open against organism repo with `tag:helms-migration`. The
migration above is reversible until step 3 (dropping `organism_recipe`);
ship 1–2 first, sit on it for a sprint, then continue.
