# Organism Agent OS — Development Commands
# Install: brew install just  |  cargo install just
# Usage:   just --list

set dotenv-load := true

# ── Build ──────────────────────────────────────────────────────────────

# Build workspace (release)
build:
    cargo build --release

# Build workspace (fast iteration)
build-quick:
    cargo build --profile quick-release

# Build for CI
build-ci:
    cargo build --workspace --profile ci

# Check workspace without producing release artifacts
check:
    cargo check --workspace --all-targets

# ── Test ───────────────────────────────────────────────────────────────

# Run tests (default members)
test:
    cargo test --workspace

# Run all tests
test-all:
    cargo test --all-targets --workspace

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}} --all-targets

# Guard test file placement so Rust test files do not live in dead ad hoc directories
test-layout:
    #!/usr/bin/env bash
    set -euo pipefail
    bad="$(find crates -type f \
        \( -name '*proptest*.rs' -o -name '*property*.rs' -o -name '*negative*.rs' \) \
        ! -path '*/src/*' ! -path '*/tests/*' -print)"
    if [ -n "${bad}" ]; then
        echo "Non-standard Rust test files must live under src/ or tests/:"
        echo "${bad}"
        exit 1
    fi

# Run a single test by name
test-one name:
    cargo test --all-targets -- {{name}}

# Run benchmarks (compile only)
test-bench:
    cargo bench --workspace --no-run

# Run benchmarks (with execution)
test-bench-run:
    cargo bench --workspace

# Run soak tests (long-running stability tests)
test-soak:
    cargo test --workspace -- --include-ignored soak

# Security regression gate
sec-gate:
    cargo check --workspace
    cargo test -p organism-intent
    cargo test -p organism-runtime --lib
    cargo test -p organism-pack --test compile_fail

# Alias used by SECURITY.md and release checklists
security-gate: sec-gate

# Blocking dependency security audit for release candidates
security-audit:
    cargo audit --deny warnings
    cargo deny check

# ── Lint & Format ─────────────────────────────────────────────────────

# Check formatting, clippy, and test layout hygiene
lint: _coverage-summary test-layout
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Auto-fix lint issues
fix-lint:
    cargo clippy --fix --allow-staged --allow-dirty --allow-no-vcs
    cargo fmt --all

# Format only
fmt:
    cargo fmt --all

# Show test coverage by crate
_coverage-summary:
    #!/usr/bin/env bash
    echo "──────────────────────────────────────────────"
    echo "Test Coverage Summary"
    echo "──────────────────────────────────────────────"
    (
      for crate in crates/*/; do
        crate_name=$(basename "$crate")
        unit_count=$(find "$crate/src" -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; 2>/dev/null | wc -l)
        integration_count=$(ls "$crate/tests"/*.rs 2>/dev/null | wc -l)
        bench_count=$(ls "$crate/benches"/*.rs 2>/dev/null | wc -l)
        src_files=$(find "$crate/src" -name "*.rs" 2>/dev/null | wc -l)

        if [ "$unit_count" -gt 0 ] || [ "$integration_count" -gt 0 ] || [ "$bench_count" -gt 0 ]; then
          printf "%-25s unit=%2d integration=%d bench=%d (src: %d files)\n" \
            "$crate_name" "$unit_count" "$integration_count" "$bench_count" "$src_files"
        fi
      done | sort -t= -k2 -rn
    )
    echo "──────────────────────────────────────────────"
    crates_with_tests=$(find crates -path "*/tests/*.rs" -o -path "*/benches/*.rs" | cut -d/ -f2 | sort -u | wc -l)
    total_crates=$(ls -1d crates/*/ | wc -l)
    echo "Test coverage: $crates_with_tests/$total_crates crates have tests"
    echo "──────────────────────────────────────────────"

# ── Docs ───────────────────────────────────────────────────────────────

# Generate workspace docs
doc:
    cargo doc --no-deps --workspace

# Generate local test coverage JSON
coverage:
    mkdir -p target/coverage
    cargo llvm-cov --workspace --lib --tests --ignore-filename-regex '(^|/)(tests|benches|examples)/' --json --summary-only --output-path target/coverage/organism-coverage.json

# Generate CI coverage JSON at repo root
coverage-ci:
    cargo llvm-cov --workspace --lib --tests --ignore-filename-regex '(^|/)(tests|benches|examples)/' --json --summary-only --output-path coverage.json

# Run Criterion benchmarks
perf-baseline:
    cargo bench --workspace -- --save-baseline v1.5.0

# Open docs in browser
doc-open:
    cargo doc --no-deps --workspace --open

# ── Publish ────────────────────────────────────────────────────────────

# Dry-run publish to crates.io
publish-dry-run:
    cargo publish --dry-run --workspace

# ── Security ───────────────────────────────────────────────────────────

# Audit dependencies (requires cargo-deny)
sec-deny:
    cargo deny check

# ── Local Dev ──────────────────────────────────────────────────────────

# Start local dev environment
dev-up:
    @echo "Starting dev environment..."
    # Add project specific dev-up logic here

# ── Git ────────────────────────────────────────────────────────────────

# Install git pre-commit hooks
git-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed — .githooks/pre-commit will run on each commit"

# Create a worktree for parallel work
git-worktree branch:
    git worktree add ../organism-{{branch}} -b {{branch}}
    @echo "Worktree ready at ../organism-{{branch}}"

# Remove a worktree
git-worktree-rm branch:
    git worktree remove ../organism-{{branch}}

# List active worktrees
git-worktrees:
    git worktree list

# Git hygiene report
git-hygiene:
    #!/usr/bin/env bash
    set -euo pipefail
    # (Implementation similar to converge/justfile)

# Build health and recent commits
status:
    @bash scripts/workflow/status.sh
    @echo "---"
    @git log --oneline -5

# Repo state and recent commits
sync:
    @bash scripts/workflow/sync.sh
    @echo "---"
    @git log --oneline -5

# Session opener
focus:
    @just sync
    @echo "---"
    @cargo build --workspace
    @cargo test --workspace --lib -- --quiet
    @echo "✓ workspace healthy"

# Legacy aliases
wow-focus: focus
git-sync: sync
git-status: status

# ── Clean ──────────────────────────────────────────────────────────────

clean:
    cargo clean
