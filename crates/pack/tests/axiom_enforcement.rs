//! Compile-time axiom enforcement tests.
//!
//! These tests prove that Organism cannot bypass Converge's governance:
//! - `Fact` cannot be constructed (private fields, no public constructor)
//! - `ProposedFact` CAN be constructed (the intended path)
//!
//! If Converge ever accidentally makes `Fact` constructible, these tests
//! will fail — catching the regression before it ships.

#[test]
fn fact_construction_blocked() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/fact_construction_blocked.rs");
    t.compile_fail("tests/compile_fail/fact_no_new.rs");
    t.pass("tests/compile_fail/proposed_fact_compiles.rs");
}
