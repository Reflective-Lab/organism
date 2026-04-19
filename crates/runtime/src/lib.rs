//! # Organism Runtime
//!
//! This crate is the curated in-process execution API for Organism.
//! Consumers model planning semantics in `organism-pack`, then use
//! `organism-runtime` to resolve packs, check readiness, and wire the
//! planning loop to Converge.
//!
//! Converge integration: organism crates use `converge-pack`,
//! `converge-kernel`, and `converge-model` directly. The Rust type system
//! enforces the axioms — no wrapper layer needed. For remote deployment,
//! use `converge-client` directly.

pub mod collaboration;
pub mod readiness;
pub mod registry;

pub use collaboration::{CollaborationParticipant, CollaborationRunner, CollaborationRunnerError};
pub use organism_pack::{
    CapabilityRequirement, DeclarativeBinding, IntentBinding, IntentResolver, PackRequirement,
    ResolutionLevel, ResolutionTrace,
};
pub use readiness::{
    BudgetProbe, CredentialProbe, GapSeverity, PackProbe, ReadinessConfirmation, ReadinessGap,
    ReadinessItem, ReadinessProbe, ReadinessReport, ResourceKind, check as check_readiness,
};
pub use registry::{RegisteredCapability, RegisteredPack, Registry, StructuralResolver};

use organism_pack::IntentPacket;

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
