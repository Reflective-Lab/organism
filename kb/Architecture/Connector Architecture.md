---
tags: [architecture, integrations]
source: mixed
---
# Connector Architecture

How Organism bridges the gap between its planning loop and the ecosystem of
legacy system connectors being built by the community.

## The Problem

Organism reasons, plans, and simulates — but eventually needs to touch the
real world: CRMs, ERPs, ticketing systems, billing platforms, HR tools. Building
every connector in-house is not viable. The ecosystem is building them. The
question is how to consume them cleanly.

## Three Tiers

### Tier 1 — Tools (generic, ecosystem-built)

For any legacy system where the interaction is essentially "call this endpoint
with these parameters and get data back," community-built connectors plug in
as **Tools** discovered by Converge's ToolRegistry.

```
Legacy System → MCP Server (community) → Converge ToolRegistry → Planning loop
Legacy System → OpenAPI spec            → Converge ToolRegistry → Planning loop
Legacy System → GraphQL endpoint        → Converge ToolRegistry → Planning loop
```

Organism never imports these. They appear as available actions in the planning
loop via `ToolDefinition` from `converge-provider`.

**Examples:** Salesforce CRUD, Jira ticket creation, Slack messaging, calendar
scheduling, generic database queries.

### Tier 2 — Organism Ports (domain-semantic, typed)

For systems where the raw API response needs **domain interpretation** —
multi-step workflows, semantic enrichment, or cross-system correlation — the
connector lives as a typed trait in `organism-intelligence`.

```rust
pub trait LinkedInProvider: Send + Sync {
    fn get(&self, request: &LinkedInGetRequest, ctx: &CallContext)
        -> Result<LinkedInGetResponse, String>;
}
```

These traits carry provenance (`Observation<T>`) and feed into the learning
system. They are not thin HTTP wrappers — they add semantic value.

**Examples:** LinkedIn profile enrichment with citation graph, patent search
with prior-art analysis, OCR with document structure understanding, social
profile normalization across platforms.

### Tier 3 — Converge Providers (interchangeable backends)

LLM and embedding backends. These are **not connectors** — they are
interchangeable implementations of the same capability (chat, embed, search).

Owned entirely by `converge-provider`. Organism never defines providers.

## Decision Rule

| Question | Answer | Tier |
|----------|--------|------|
| Is the interaction generic CRUD / data fetch? | Yes | Tier 1 (Tool) |
| Does the response need domain reasoning to interpret? | Yes | Tier 2 (Port) |
| Is it an interchangeable AI/ML backend? | Yes | Tier 3 (Provider) |

## What Organism Does NOT Do

- Build generic API connectors (the ecosystem handles this)
- Wrap Converge's ToolRegistry (use it directly)
- Own the tool discovery mechanism (Converge owns MCP/OpenAPI/GraphQL discovery)
- Maintain connector SDKs for third-party systems

## What Organism DOES Do

- Define typed port traits for domain-semantic integrations
- Wrap observations with provenance for learning
- Expose tools to the planning loop as available actions
- Let the adversarial layer challenge tool usage before commit

## Strategic Intent: API-Only Infrastructure

The SaaS solutions built on this stack are **API-only infrastructure**. Organism
is not a monolithic application — it is a composable intelligence API that
others build on top of.

This shapes the connector architecture:

- Organism's intelligence capabilities are exposed as API surfaces, not locked
  behind a product UI
- External systems connect via standard protocols (MCP, OpenAPI, GraphQL) at the
  Converge layer — no proprietary connector format
- The planning loop, adversarial review, and simulation are services that any
  API consumer can invoke
- Port traits are the typed contract for domain-semantic APIs, not internal
  implementation details

## Converge Integration Point

Converge's `ToolSource` enum (in `converge-provider/src/tools/definition.rs`):

```rust
pub enum ToolSource {
    Mcp { server_name: String, server_uri: String },
    OpenApi { spec_path: String, operation_id: String, method: String, path: String },
    GraphQl { endpoint: String, operation_name: String, operation_type: GraphQlOperationType },
    Inline,
}
```

This is the bridge. Community connectors register as MCP servers or expose
OpenAPI specs. Converge discovers them. Organism plans with them.

## Data Flow Through All Tiers

```
Intent arrives
  → Planning loop considers available Tools + Port capabilities
  → Adversarial review challenges proposed tool usage
  → Simulation estimates cost/risk of external calls
  → Approved plan submitted to Converge commit boundary
  → Converge executes (via ToolHandler or Port implementation)
  → Observation recorded with provenance
  → Learning adapts priors for next cycle
```

## Relationship to Helms Port Taxonomy

Helms (the application layer) composes Organism ports into application-facing
seams. See `helms/kb/Architecture/Port Capability Taxonomy.md` for the
layering rule:

- Reusable capabilities → `organism-intelligence`
- Application orchestration → Helms application ports
- Generic connectivity → Converge ToolRegistry

See also: [[Converge Contract]], [[API Surfaces]], [[Two-Sided Capabilities]]
