---
tags: [workflow, git]
source: mixed
---
# Git Strategy

This repo needs a boring Git model. The goal is not creativity. The goal is a
clean `main`, reproducible releases, and short-lived branches that do not rot.

Repo-native report:

```bash
just git-hygiene
```

Use it to see the current branch, worktrees, latest release tag, and remote
branch cleanup candidates before starting or after merging work.

## Core Rules

1. `main` is the integration branch, not a scratch branch.
2. The root checkout stays on `main` and stays clean.
3. Any non-trivial change starts on a short-lived topic branch.
4. If two changes are unrelated, they do not share a branch or worktree.
5. Releases are defined by annotated tags, not by whatever commit `main` is on
   today.
6. Remote branches are not archival storage. Merge them or delete them.

## Operating Model

- Keep the primary checkout on `main`.
- Use that checkout for:
  - `just focus`
  - `just sync`
  - release tagging
  - branch cleanup
- Do implementation work in a dedicated topic branch, preferably in a dedicated
  worktree.

This keeps the root checkout readable and prevents stacked local state from
turning into folklore.

## Branch Types

| Prefix | Use |
|---|---|
| `feat/<slug>` | new capability or surface |
| `fix/<slug>` | bug fix or regression repair |
| `docs/<slug>` | README, KB, or API docs |
| `ci/<slug>` | workflows, hooks, badges, coverage scripts |
| `chore/<slug>` | maintenance that is not user-facing |
| `release/<version>` | version bump, release notes, tag prep |
| `spike/<slug>` | disposable investigation branch; do not let these linger |

Use one concern per branch. If a docs update becomes a runtime fix, split it.

## Worktree Policy

- The root repo directory stays on `main`.
- Parallel work gets a separate worktree.
- Name worktrees after the branch or task so they are obvious.

Example:

```bash
git switch main
git pull --ff-only
git worktree add ../organism-fix-formation -b fix/formation-boundary main
```

When the branch is merged or abandoned:

```bash
git worktree remove ../organism-fix-formation
git branch -d fix/formation-boundary
```

If a branch is still active, do not share its worktree with a second concern.

## Daily Flow

1. Start in the root checkout.
2. Run `just focus` or `just sync`.
3. Check `git status --short --branch` and `git worktree list`.
4. Run `just git-hygiene` if branch or remote state looks suspicious.
5. If the task is more than a trivial one-file fix, create a topic branch.
6. If another task appears while one is in flight, create another branch and
   worktree instead of stacking changes.
7. Before pushing, run the repo's quality gate for the scope of the change.
8. After merge, delete the local branch, remove the worktree, and delete the
   remote branch.

## Merge Policy

- Prefer a linear `main`.
- Rebase the topic branch onto current `main` before merge.
- Prefer squash or rebase merge for normal work.
- Do not keep stale merge-commit archaeology on routine branches.
- A branch that is weeks behind `main` is usually cheaper to recreate than to
  nurse back to life.

## Release Policy

- A release is one specific commit with one specific annotated tag.
- Use tags like `v1.2.0`.
- The version bump, release notes, and release validation belong to the same
  release branch or release commit.
- `main` may move immediately after the tag. That is normal.
- Never assume "`HEAD` equals latest release." The tag is the source of truth.

## Automation Branches

### Dependabot

Dependabot cargo and GitHub Actions bumps are low-drama maintenance work.

- Default policy: auto-merge if the update is isolated, CI is green, and the
  change does not require code or contract edits.
- Manually review:
  - major version bumps
  - updates that break lockstep crate families
  - updates that require runtime, policy, protocol, or API changes

Do not let stale dependabot branches accumulate. If one falls far behind
`main`, close it and let automation recreate it.

### CI / Docs Maintenance Branches

Branches for badges, hooks, coverage wiring, or README cleanup are ordinary
topic branches, not permanent infrastructure.

- Merge quickly if still relevant.
- Close and recreate if stale.
- Delete the remote branch as soon as the PR is merged or abandoned.

## Remote Hygiene

- Delete merged remote branches immediately.
- Delete stale unmerged branches once their PR is closed or superseded.
- If a branch is old, behind `main`, and unreviewed, it is clutter.
- Recreate from current `main` instead of preserving dead history out of
  sentimentality.

## Agent Rules

- Do not begin substantive implementation directly on `main`.
- Do not mix unrelated changes in one branch.
- Do not leave a dirty root checkout as the default team state.
- Do not describe a branch as "the release" when the tag says otherwise.

See also: [[Workflow/Daily Journey]], [[Workflow/Working with Claude]], [[Workflow/Working with Codex]], [[Workflow/Working with Gemini]]
