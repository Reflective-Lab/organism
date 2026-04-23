---
tags: [building]
---
# Starter Task — Wire Converge Formation Integration

## Context

The organism pipeline does not end with a dumb commit submission. It ends by
assembling a `Formation` and running it in Converge.

- **Embedded:** use `converge-kernel` directly (primary path)
- **Remote:** use `converge-client` only when deployment requires out-of-process execution

## What to Build

### 1. Add Converge dependencies to runtime

```toml
[dependencies]
converge-pack = "3.7.3"
converge-model = "3.7.3"
converge-kernel = "3.7.3"
converge-provider-api = "3.7.3"
# converge-client = "3.7.3" # only when you truly need remote mode
```

Current Converge release for the formation compiler substrate: `3.7.3`.
Use workspace-pinned published crates, not sibling path dependencies, unless
you are deliberately coordinating a Converge change.

### 2. Implement embedded Formation execution

```rust
let formation = Formation::new("example")
    .agent(my_llm)
    .agent(my_policy_gate)
    .seed(ContextKey::Seeds, "intent-1", payload_json, "organism");

let result = formation.run().await?;
```

### 3. Write a test proving the axioms hold

- Construct a `ProposedFact` — verify it compiles
- Attempt to construct a `Fact` directly — verify it does NOT compile (trybuild)
- Run a formation through the engine — verify promotion gate runs

## What NOT to Build

- No wrapper types around Converge types — use them directly
- No separate crate for Converge integration — runtime owns it
- Do not depend on `converge-core` or other internal Converge crates
- Do not resurrect a `CommitBoundary` abstraction over embedded Converge execution
- Do not invent side-car in-loop traits to bypass `Suggestor`

## Reference Points

Use `crates/runtime/src/formation.rs` for the current Formation contract and
`examples/expense-approval` for the intended end-to-end wiring pattern.

See also: [[Architecture/Converge Contract]], [[Philosophy/Relationship to Converge]]
