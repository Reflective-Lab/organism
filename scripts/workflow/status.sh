#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

indent_block() {
    sed 's/^/  /'
}

run_step() {
    local label="$1"
    local command="$2"
    local log_file

    log_file="$(mktemp)"
    if bash -lc "$command" >"$log_file" 2>&1; then
        local detail
        detail="$(tr -d '\r' <"$log_file")"
        rm -f "$log_file"
        if [[ "$detail" == "not-scaffolded" || "$detail" == "unavailable" ]]; then
            printf '%s\t%s\t\n' "$label" "$detail"
            return
        fi
        printf '%s\tpass\t\n' "$label"
        return
    fi

    local detail
    detail="$(tail -n 10 "$log_file")"
    rm -f "$log_file"
    printf '%s\tfail\t%s\n' "$label" "$detail"
}

check_result="$(run_step "Rust check" "just check")"
test_result="$(run_step "Rust tests" "just test")"
lint_result="$(run_step "Clippy" "just lint")"

print_result() {
    local result="$1"
    local label status detail

    label="$(printf '%s' "$result" | cut -f1)"
    status="$(printf '%s' "$result" | cut -f2)"
    detail="$(printf '%s' "$result" | cut -f3-)"

    if [[ "$status" == "not-scaffolded" ]]; then
        printf '%s:  %s\n' "$label" "not scaffolded"
        return
    fi

    if [[ "$status" == "unavailable" ]]; then
        printf '%s:  %s\n' "$label" "unavailable"
        return
    fi

    printf '%s:  %s\n' "$label" "$status"
    if [[ -n "$detail" ]]; then
        printf '%s detail:\n' "$label"
        printf '%s\n' "$detail" | indent_block
    fi
}

printf '── Status ─────────────────────────────────────────\n\n'
print_result "$check_result"
print_result "$test_result"
print_result "$lint_result"
printf '\n────────────────────────────────────────────────────\n'
