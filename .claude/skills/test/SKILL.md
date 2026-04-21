---
name: test
model: opus
description: Add tests to a Rust codebase — unit, negative, integration, property, compile-fail, and soak.
user-invocable: true
argument-hint: [crate-or-module] [category]
allowed-tools: Read, Edit, Write, Bash, Grep, Glob, Agent
---
# Test — Rust Test Expansion

Expand test coverage for `$ARGUMENTS`. If no argument is given, ask the user which crate or module to target.

## Invocation
`/test` — guided test expansion (asks for target)
`/test <crate>` — expand tests for the named crate
`/test <crate> <category>` — add only tests of the given category (unit | negative | integration | property | compile-fail | soak)

---

## Workflow

### 1. Orient
- Read the target module(s): source files, existing `#[cfg(test)]` blocks, `tests/` directory.
- Read `Cargo.toml` for existing dev-dependencies.
- Identify what is already covered and what is missing.
- Do **not** duplicate tests that already exist.

### 2. Identify gaps
For each public function, type, or behavior in the target, check coverage against the pyramid below. Note what is absent.

### 3. Add tests bottom-up
Follow the priority order: unit → negative → integration → property → compile-fail → soak.
Stop when the user's scope is satisfied or ask if they want to continue to the next tier.

### 4. Verify
Run `just test` (or `cargo test -p <crate>`) after each tier. Fix any failures before adding the next tier.

### 5. Lint
Run `just lint` before finishing. Fix any clippy warnings introduced by new test code.

---

## Test Pyramid Reference

### Unit tests
- Location: `#[cfg(test)]` module inside the source file.
- Runtime: `#[test]` (sync) or `#[tokio::test]` (async).
- Cover: parsing, validation, state transitions, retry/backoff logic, permission checks, serialization helpers.
- Template:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_does_the_thing() {
        // arrange
        // act
        // assert
    }
}
```

### Negative tests
- Same location as unit tests.
- Cover: invalid format, missing required fields, permission denied, timeout, overflow edges, corrupted state, dependency errors.
- Every `Err` variant your public API can return must have at least one negative test.
- Template:
```rust
#[test]
fn rejects_invalid_input() {
    let result = parse("!!!invalid!!!");
    assert!(result.is_err());
    // optionally assert the error kind:
    // assert_eq!(result.unwrap_err(), MyError::InvalidFormat);
}
```

### Integration tests
- Location: `tests/<name>.rs` in the crate root.
- Exercise the crate as a consumer would — no access to private internals.
- Cover: API workflows, storage read/write, retries, config loading, CLI flows.
- Add dev-dependencies (e.g. `tempfile`, `assert_cmd`) to `Cargo.toml` under `[dev-dependencies]` if needed.
- Template:
```rust
// tests/integration_foo.rs
use my_crate::Foo;

#[test]
fn full_workflow_succeeds() {
    let foo = Foo::new(/* real config */);
    let result = foo.run();
    assert!(result.is_ok());
}
```

### Property tests (proptest)
- Location: `#[cfg(test)]` inside source file or `tests/`.
- Add `proptest = { workspace = true }` to dev-dependencies if not present.
- Cover: roundtrip correctness, idempotency, monotonic ordering, determinism, no panic on valid input.
- Think: "for all valid inputs, this invariant must hold."
- Template:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn roundtrip(s in "[a-z]{1,32}") {
        let encoded = encode(&s);
        let decoded = decode(&encoded).unwrap();
        prop_assert_eq!(s, decoded);
    }
}
```

### Compile-fail tests (trybuild)
- Use when the crate exposes macros, trait bounds, builders, or type-level safety guarantees.
- Add `trybuild = { workspace = true }` to dev-dependencies if not present.
- Location: `tests/compile_fail.rs` + `tests/ui/*.rs` (failing programs) and `tests/ui/*.stderr` (expected errors).
- Cover: invalid API usage does not compile, safe builder states enforced, trait bounds produce correct diagnostics.
- Template:
```rust
// tests/compile_fail.rs
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail_*.rs");
    t.pass("tests/ui/pass_*.rs");
}
```
- Each `tests/ui/fail_*.rs` must have a matching `tests/ui/fail_*.stderr` with the expected compiler output.
- Run `TRYBUILD=overwrite cargo test` to regenerate `.stderr` files after intentional changes.

### Soak / longevity tests
- **Not run by `cargo test` by default.** Gate them behind a feature flag or environment variable.
- Intended for nightly CI or pre-prod environments.
- Cover: memory growth, file descriptor leaks, task count, queue backlog, connection churn, restart loops, retry storms, latency stability.
- Template:
```rust
#[tokio::test]
#[ignore = "soak: run manually or in nightly CI"]
async fn no_memory_growth_under_load() {
    let duration = std::time::Duration::from_secs(300); // 5 min
    let start = std::time::Instant::now();
    while start.elapsed() < duration {
        // exercise the system
        // sample RSS or task count periodically
    }
    // assert final metrics within bounds
}
```
- Run manually with: `cargo test -- --ignored soak`

---

## Coverage Heuristics

| Surface | Required tiers |
|---------|----------------|
| Business logic | unit + property |
| Error handling | negative |
| Public APIs | integration |
| Macros / generics / builders | compile-fail |
| Async / concurrency / runtime | soak + integration |
| CLI | integration (assert_cmd) |

---

## Regression Rule
If a bug happened once, add a named regression test:
```rust
#[test]
fn regression_issue_123_overflow_on_empty_vec() { ... }
```

---

## Recommended Crates

| Crate | Purpose |
|-------|---------|
| `proptest` | Property-based testing |
| `trybuild` | Compile-fail / compile-pass tests |
| `assert_cmd` | CLI integration tests |
| `tempfile` | Temporary files/dirs in tests |
| `insta` | Snapshot testing |
| `tokio::test` | Async unit/integration tests |
| `criterion` | Benchmarks (separate from correctness) |
| `serial_test` | Serialize tests sharing global state (use sparingly) |

When adding a new crate, add it to `[dev-dependencies]` in the crate's `Cargo.toml` with `workspace = true` if it is already in the workspace, otherwise add it there first.
