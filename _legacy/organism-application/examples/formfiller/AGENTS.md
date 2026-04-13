# Agent Instructions

## Version Control

Use **Jujutsu (jj) on top of git** for day-to-day version control operations:

```bash
jj status
jj diff
jj commit -m "message"
jj git fetch
jj rebase -d main@origin
jj git push
```

Git remains the interoperability layer (hosting, remotes, CI). Use `jj git ...` for fetch/push.

## Work Tracking

Use the team's current task tracker (GitHub Issues, Jira, Linear, or a repo-local `TASKS.md`). Do not depend on a repo-specific issue-tracker CLI in this repo.

Minimum task record:
- stable identifier or clear title
- status (`todo`, `in_progress`, `blocked`, `done`)
- owner (`human` or `agent`)
- next action / unblocker
- handoff note when pausing work

If no tracker exists, create or update `TASKS.md` in the repo root.

## Agent Collaboration Standard

This repository supports a tool-agnostic agent workflow (Claude, Codex, Gemini, Cursor, local scripts, etc.). The collaboration contract is the same regardless of agent:

1. **Start** - Read `AGENTS.md` and relevant docs, sync with `jj git fetch`, and select/claim work in the tracker or `TASKS.md`.
2. **Execute** - Keep changes scoped to one task, record assumptions/decisions, and run targeted validation commands.
3. **Handoff** - Summarize changed files, commands run and outcomes, blockers, and next actions in the tracker (or `TASKS.md`).

Prefer durable artifacts (tracked docs, task comments, reports) over chat-only context so another human or agent can continue the work.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `jj git push` succeeds.

**MANDATORY WORKFLOW:**

1. **Record remaining work** - Add follow-up items and handoff notes to the team's tracker or `TASKS.md`
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update task status** - Mark finished work, blocked work, and next actions
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   jj git fetch
   jj rebase -d main@origin
   jj git push
   jj status  # Working copy should be clean
   ```
5. **Clean up** - Remove temporary branches/files and stale local state as appropriate
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `jj git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Version Control

- Use `jj` for day-to-day operations (commits, branches, status, log)
- Fall back to `git` only when `jj` cannot handle it (e.g. specific remote operations)
