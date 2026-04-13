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

build_log="$(mktemp)"
trap 'rm -f "$build_log"' EXIT

build_status="broken"
build_detail=""
if just check >"$build_log" 2>&1; then
    build_status="clean"
else
    build_detail="$(tail -n 10 "$build_log")"
fi

recent_activity="$(git log --oneline --all --since='24 hours ago' --format='%h %an: %s' 2>/dev/null || true)"
[[ -n "$recent_activity" ]] || recent_activity="None"

prs="$(gh_output pr list)"
issues="$(gh_output issue list --limit 10)"

user_name="$(git config user.name 2>/dev/null || true)"
first_session="yes"
if [[ -n "$user_name" ]] && [[ -n "$(git log --author="$user_name" -1 --format='%H' 2>/dev/null || true)" ]]; then
    first_session="no"
fi

printf '── Session Focus ──────────────────────────────────\n\n'
printf 'Project:     Organism — Organizational Intelligence Runtime\n'
printf 'Build:       %s\n\n' "$build_status"

printf 'Recent team activity (last 24h):\n'
printf '%s\n\n' "$recent_activity" | indent_block

printf 'In flight:\n'
printf '  PRs:\n'
printf '%s\n' "$prs" | indent_block
printf '  Issues:\n'
printf '%s\n\n' "$issues" | indent_block

printf 'Start here:\n'
if [[ "$first_session" == "yes" ]]; then
    cat <<'EOF' | indent_block
- Read kb/Philosophy/Why Organism.md
- Read kb/Philosophy/Relationship to Converge.md
- Read kb/Philosophy/Key Invariants.md
- Read kb/Building/Getting Started.md
EOF
else
    printf '%s\n' "You're up to speed. Pick an issue or write a ticket." | indent_block
fi

if [[ -n "$build_detail" ]]; then
    printf '\nBuild detail:\n'
    printf '%s\n' "$build_detail" | indent_block
fi

printf '\n────────────────────────────────────────────────────\n'
