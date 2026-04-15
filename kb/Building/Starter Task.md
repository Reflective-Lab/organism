---
tags: [building]
---
# Starter Task — Wire Converge Integration

## Context

The organism pipeline ends with a commit submission to Converge. The `runtime` crate owns this boundary via a `CommitBoundary` trait with two implementations:

- **Embedded:** uses `converge-kernel` directly (in-process)
- **Remote:** uses `converge-client` over gRPC (out-of-process)

## What to Build

### 1. Add Converge dependencies to runtime

```toml
[dependencies]
converge-pack = "3.0.0"
converge-model = "3.0.0"
converge-kernel = "3.0.0"   # embedded mode
# converge-client = "3.0.0" # remote mode — add when needed
```

### 2. Implement embedded CommitBoundary

```rust
use converge_kernel::Engine;
use converge_pack::ProposedFact;

pub struct EmbeddedConverge {
    engine: Engine,
}

impl CommitBoundary for EmbeddedConverge {
    fn submit(&self, run_id: &str, key: &str, content: &str, provenance: &str) -> Result<(), String> {
        // Construct ProposedFact, submit to engine
    }
}
```

### 3. Write a test proving the axioms hold

- Construct a `ProposedFact` — verify it compiles
- Attempt to construct a `Fact` directly — verify it does NOT compile (trybuild)
- Submit a proposal through the engine — verify promotion gate runs

## What NOT to Build

- No wrapper types around Converge types — use them directly
- No separate crate for Converge integration — runtime owns it
- Do not depend on `converge-core` or other internal Converge crates

## Reference Points

Use `crates/runtime/src/lib.rs` for the current `CommitBoundary` contract and `examples/expense-approval` for the intended end-to-end wiring pattern.

See also: [[Architecture/Converge Contract]], [[Philosophy/Relationship to Converge]]
