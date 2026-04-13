# organism-domain

Business domain packs, blueprints, and use cases for the Organism organizational intelligence runtime.

## What belongs here vs converge-domain?

**converge-domain** holds kernel packs — immutable data structures and deterministic state machines that are domain-agnostic:
- Money (invoices, payments, ledger)
- Trust (audit, access, provenance)
- Delivery (promises, scope, acceptance)
- Knowledge (signals, hypotheses, experiments, decisions)

**organism-domain** (this crate) holds business-specific logic that:
- Changes with business strategy (lead scoring, budget thresholds, approval chains)
- Varies by company, vertical, or GTM model
- Requires domain expertise to configure correctly
- Is often customized per deployment

## Packs

| Pack | Domain |
|------|--------|
| `autonomous_org` | Governance: policies, approvals, budgets, exceptions, delegations |
| `customers` | Revenue ops: lead scoring, routing, qualification, closing |
| `people` | People lifecycle: hiring, onboarding, payroll, reviews |
| `performance` | Performance: review cycles, feedback, calibration |
| `growth_marketing` | Marketing: campaigns, channels, audiences, attribution |
| `legal` | Legal: contracts, equity, IP, signatures |
| `product_engineering` | Product: features, releases, incidents, tech debt |
| `ops_support` | Support: ticket intake, triage, resolution, prevention |
| `procurement_assets` | Procurement: requests, approvals, purchases, tracking |
| `partnerships_vendors` | Vendors: sourcing, evaluation, contracting, monitoring |
| `virtual_teams` | Teams: formation, personas, publishing, audit |
| `linkedin_research` | Research: signals, evidence, dossiers, outreach |
| `reskilling` | Skills: assessment, learning plans, credentials |

## Blueprints

Multi-pack workflow orchestrations that wire kernel packs + organism packs:

| Blueprint | Flow |
|-----------|------|
| `lead_to_cash` | Customers → Delivery → Legal → Money |
| `hire_to_retire` | People → Legal → Money |
| `procure_to_pay` | Procurement → Legal → Money |
| `issue_to_resolution` | Ops Support → Knowledge |
| `idea_to_launch` | Product Engineering → Delivery |
| `campaign_to_revenue` | Growth Marketing → Customers → Money |
| `partner_to_value` | Partnerships → Legal → Delivery |
| `patent_research` | Knowledge → Legal → IP pipeline |

## Use Cases

Applied domain agents for specific business scenarios, with both deterministic and LLM-powered variants.
