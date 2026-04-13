# organism-application

Business-level applications built on the Organism organizational runtime.

## What belongs here?

Application-level tools that use Organism's planning, adversarial, and execution capabilities for specific end-user scenarios. These are distinct from the kernel distribution layer (`converge-application`).

## Applications

- **Form Filler** — Form completion automation (TUI + WebDriver). To be moved from converge-application/formfiller.
- **Course Planner** — University course application planner (PDF-first). To be moved from converge-application/university-course-application.

## Application Agents

Agent implementations that belong at the organism layer:
- `StrategicInsightAgent` — LLM-powered strategic synthesis
- `RiskAssessmentAgent` — Risk assessment agent

## Architecture

```
organism-application
    ↓ uses
organism-domain (business domain packs + blueprints)
    ↓ uses
organism-core (intent, planning, adversarial, authority)
    ↓ runs on
converge-core (convergence engine, context, gates)
```
