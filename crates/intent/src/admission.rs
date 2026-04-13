//! Admission control — feasibility gate before any planning begins.
//!
//! Cheap checks that filter out obviously infeasible, expired, or forbidden
//! intents. Admission control deliberately knows nothing about plans —
//! it only inspects the [`IntentPacket`] itself.

use chrono::Utc;

use crate::{IntentError, IntentPacket};

/// Result of an admission decision.
#[derive(Debug)]
pub enum Admission {
    Admit,
    Reject(IntentError),
}

/// Run admission control over an intent packet.
pub fn admit(intent: &IntentPacket) -> Admission {
    if intent.is_expired(Utc::now()) {
        return Admission::Reject(IntentError::Expired(intent.expires));
    }
    if intent.outcome.trim().is_empty() {
        return Admission::Reject(IntentError::Infeasible("empty outcome".into()));
    }
    Admission::Admit
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn rejects_expired() {
        let intent = IntentPacket::new("ship q3", Utc::now() - Duration::seconds(1));
        assert!(matches!(admit(&intent), Admission::Reject(_)));
    }

    #[test]
    fn rejects_empty_outcome() {
        let intent = IntentPacket::new("", Utc::now() + Duration::hours(1));
        assert!(matches!(admit(&intent), Admission::Reject(_)));
    }

    #[test]
    fn admits_valid() {
        let intent = IntentPacket::new("ship q3", Utc::now() + Duration::hours(1));
        assert!(matches!(admit(&intent), Admission::Admit));
    }
}
