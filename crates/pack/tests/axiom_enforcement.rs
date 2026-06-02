//! Compile-time axiom enforcement tests.
//!
//! These tests prove that Organism cannot bypass Converge's governance:
//! - `Fact` cannot be constructed (private fields, no public constructor)
//! - `ProposedFact` CAN be constructed (the intended path)
//!
//! If Converge ever accidentally makes `Fact` constructible, these tests
//! will fail — catching the regression before it ships.
//!
//! ## Patched-dep caveat
//!
//! The trybuild `.stderr` snapshots use trybuild's `$CARGO/$VERSION`
//! placeholders for `converge-pack`'s source path. trybuild can only
//! substitute those placeholders when `converge-pack` resolves from
//! crates.io (or a git source). When organism's workspace activates
//! `[patch.crates-io] converge-pack = { path = "../converge/crates/pack" }`
//! the placeholders no longer match — the compiler emits the absolute
//! local path instead — and the test would fail with a stale-snapshot
//! diff that has nothing to do with the public API contract we want
//! to verify.
//!
//! To keep the test honest, we detect that patched-local case at runtime
//! and skip with a load-bearing message. Releases and CI must run this
//! test against an *unpatched* checkout to verify the canonical published
//! API surface — that's when the placeholders substitute correctly and
//! the snapshot is meaningful.

#[test]
fn fact_construction_blocked() {
    if converge_pack_is_patched() {
        eprintln!(
            "SKIP fact_construction_blocked: converge-pack is patched locally via \
             [patch.crates-io] -> ../converge/crates/pack. The trybuild snapshots use \
             $CARGO/$VERSION placeholders that only substitute when converge-pack \
             resolves from crates.io. Run this test on an unpatched checkout (or in \
             release CI) to verify the canonical API surface."
        );
        return;
    }

    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/fact_construction_blocked.rs");
    t.compile_fail("tests/compile_fail/fact_no_new.rs");
    t.pass("tests/compile_fail/proposed_fact_compiles.rs");
}

/// True if organism's workspace patches `converge-pack` to a local path.
///
/// We probe the filesystem rather than reading Cargo.toml so the check
/// doesn't depend on parsing a moving target.
fn converge_pack_is_patched() -> bool {
    // CARGO_MANIFEST_DIR points to crates/pack at test time; walk to the
    // workspace root and look for the sibling converge checkout that the
    // patch entry redirects to.
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(workspace_root) = manifest_dir.ancestors().nth(2) else {
        return false;
    };
    workspace_root
        .join("../converge/crates/pack/Cargo.toml")
        .exists()
}
