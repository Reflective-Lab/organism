---
name: audit
description: Full workspace health — security, compliance, drift, observations. Weekly.
user-invocable: true
allowed-tools: Bash, Read, Edit, Grep, Glob, Agent
---
# Audit
Weekly workspace review. Runs from ~/dev/work/ level.
## Steps
1. **Security** — for each project (parallel agents):
   - Secrets in tracked files (sk-, AKIA, ghp_, password=, secret=, token=)
   - .env files tracked or on disk with real values
   - `cargo audit` / dep vulnerabilities
2. **Compliance** — for each project:
   - .gitignore covers target/, node_modules/, build/, dist/, .env, .DS_Store
   - No tracked artifacts (build/, .obsidian/)
   - .git/ size < 50MB
   - GitHub remote configured, code pushed
   - Scaffold: CLAUDE.md, AGENTS.md, MILESTONES.md, CHANGELOG.md, kb/, skills
   - Code compiles: `just check` or `bun run check`
3. **Drift** — for each project:
   - Rust edition 2024, rust-version 1.94, no unsafe
   - Bun (not npm), SvelteKit, Tauri
   - Converge deps use public crates only
   - Justfile has check/lint/test/dev recipes
   - Skills match standard set (13)
   - Cloud resources in Terraform
   - Layering (organism ≠ axioms, saas-killer → through organism)
4. **Observations** — read `kb/Observations.md`:
   - Propose graduation for validated observations (rule, skill, kb, discard)
   - **Ask user to confirm** each graduation
   - Apply confirmed graduations, remove from Observations.md, log in LOG.md
5. **Milestones** — read MILESTONES.md and EPIC.md:
   - Flag overdue or stalled milestones
   - Flag deadline risk (< 7 days with open deliverables)
6. Update `kb/Audits/` and `kb/History/Audit Log.md`.
7. Output:
```
── Audit ──────────────────────────────────────────
Date: <today>
Security:     <PASS|ISSUES>
Compliance:   <PASS|ISSUES>
Drift:        <PASS|ISSUES>
Milestones:   <at risk or on track>
Observations: <N> pending, <N> graduated, <N> discarded
Action items:
1. ...
────────────────────────────────────────────────────
```
## Rules
- Be direct about problems.
- Stalled = no progress in 7+ days.
- Priority: security > compliance > drift > milestones.
- Never graduate without user confirmation.
- Observations pending 3+ weeks → propose discard.
