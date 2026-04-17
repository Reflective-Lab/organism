---
name: experiment
description: Formulate a hypothesis, run an experiment, record the outcome.
user-invocable: true
allowed-tools: Read, Write, Edit, Bash, Grep, Glob, Agent
---
# Experiment
Hypothesis-driven development workflow. Every experiment produces evidence.

## Invocation
`/experiment` — start a new experiment
`/experiment log` — show the experiment log
`/experiment <id>` — resume or review an existing experiment

## New Experiment Flow
1. **Ask** the user for:
   - **Hypothesis**: a falsifiable statement ("I believe X because Y")
   - **Falsification criterion**: what would prove it wrong ("If Z, the hypothesis fails")
   - **Method**: how to test it (code change, test run, benchmark, data probe, etc.)
   - If the user already stated the hypothesis inline, extract it — don't re-ask.

2. **Assign an ID**: `EXP-<NNN>` (next sequential number from `experiments/LOG.md`).

3. **Create the experiment file**: `experiments/EXP-<NNN>.md` using this template:
```markdown
# EXP-<NNN>: <short title>
**Date:** <YYYY-MM-DD>
**Status:** running | confirmed | falsified | inconclusive
**Hypothesis:** <the falsifiable statement>
**Falsification criterion:** <what would disprove it>
**Method:** <how we test it>

## Setup
<code changes, config, branch — whatever is needed>

## Run
<commands executed, output captured>

## Evidence
<data, metrics, logs, diffs — the raw signal>

## Outcome
<confirmed | falsified | inconclusive>
<what we learned, in 1-3 sentences>
```

4. **Execute the method** — write code, run tests, benchmarks, or probes as needed.

5. **Record evidence** — capture output, metrics, diffs into the experiment file.

6. **Conclude** — update status and outcome. Append a one-liner to `experiments/LOG.md`.

## Log Format (`experiments/LOG.md`)
```markdown
# Experiment Log
| ID | Date | Hypothesis (short) | Status | Outcome |
|----|------|---------------------|--------|---------|
| EXP-001 | 2026-04-17 | Kong metadata captured in headers | confirmed | 6 headers extracted |
```

## Resume Flow (`/experiment <id>`)
1. Read `experiments/EXP-<NNN>.md`
2. Show current status and what's left
3. Continue execution or record outcome

## Rules
- Every experiment MUST have a falsification criterion before running.
- Record evidence even when the outcome is obvious — the log is the value.
- "Inconclusive" is a valid outcome. State why and what would resolve it.
- Don't delete or edit past outcomes. Corrections are new experiments.
- Keep experiment files self-contained — someone reading just that file should understand what happened.
- If an experiment changes code that should persist, commit it separately from the experiment file.
