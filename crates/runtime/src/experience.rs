//! Run-scoped experience plumbing owned by Organism.
//!
//! Converge emits raw `ExperienceEvent`s from the engine. Organism owns the
//! formation-level tenant and correlation scope, so it wraps events into
//! envelopes before appending them to a store or other sink.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use converge_kernel::{
    CorrelationId, EventId, ExperienceEvent, ExperienceEventEnvelope, ExperienceEventObserver,
    ExperienceStore,
};

pub trait ExperienceEnvelopeSink: Send + Sync {
    fn append_envelope(&self, envelope: ExperienceEventEnvelope);
}

impl<S> ExperienceEnvelopeSink for S
where
    S: ExperienceStore + Send + Sync,
{
    fn append_envelope(&self, envelope: ExperienceEventEnvelope) {
        let _ = self.append_event(envelope);
    }
}

pub struct FormationExperienceObserver<S> {
    sink: Arc<S>,
    tenant_id: Option<String>,
    correlation_id: CorrelationId,
    next_id: AtomicU64,
}

impl<S> FormationExperienceObserver<S>
where
    S: ExperienceEnvelopeSink,
{
    pub fn new(
        sink: Arc<S>,
        tenant_id: Option<String>,
        correlation_id: impl Into<CorrelationId>,
    ) -> Self {
        Self {
            sink,
            tenant_id,
            correlation_id: correlation_id.into(),
            next_id: AtomicU64::new(0),
        }
    }

    pub fn tenant_scoped(
        sink: Arc<S>,
        tenant_id: impl Into<String>,
        correlation_id: impl Into<CorrelationId>,
    ) -> Self {
        Self::new(sink, Some(tenant_id.into()), correlation_id)
    }

    pub fn sink(&self) -> &Arc<S> {
        &self.sink
    }
}

impl<S> ExperienceEventObserver for FormationExperienceObserver<S>
where
    S: ExperienceEnvelopeSink,
{
    fn on_event(&self, event: &ExperienceEvent) {
        let sequence = self.next_id.fetch_add(1, Ordering::Relaxed);
        let event_id = EventId::new(format!("formation:{}:{sequence}", self.correlation_id));
        let mut envelope = ExperienceEventEnvelope::new(event_id, event.clone())
            .with_correlation(self.correlation_id.clone());

        if let Some(tenant_id) = &self.tenant_id {
            envelope = envelope.with_tenant(tenant_id.clone());
        }

        self.sink.append_envelope(envelope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_kernel::{BudgetResource, DecisionStep, ExperienceEvent};
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingSink {
        envelopes: Mutex<Vec<ExperienceEventEnvelope>>,
    }

    impl CapturingSink {
        fn envelopes(&self) -> Vec<ExperienceEventEnvelope> {
            self.envelopes.lock().expect("envelope lock").clone()
        }
    }

    impl ExperienceEnvelopeSink for CapturingSink {
        fn append_envelope(&self, envelope: ExperienceEventEnvelope) {
            self.envelopes.lock().expect("envelope lock").push(envelope);
        }
    }

    #[test]
    fn observer_wraps_event_with_tenant_and_correlation() {
        let sink = Arc::new(CapturingSink::default());
        let observer =
            FormationExperienceObserver::tenant_scoped(sink.clone(), "tenant-a", "corr-1");
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "chain-1".into(),
            resource: BudgetResource::Tokens,
            limit: "100".to_string(),
            observed: Some("120".to_string()),
        };

        observer.on_event(&event);

        let envelopes = sink.envelopes();
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].tenant_id.as_deref(), Some("tenant-a"));
        assert_eq!(envelopes[0].correlation_id.as_deref(), Some("corr-1"));
        assert!(matches!(
            envelopes[0].event,
            ExperienceEvent::BudgetExceeded { .. }
        ));
    }

    #[test]
    fn observer_keeps_sequence_stable_for_run_scope() {
        let sink = Arc::new(CapturingSink::default());
        let observer = FormationExperienceObserver::new(sink.clone(), None, "corr-2");
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".into(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: None,
            tokens: None,
            cost_microdollars: None,
            backend: None,
            metadata: std::collections::HashMap::new(),
        };

        observer.on_event(&event);
        observer.on_event(&event);

        let envelopes = sink.envelopes();
        assert_eq!(envelopes[0].event_id.as_str(), "formation:corr-2:0");
        assert_eq!(envelopes[1].event_id.as_str(), "formation:corr-2:1");
        assert!(envelopes.iter().all(|env| env.tenant_id.is_none()));
    }
}
