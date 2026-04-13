---
tags: [philosophy]
---
# Key Invariants

These cannot be broken. If a design decision violates one, the decision is wrong.

## 1. Authority is Recomputed at the Commit Boundary

Planning code must not assume that producing a plan grants the right to execute it. Authority belongs to Converge, not to Organism.

## 2. No Plan Commits Without Passing Both Gates

Every candidate plan must pass adversarial review AND simulation before reaching the converge-client. No shortcut. No "trusted plan" exception.

## 3. Learning Flows Backward, Authority Flows Forward

Learning signals flow from execution outcomes back into planning priors. They never flow directly into authority. Organism learns to plan better — it doesn't learn to bypass governance.

## 4. Reasoning, Planning, Governance, Execution Are Separate Layers

Do not collapse them for convenience. Intent interpretation is not planning. Planning is not governance. Governance is not execution. Each has its own crate, its own types, its own boundary.

See also: [[Philosophy/Relationship to Converge]], [[Concepts/Intent Pipeline]]
