# Converge Experience Store — Integration Guide

## What it is

Converge emits `ExperienceEvent`s during every convergence run — fact promotions, outcomes, budget exhaustion. These are the audit trail that organism layers should consume for analytics, debugging, and governance.

## How converge wires it

The converge application binary creates an `InMemoryExperienceStore` and a `StoreObserver` bridge, then attaches it to the engine:

```rust
use converge_kernel::{Engine, EventQuery, ExperienceStore};
use converge_experience::{InMemoryExperienceStore, StoreObserver};
use std::sync::Arc;

let store = Arc::new(InMemoryExperienceStore::new());
let observer = Arc::new(StoreObserver::new(store.clone()));

let mut engine = Engine::new();
engine.set_event_observer(observer);

// ... register suggestors, run engine ...
let result = engine.run(context)?;

// Query captured events
let events = store.query_events(&EventQuery::default())?;
```

Those queried `ExperienceEventEnvelope`s are the canonical input to Organism learning. The adapter now lives in `organism-learning`:

```rust
use organism_pack::{build_episode_from_run, extract_signals_from_run};

let episode = build_episode_from_run(intent_id, plan_id, subject, &result.context, &events);
let signals = extract_signals_from_run(&result.context, &events);
```

`LearningEpisode` now separates the two layers explicitly:
- `actual_outcome` is the governed business outcome that survived promotion
- `run_status` is the terminal engine status from `OutcomeRecorded`

## How organism should wire a database-backed store

Converge ships three backends in `converge-experience`:

| Backend | Feature flag | Use case |
|---|---|---|
| `InMemoryExperienceStore` | (always on) | Dev, tests, short-lived processes |
| `SurrealDbExperienceStore` | `surrealdb` | Production — persistent, queryable |
| `LanceDbExperienceStore` | `lancedb` | Production — vector-indexed similarity search |

### SurrealDB example (recommended for organism)

```toml
# organism-runtime/Cargo.toml
[dependencies]
converge-experience = { workspace = true, features = ["surrealdb"] }
```

```rust
use converge_experience::{SurrealDbConfig, SurrealDbExperienceStore, StoreObserver};
use converge_kernel::Engine;
use std::sync::Arc;

let config = SurrealDbConfig::new(
    "ws://localhost:8000",  // or production SurrealDB endpoint
    "organism",             // namespace
    "experience",           // database
).with_root_auth("root", "root");  // or use env vars

let store = Arc::new(SurrealDbExperienceStore::connect(config)?);
let observer = Arc::new(StoreObserver::new(store.clone()));
engine.set_event_observer(observer);
```

### LanceDB example (for vector similarity over events)

```toml
converge-experience = { workspace = true, features = ["lancedb"] }
```

Same pattern — use `StoreObserver<LanceDbExperienceStore>`. LanceDB adds `VectorEvent` with embeddings for semantic search over the event history.

## Events emitted today

| Event | When | Data |
|---|---|---|
| `FactPromoted` | Each proposal → fact promotion | proposal_id, fact_id, promoted_by, reason |
| `OutcomeRecorded` | Run completes successfully | outcome description, criteria results |
| `BudgetExceeded` | Budget exhaustion | budget kind |

9 additional event variants are defined but not yet emitted by the engine (`ProposalCreated`, `ProposalValidated`, `RecallExecuted`, etc.). These will be wired in future converge releases.

## Querying events

```rust
use converge_kernel::EventQuery;

// All events
let all = store.query_events(&EventQuery::default())?;

// Filter by tenant
let tenant = store.query_events(&EventQuery {
    tenant_id: Some("org-123".to_string()),
    ..Default::default()
})?;

// Filter by correlation (run ID)
let run = store.query_events(&EventQuery {
    correlation_id: Some("run-abc".to_string()),
    ..Default::default()
})?;
```

## What organism should build on top

- **Run history dashboard** — query events per correlation_id for timeline view
- **Audit log** — all FactPromoted events with provenance chain
- **Anomaly detection** — LanceDB vector search over event embeddings
- **Billing/metering** — count events per tenant per time window
- **Learning loop** — query a run's envelopes, then feed them into `build_episode_from_run()` and `extract_signals_from_run()`
