---
tags: [workflow, claude]
---
# Working with Claude

Skills = reasoning + multi-step (inside Claude). Justfile = deterministic shell (terminal).

| I want to... | Use |
|---|---|
| Build | `just build` |
| Run tests | `just test` |
| Lint | `just lint` |
| Orient myself | `/focus` |
| Sync with team | `/sync` |
| Pick next task | `/next` |
| Fix an issue end-to-end | `/fix 42` |
| Run quality checks | `/check` |
| Expand test coverage | `/test <crate>` |
| Create a ticket | `/ticket add planning reasoner` |
| Create a PR | `/pr` |
| Review a PR | `/review 17` |
| End session | `/done` |
| Audit (Monday) | `/audit` |
| Hypothesis + evidence | `/experiment` |
| Start a topic branch | `/branch feat/my-thing` |
| Clean up after merge | `/merge-cleanup feat/my-thing` |
| Check git state | `just git-hygiene` |
| New worktree | `just worktree feat/my-thing` |
| Remove worktree | `just worktree-rm feat/my-thing` |

See also: [[Workflow/Daily Journey]], [[Workflow/Git Strategy]]
