# Converge Experience Store — Integration Guide

## What it is

Converge emits `ExperienceEvent`s during every convergence run — fact promotions, outcomes, budget exhaustion. These are the audit trail that organism layers should consume for analytics, debugging, and governance.

## How converge wires it (as of v3.0.2)

The converge application binary creates an `InMemoryExperienceStore` and a `StoreObserver` bridge, then attaches it to the engine:

```rust
use converge_core::{Engine, ExperienceStore};
use converge_experience::{InMemoryExperienceStore, StoreObserver};
use std::sync::Arc;

let store = Arc::new(InMemoryExperienceStore::new());
let observer = Arc::new(StoreObserver::new(store.clone()));

let mut engine = Engine::new();
engine.set_event_observer(observer);

// ... register suggestors, run engine ...
let result = engine.run(context)?;

// Query captured events
let events = store.query_events(&converge_core::EventQuery::default())?;
```

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
use converge_core::Engine;
use std::sync::Arc;

let config = SurrealDbConfig::new(
    "ws://localhost:8000",  // or production SurrealDB endpoint
    "organism",             // namespace
    "experience",           // database
).with_root_auth("root", "root");  // or use env vars

let store = Arc::new(SurrealDbExperienceStore::connect(config)?);
// StoreObserver currently only wraps InMemoryExperienceStore.
// To use SurrealDB, organism should implement ExperienceEventObserver
// that calls store.append_event() directly:

use converge_core::{ExperienceEvent, ExperienceEventEnvelope, ExperienceEventObserver, ExperienceStore};
use std::sync::atomic::{AtomicU64, Ordering};

struct SurrealObserver {
    store: Arc<SurrealDbExperienceStore>,
    next_id: AtomicU64,
}

impl ExperienceEventObserver for SurrealObserver {
    fn on_event(&self, event: &ExperienceEvent) {
        let id = format!("evt-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let envelope = ExperienceEventEnvelope::new(id, event.clone());
        let _ = self.store.append_event(envelope);
    }
}

let observer = Arc::new(SurrealObserver {
    store: store.clone(),
    next_id: AtomicU64::new(0),
});
engine.set_event_observer(observer);
```

### LanceDB example (for vector similarity over events)

```toml
converge-experience = { workspace = true, features = ["lancedb"] }
```

Same pattern — implement `ExperienceEventObserver` wrapping `LanceDbExperienceStore`. LanceDB adds `VectorEvent` with embeddings for semantic search over the event history.

## Events emitted today

| Event | When | Data |
|---|---|---|
| `FactPromoted` | Each proposal → fact promotion | proposal_id, fact_id, promoted_by, reason |
| `OutcomeRecorded` | Run completes successfully | outcome description, criteria results |
| `BudgetExceeded` | Budget exhaustion | budget kind |

9 additional event variants are defined but not yet emitted by the engine (`ProposalCreated`, `ProposalValidated`, `RecallExecuted`, etc.). These will be wired in future converge releases.

## Querying events

```rust
use converge_core::EventQuery;

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
