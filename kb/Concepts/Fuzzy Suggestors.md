# Fuzzy Suggestors

`organism-planning::FuzzyInferenceSuggestor` is the reusable in-loop adapter for
fuzzy work.

The split is:

- Prism owns membership functions, rule evaluation, and activated-rule math.
- Organism owns the Converge `Suggestor` shape and Formation participation.
- Apps own domain variables, input extraction, proposal ids, and typed payloads.

This keeps downstream work as normal converging loops. A fuzzy score is not a
side calculation; it is proposed evidence inside the same engine boundary as
LLM, solver, policy, retrieval, and adversarial Suggestors.

The adapter supports two modes:

- membership-only traces for linguistic-variable grading;
- full Mamdani rule traces when the app provides fuzzy rules.

Apps should configure the adapter rather than implement their own fuzzy
`Suggestor` unless they need materially different convergence behavior.
