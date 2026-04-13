# Gemini CLI Entrypoint

Read and follow `AGENTS.md` — it is the canonical project documentation.

## Gemini-Specific Notes

- Use `codebase_investigator` for deep architectural research or bug root-cause analysis.
- Use `generalist` for batch refactoring or high-volume file operations across the workspace.
- Prefer `grep_search` and `glob` over reading entire files. Lazy-load `kb/` pages as needed.
- Use `save_memory` for personal preferences only — project knowledge belongs in `kb/`.
- See `~/dev/work/EPIC.md` for strategic context (Organism = E2).
