# Prior Consultation Policy on IntentPacket — Proposal

Status: **proposal**, not implemented. Seeking maintainer alignment before code.
Originating context: Atlas integration app + Mnemos PR #8 (`PriorConsultationPolicy`) + `marquee-apps/atlas-integration/kb/Architecture/Upstream Types Audit.md`.

## TL;DR

Mnemos PR #8 (merged) introduced `PriorConsultationPolicy` as the typed vocabulary for "how much should a run depend on relevant prior episodes." Mnemos owns the vocabulary, hosts decide enforcement. Today there is no typed field on `IntentPacket` for an intent to carry the policy — apps stuff a string into `IntentPacket.context["prior_consultation"]` as a workaround (see `marquee-apps/atlas-integration/crates/atlas-app/src/intent.rs:85`).

This proposal adds `pub prior_consultation: Option<PriorConsultationPolicy>` to `IntentPacket`, matching the existing precedent of `pub convergence: Option<ConvergenceCriteria>` and the existing dependency precedent of `organism-intent` consuming `prism` for fuzzy types.

## Motivation

### Current state

`IntentPacket` (`crates/intent/src/lib.rs:32`) carries typed policy enums today:

- `reversibility: Reversibility` — declared, not enforced; admission consults it
- `expiry_action: ExpiryAction` — declared, not enforced; expiry handling consults it
- `forbidden: Vec<ForbiddenAction>` — declared, not enforced; runtime consults it
- `convergence: Option<ConvergenceCriteria>` (added by PR #13) — declared, not enforced; formation runner consults it

These are *intent-side declarations* the executor must respect. `PriorConsultationPolicy` is structurally identical: an intent-side declaration that a recall executor (Mnemos) consults to decide whether to gate, advise, or skip prior-episode recall.

Atlas's workaround in `crates/atlas-app/src/intent.rs:85`:

```rust
.with_context(json!({
    // ...
    "prior_consultation": "require_when_available"
}))
```

This is stringly-typed. A typo (`"require_when_avilable"`) is silent. Mnemos's enforcement reads the JSON and parses it back into `PriorConsultationPolicy`, paying the cost of stringification twice and losing the compile-time guarantee at the boundary.

### Why this is the right next step

Three signals converge:

1. **Mnemos owns the vocabulary, by design.** From `mnemos/src/agentic/policy.rs:4-5`: *"Mnemos owns this vocabulary because it owns agentic memory and recall; host runtimes decide how to enforce it."* This proposal does not change ownership — it adds a typed field on `IntentPacket` that *carries* a Mnemos-owned type.
2. **Precedent exists.** `organism-intent/Cargo.toml:14` already depends on `prism` (a Mosaic extension) for fuzzy types. Adding `mnemos` as a dependency is the same pattern: Organism consumes vocabulary owned by a Mosaic extension.
3. **`convergence: Option<ConvergenceCriteria>` set the pattern in PR #13.** A typed, optional, intent-side policy field added without breaking serialization. This proposal mirrors it exactly.

## Proposed change

### `organism-intent/Cargo.toml`

```diff
 [dependencies]
 async-trait.workspace = true
 chrono.workspace = true
 converge-pack.workspace = true
+mnemos.workspace = true
 prism.workspace = true
 serde.workspace = true
 serde_json.workspace = true
 thiserror.workspace = true
 uuid.workspace = true
```

### `organism-intent/src/lib.rs`

```rust
pub use convergence::{ConvergenceCriteria, ConvergenceSignal};
pub use graded_admission::{DimensionRulebook, GradedAdmissionController};
pub use mnemos::agentic::PriorConsultationPolicy; // ← new re-export

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPacket {
    pub id: Uuid,
    pub outcome: String,
    pub context: serde_json::Value,
    pub constraints: Vec<String>,
    pub authority: Vec<String>,
    pub forbidden: Vec<ForbiddenAction>,
    pub reversibility: Reversibility,
    pub expires: DateTime<Utc>,
    pub expiry_action: ExpiryAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convergence: Option<ConvergenceCriteria>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_consultation: Option<PriorConsultationPolicy>, // ← new field
}

impl IntentPacket {
    // ... existing methods ...

    pub fn with_prior_consultation(mut self, policy: PriorConsultationPolicy) -> Self {
        self.prior_consultation = Some(policy);
        self
    }
}
```

### Serialization compatibility

`#[serde(default, skip_serializing_if = "Option::is_none")]` matches the existing `convergence` field's pattern:

- IntentPackets serialized before this change deserialize as `prior_consultation: None`
- IntentPackets with `None` serialize without the field present, byte-identical to today
- IntentPackets with `Some(policy)` add one field — additive, no breaking change

### Enforcement semantics (unchanged from Mnemos PR #8)

- `None` → executor uses `PriorConsultationPolicy::default()` which is `RequireWhenAvailable`
- `Some(RequireWhenAvailable)` → consult priors when available; continue without if recall is unavailable
- `Some(RequireForLoadBearingClaims)` → block promotion of load-bearing claims if recall is unavailable
- `Some(OptionalAdvisory)` → surface priors if recalled, never block
- `Some(IgnorePriors)` → explicit opt-out

Mnemos's `blocks_without_priors()` and `consults_priors()` methods already cover the enforcement decision; no Organism-side changes to enforcement code are required.

## Migration path

Three phases, each shippable:

1. **Land the field.** Add the `Option<PriorConsultationPolicy>` field, the `with_prior_consultation` builder, the `mnemos` dependency. Tests verify serialization round-trip with and without the field. (Single PR, ~30 lines + tests.)
2. **Atlas migrates.** Atlas's `intent.rs` replaces `"prior_consultation": "require_when_available"` in `context` with `.with_prior_consultation(PriorConsultationPolicy::RequireWhenAvailable)`. The context-JSON workaround dies in Atlas. (Atlas-side commit.)
3. **Mnemos enforcement reads the typed field.** Today Mnemos enforcement (if any exists yet) reads the context JSON. Future enforcement reads `intent.prior_consultation.unwrap_or_default()` directly. (Future Mnemos PR.)

Phases 1 and 2 unblock Atlas and remove the stringly-typed workaround. Phase 3 closes the loop when Mnemos enforcement matures.

## Architectural question — dependency direction

This proposal makes `organism-intent` depend on `mnemos`. That's the second time a bedrock-platform crate depends on a Mosaic extension (`prism` is the first). Worth naming the question explicitly:

| Direction | Today | After this proposal |
|---|---|---|
| `mnemos` → `converge` | Yes (Mnemos depends on Converge contracts) | Unchanged |
| `prism` → `converge` | Yes | Unchanged |
| `organism-intent` → `prism` | Yes (existing) | Unchanged |
| `organism-intent` → `mnemos` | No | **New** |

The Mosaic README declares: *"Converge contracts ← Mosaic extensions ← products / deployments."* Strictly read, `organism-intent → prism` already violates this. Pragmatically, fuzzy types and policy vocabularies are *contract-shaped*, not *implementation-shaped*, and live in the wrong crate today.

Three resolutions, in increasing order of effort:

- **A. Accept the precedent.** This proposal adds one dependency (`organism-intent → mnemos`) matching the existing one (`organism-intent → prism`). When/if the platform gains a third such dependency, revisit.
- **B. Move the *types* into bedrock-platform, leave the *implementations* in Mosaic.** Create a small `bedrock-platform/contracts` crate holding cross-cutting vocabularies: `MembershipDegree`, `MaterialityDegree`, `PriorConsultationPolicy`, future kin. Mosaic crates import from `contracts`; `organism-intent` imports from `contracts`. Dep direction stays clean.
- **C. Move the type into `organism-intent` directly.** Mnemos imports the policy from `organism-intent`. Reverses today's situation. Tempting because `PriorConsultationPolicy` is intent-shaped, but contradicts Mnemos's stated ownership (`mnemos/src/agentic/policy.rs:4-5`).

**Recommendation: A (this proposal as written), with B queued as a follow-up.** A unblocks Atlas now. B is the right long-term shape — but the right time to pay for it is when there are three or more cross-cutting vocabulary types, not two. Filing B as a separate proposal once a third candidate (e.g., a typed `Jurisdiction` or `ComplianceLevel`) appears in cross-app traffic gives the refactor real volume to justify itself.

C is wrong because Mnemos has explicitly claimed ownership and the doc comment is a design declaration, not a coincidence.

## Open questions

1. **Should the `IntentResolver` pipeline consult `prior_consultation` when matching formations?** E.g., if `IgnorePriors`, the resolver could deprioritize Mnemos-backed packs. Recommendation: **defer.** This proposal lands the field; the resolver integration is its own conversation.
2. **Should `RequireForLoadBearingClaims` block IntentPacket admission if no Mnemos backend is registered?** Today, admission is policy-blind to recall availability. Recommendation: **no — keep admission concerned with intent validity, not executor availability.** Runtime decides at enforcement time.
3. **Should `IntentPacket` gain a typed `Mnemos backend hint`?** No. The policy says *what is required*; backend selection is the platform's. Adding a backend-naming field would re-create the divergence Atlas just escaped.
4. **Wire-format precedent for re-exporting Mosaic types from `organism-intent`.** This proposal sets the pattern. Worth a one-paragraph note in `Dependency Rules.md` confirming "Organism intent may consume Mosaic vocabulary types — this is intentional, not a layering bug."

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep using `context["prior_consultation"]` as a string | Stringly-typed, no compile-time check, double parsing cost. The whole point of typed Intent is to avoid this. |
| Put the policy on `Reversibility` (extend the enum) | Conflates orthogonal concerns. Reversibility describes the action's nature; prior consultation describes evidentiary requirements. |
| Add as a sub-field on `convergence: ConvergenceCriteria` | Same conflation. Convergence is about *when to stop*; prior consultation is about *what to require before producing*. |
| Move `PriorConsultationPolicy` into `organism-intent` (option C above) | Contradicts Mnemos's documented ownership. |
| Wait for the `bedrock-platform/contracts` refactor (option B) | Blocks Atlas's typed-intent work on a larger refactor. The right scope of that refactor is unclear until there are three or more candidate types. |

## What this proposal does not do

- Doesn't change Mnemos's ownership of the policy vocabulary.
- Doesn't add enforcement code to Organism — enforcement remains where the work happens (in Mnemos and recall consumers).
- Doesn't propose the `bedrock-platform/contracts` crate (separate, larger proposal).
- Doesn't change the existing `prism` dependency or relitigate its placement.
- Doesn't change the `IntentResolver` pipeline.

## See also

- `marquee-apps/atlas-integration/kb/Architecture/Upstream Types Audit.md` — full audit identifying this gap.
- `marquee-apps/atlas-integration/crates/atlas-app/src/intent.rs` — current consumer using the context-string workaround.
- `stack/mosaic-extensions/mnemos-knowledge/crates/mnemos/src/agentic/policy.rs` — `PriorConsultationPolicy` definition.
- `crates/intent/src/lib.rs:32` — current `IntentPacket` shape.
- `crates/intent/src/convergence.rs` — `ConvergenceCriteria` precedent for the optional-typed-field pattern.
