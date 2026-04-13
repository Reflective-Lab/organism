#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

indent_block() {
    sed 's/^/  /'
}

gh_output() {
    if ! command -v gh >/dev/null 2>&1; then
        echo "None (gh CLI not installed)"
        return
    fi

    local output
    if output="$(gh "$@" 2>&1)"; then
        if [[ -n "$output" ]]; then
            printf '%s\n' "$output"
        else
            echo "None"
        fi
        return
    fi

    output="${output%%$'\n'*}"
    printf 'Unavailable (%s)\n' "$output"
}

run_step() {
    local label="$1"
    local command="$2"
    local log_file

    log_file="$(mktemp)"
    if bash -lc "$command" >"$log_file" 2>&1; then
        rm -f "$log_file"
        printf '%s\tpass\t\n' "$label"
        return
    fi

    local detail
    detail="$(tail -n 10 "$log_file")"
    rm -f "$log_file"
    printf '%s\tfail\t%s\n' "$label" "$detail"
}

recent_work="$(git log --oneline --all --since='24 hours ago' --format='%an: %s' 2>/dev/null | sort || true)"
[[ -n "$recent_work" ]] || recent_work="None"

prs="$(gh_output pr list)"
issues="$(gh_output issue list --limit 15)"
stale_kb="$(find kb -name '*.md' -mtime +14 | sort 2>/dev/null || true)"
[[ -n "$stale_kb" ]] || stale_kb="None"

check_result="$(run_step "Build" "just check")"
test_result="$(run_step "Tests" "just test")"

check_status="$(printf '%s' "$check_result" | cut -f2)"
check_detail="$(printf '%s' "$check_result" | cut -f3-)"
test_status="$(printf '%s' "$test_result" | cut -f2)"
test_detail="$(printf '%s' "$test_result" | cut -f3-)"

printf '── Team Sync ──────────────────────────────────────\n\n'

printf 'Recent work (last 24h):\n'
printf '%s\n\n' "$recent_work" | indent_block

printf 'PRs awaiting review:\n'
printf '%s\n\n' "$prs" | indent_block

printf 'Issues:\n'
printf '%s\n\n' "$issues" | indent_block

printf 'Build:       %s\n' "$check_status"
printf 'Tests:       %s\n' "$test_status"

if [[ -n "$check_detail" ]]; then
    printf '\nBuild detail:\n'
    printf '%s\n' "$check_detail" | indent_block
fi

if [[ -n "$test_detail" ]]; then
    printf '\nTest detail:\n'
    printf '%s\n' "$test_detail" | indent_block
fi

printf '\nStale KB pages (>14 days):\n'
printf '%s\n' "$stale_kb" | indent_block

printf '\n────────────────────────────────────────────────────\n'
