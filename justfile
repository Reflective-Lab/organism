default:
    @just --list

build:
    cargo build --workspace

test:
    cargo test --workspace

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt --all

check:
    cargo check --workspace --all-targets

clean:
    cargo clean

# Install git pre-commit hooks (fmt + clippy)
hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed — .githooks/pre-commit will run on each commit"

# ── Workflow ──────────────────────────────────────────────────────────

# Session opener — repo health + recent activity
focus:
    @bash scripts/workflow/focus.sh

# Team sync — PRs, issues, recent commits
sync:
    @bash scripts/workflow/sync.sh

# Build health, test results
status:
    @bash scripts/workflow/status.sh
