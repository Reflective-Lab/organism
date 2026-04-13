---
name: quality
description: Capture code quality metrics and append to quality log — track trends over time
disable-model-invocation: true
user-invocable: true
argument-hint: [baseline|check|trend]
allowed-tools: Bash, Read, Write, Grep, Glob
---

# Code Quality Index

Capture quality metrics for the Wolfgang codebase and track trends.

## Metrics collected

### Rust
1. **Clippy warnings** — `cargo clippy --workspace 2>&1 | grep "warning:" | wc -l`
2. **Clippy complexity** — `cargo clippy --workspace -- -W clippy::cognitive_complexity 2>&1 | grep "cognitive_complexity" | wc -l`
3. **Test count** — `cargo test --workspace 2>&1 | grep "test result"` (extract pass/fail/ignore counts)
4. **Unsafe blocks** — `grep -r "unsafe" --include="*.rs" -l` (excluding target/)
5. **Dependency vulnerabilities** — `cargo audit 2>&1 | grep -c "Vulnerability found"`

### Svelte/TypeScript
6. **Svelte errors/warnings** — `bunx svelte-check 2>&1 | grep "COMPLETED"`
7. **TODO/FIXME count** — grep across all source files

### Codebase size
8. **Lines of code** — `tokei` if available, otherwise wc -l by language
9. **File count** — by type (*.rs, *.svelte, *.ts)

## Commands

### `check` (default)
Run all metrics, output a summary table, and append to `quality-log.csv`.

Output format:
```
── Quality Index ──────────────────────────────────

Date:            <YYYY-MM-DD>
Clippy:          <N> warnings (<+/- vs last>)
Complexity:      <N> functions flagged (<+/- vs last>)
Tests:           <pass>/<total> (<+/- vs last>)
Svelte:          <N> errors, <M> warnings (<+/- vs last>)
TODOs:           <N> (<+/- vs last>)
Unsafe:          <N> files (<+/- vs last>)
Vulnerabilities: <N> (<+/- vs last>)
LOC:             <N> Rust, <M> Svelte, <K> TypeScript

Trend: <improving | stable | deteriorating>

────────────────────────────────────────────────────
```

### `baseline`
Run all metrics and write the first row to `quality-log.csv`. Use this once to establish the starting point.

### `trend`
Read `quality-log.csv` and show the trend over the last 5 entries. Flag any metric that has worsened for 3+ consecutive entries.

## Quality log format

File: `quality-log.csv` at repo root.

```csv
date,clippy_warnings,complexity_flags,tests_pass,tests_total,svelte_errors,svelte_warnings,todos,unsafe_files,vulnerabilities,loc_rust,loc_svelte,loc_ts
```

## Rules

- Always run from repo root.
- Use `bunx` not `npx` for Svelte checks.
- Use `just test` when available instead of raw cargo test.
- Compare each metric to the previous entry and show +/- delta.
- "Deteriorating" = 2+ metrics worsened with none improved. "Improving" = 2+ improved with none worsened. Otherwise "stable".
- Do not fail the skill on bad metrics — just report honestly.
