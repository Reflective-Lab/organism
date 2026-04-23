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
git-hooks:
    git config core.hooksPath .githooks
    @echo "Git hooks installed — .githooks/pre-commit will run on each commit"

# ── Git ───────────────────────────────────────────────────────────────

# Create a worktree for parallel work (e.g., just git-worktree feat/my-feature)
git-worktree branch:
    git worktree add ../organism-{{branch}} -b {{branch}}
    @echo "Worktree ready at ../organism-{{branch}}"
    @echo "When done: just git-worktree-rm {{branch}}"

# Remove a worktree
git-worktree-rm branch:
    git worktree remove ../organism-{{branch}}
    @echo "Worktree removed. Branch '{{branch}}' still exists — delete with: git branch -d {{branch}}"

# List active worktrees
git-worktrees:
    git worktree list

# Report branch/worktree/release hygiene and remote cleanup candidates
git-hygiene:
    #!/usr/bin/env bash
    set -euo pipefail

    current_branch="$(git branch --show-current 2>/dev/null || true)"
    current_sha="$(git rev-parse --short HEAD)"
    latest_tag="$(git tag --sort=-creatordate --list 'v*' | head -n1 || true)"

    echo "──────────────────────────────────────────────"
    echo "Git Hygiene"
    echo "──────────────────────────────────────────────"
    printf "branch: %s\n" "${current_branch:-DETACHED}"
    printf "head:   %s\n" "${current_sha}"
    if [ -n "${latest_tag}" ]; then
        tag_sha="$(git rev-list -n 1 "${latest_tag}")"
        since_tag="$(git rev-list --count "${latest_tag}..HEAD")"
        printf "latest release tag: %s (%s)\n" "${latest_tag}" "$(git rev-parse --short "${tag_sha}")"
        printf "commits since tag:  %s\n" "${since_tag}"
    else
        echo "latest release tag: none"
    fi

    echo
    echo "Worktrees"
    echo "─────────"
    git worktree list

    echo
    echo "Local Branches"
    echo "──────────────"
    git branch -vv

    echo
    echo "Working Tree"
    echo "────────────"
    git status --short --branch

    if git show-ref --verify --quiet refs/remotes/origin/main; then
        echo
        echo "Merged Remote Branches (safe delete candidates)"
        echo "──────────────────────────────────────────────"
        merged=0
        while IFS= read -r branch; do
            [ -z "${branch}" ] && continue
            case "${branch}" in
                origin|origin/HEAD|origin/main) continue ;;
            esac
            if git merge-base --is-ancestor "${branch}" origin/main; then
                printf "%s\t%s\t%s\n" \
                    "${branch}" \
                    "$(git for-each-ref --format='%(committerdate:short)' "refs/remotes/${branch}")" \
                    "$(git log -1 --format=%s "${branch}")"
                merged=1
            fi
        done < <(git for-each-ref --format='%(refname:short)' refs/remotes/origin)
        if [ "${merged}" -eq 0 ]; then
            echo "none"
        fi

        echo
        echo "Unmerged Remote Branches (review or recreate)"
        echo "─────────────────────────────────────────────"
        unmerged=0
        while IFS= read -r branch; do
            [ -z "${branch}" ] && continue
            case "${branch}" in
                origin|origin/HEAD|origin/main) continue ;;
            esac
            if ! git merge-base --is-ancestor "${branch}" origin/main; then
                counts="$(git rev-list --left-right --count "${branch}...origin/main")"
                read -r ahead behind <<< "${counts}"
                printf "%s\tahead=%s\tbehind=%s\t%s\t%s\n" \
                    "${branch}" \
                    "${ahead}" \
                    "${behind}" \
                    "$(git for-each-ref --format='%(committerdate:short)' "refs/remotes/${branch}")" \
                    "$(git log -1 --format=%s "${branch}")"
                unmerged=1
            fi
        done < <(git for-each-ref --format='%(refname:short)' refs/remotes/origin)
        if [ "${unmerged}" -eq 0 ]; then
            echo "none"
        fi
    fi

# Repo state and recent commits
git-sync:
    @bash scripts/workflow/sync.sh

# Build health, test results
git-status:
    @bash scripts/workflow/status.sh

# ── Workflow ──────────────────────────────────────────────────────────

# Session opener — repo health + recent activity
wow-focus:
    @bash scripts/workflow/focus.sh
