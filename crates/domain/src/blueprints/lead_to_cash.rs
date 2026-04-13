//! Lead-to-cash blueprint.
//!
//! Packs: Customers → Delivery → Legal → Money (converge-domain)
//!
//! Cross-pack invariants:
//! - promise_has_deal: Delivery must reference a Customers deal
//! - closed_won_triggers_handoff: Deal closure triggers CSM
//! - signature_required: Contract must be executed before delivery
//! - invoice_has_customer: Invoice must reference deal
//! - legal_actions_audited: All legal actions auditable
