//! Organism runtime.
//!
//! Wires intent → planning → adversarial review → simulation → Converge.
//! Owns LLM integration and human-in-the-loop checkpoints.
//!
//! Converge integration: organism crates use `converge-pack`, `converge-kernel`,
//! and `converge-model` directly. The Rust type system enforces the axioms —
//! no wrapper layer needed. For remote deployment, use `converge-client` (the
//! Converge crate) directly.

pub mod readiness;
pub mod registry;

use organism_intent::IntentPacket;

/// Trait for submitting plans to Converge's commit boundary.
///
/// Embedded mode: implement via `converge-kernel` (in-process).
/// Remote mode: implement via `converge-client` (gRPC).
pub trait CommitBoundary: Send + Sync {
    fn submit(
        &self,
        run_id: &str,
        key: &str,
        content: &str,
        provenance: &str,
    ) -> Result<(), String>;
}

/// Top-level orchestrator. A `Runtime` takes an `IntentPacket` and drives it
/// through the full pipeline up to (but not past) the Converge commit boundary.
pub struct Runtime<C: CommitBoundary> {
    pub converge: C,
}

impl<C: CommitBoundary> Runtime<C> {
    pub fn new(converge: C) -> Self {
        Self { converge }
    }

    /// Drive an intent through the pipeline. Stub.
    pub async fn handle(&self, _intent: IntentPacket) -> anyhow::Result<()> {
        // 1. admission control          (intent)
        // 2. decomposition               (intent)
        // 3. huddle                      (planning)
        // 4. adversarial review          (adversarial)
        // 5. simulation swarm            (simulation)
        // 6. submit to commit boundary   (converge-pack / converge-kernel)
        Ok(())
    }
}
