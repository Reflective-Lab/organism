---
tags: [concept, vendor-selection, rfp, formations, compiler]
---
# Vendor Selection Lifecycle

Vendor selection is the first proof wedge for the Organism formation compiler.
It should not be modeled as a simple scorecard. The hackathon materials describe
a full governed lifecycle that collapses a manual 8- or 9-step RFP process into
four composite decision flows.

## Business Problem

Manual enterprise AI vendor selection typically takes 4-6 months. The work is
slow because humans reconcile fragmented evidence across stakeholders, RFP
documents, pricing spreadsheets, security reviews, legal clauses, procurement
portals, demos, proof-of-concepts, and contract redlines.

The current process has recurring failure modes:

- version-control chaos in scoring matrices
- pricing incomparability across vendor models
- legal slowdowns around AI-specific clauses
- information bottlenecks through procurement
- proof-of-concept overhead
- evaluation fatigue across long timelines

Organism should not merely automate that process. It should run a better process
that humans cannot afford to coordinate manually.

## Four-Flow Model

The orchestrator view maps the long RFP lifecycle into four composite truths.
Stages that share the same principals, schema, and reasoning chain become one
flow.

| Flow | Manual Stages Covered | Why They Merge |
|---|---|---|
| F1 Frame | need, strategy, requirements | The scoring rubric must be signed with the business need. You cannot score what was not defined. |
| F2 Source | RFP issuance, vendor response | Fairness, Q&A, NDA tracking, deadlines, and ingest share one evidence pipeline. |
| F3 Decide | due diligence, evaluation, final selection | Verification, scoring, contradiction handling, and synthesis share one evidence base. |
| F4 Operate | contract, onboarding, monitoring | Contract commitments become the monitoring baseline and renewal evidence. |

This preserves the buyer's familiar vocabulary while removing unnecessary
handoff taxes.

## F1 Frame

Goal: define the need, constraints, and rubric as one governed artifact.

Inputs:

- business pain and ROI hypothesis
- budget envelope and sponsor
- standards, data residency, exit constraints
- decision principals such as CIO, CISO, DPO, Legal, Procurement, and Finance

Suggested roles:

- pain quantifier
- market scanner
- capability gap mapper
- functional requirements drafter
- security requirements drafter
- AI governance requirements drafter
- commercial requirements drafter
- weight synthesizer

Output:

- `ScoringRubric`
- `ShortlistSeed`
- signed audit entries for requirement and weight decisions

## F2 Source

Goal: issue the RFP fairly and ingest responses into a comparable shape.

Inputs:

- signed rubric and shortlist from F1
- RFP timeline
- NDA template
- Q&A process owner

Suggested roles:

- shortlist ranker
- RFP packager
- NDA tracker
- deadline watchdog
- Q&A consolidator
- PDF/OCR ingester
- structured extractor
- commercial normalizer
- missing evidence flagger
- boilerplate detector

Output:

- `NormalizedVendorResponse[]`
- `QALedger`
- `EvidenceGapReport`

## F3 Decide

Goal: run due diligence, evaluation, contradiction handling, synthesis, and
approval as one reasoning chain.

Inputs:

- normalized vendor responses from F2
- rubric from F1
- proof-of-concept results
- reference call notes
- external diligence evidence

Suggested roles:

- compliance screener
- cost analyzer
- capability fit evaluator
- vendor risk evaluator
- reference check logger
- proof-of-concept result ingester
- contradiction detector
- decision synthesizer
- rejection memo drafter

Output:

- `VendorSelectionDecisionRecord`
- `RubricBaseline`
- `AuditEntry[]`

Stop reasons can include `Converged`, `NeedsReview`, `CriteriaBlocked`, or
`BudgetExhausted`. A `NeedsReview` stop is successful if the engine has done all
it can and the next required step is a human signature.

## F4 Operate

Goal: reconcile the contract against the bid, monitor obligations, and close the
learning loop at renewal.

Inputs:

- signed decision record
- MSA, DPA, SoW, and AI addendum
- vendor telemetry
- certification registry
- regulatory feed

Suggested roles:

- contract diff
- walk-back detector
- clause extractor
- obligation tracker
- news watcher
- certification expiration monitor
- regulatory scanner
- SLA miss detector
- renewal scorer

Output:

- `ObligationLedger`
- quarterly `AuditSnapshot`
- `RenewalRecommendation`

F4 is what turns vendor selection from a one-shot RFP into a vendor lifecycle.
Renewal recommendations either continue the relationship or hand back to F1 with
prior rubric, obligation history, and regulatory deltas.

## Compiler Implications

The compiler must support:

- archetype templates for each flow
- descriptor-based role assembly
- data contracts for reads and writes
- provider requirements per role
- hard policy and human gates
- missing-evidence and contradiction stop-gates
- outcome records that tie back to renewal and vendor performance

The first implementation fixture should prove F1 through F3 shape, even if F4 is
initially represented as typed output rather than live monitoring.
