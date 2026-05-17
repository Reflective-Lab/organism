#![allow(unreachable_code)]

/// Proves that direct `ContextFact` construction is impossible from organism code.
/// `ContextFact` has private fields — only Converge's kernel can create context
/// facts through the admission/promotion path.
use converge_pack::{ContextFact, ContextKey};

fn main() {
    // This must NOT compile: every field is private.
    let _fact = ContextFact {
        key: ContextKey::Seeds,
        id: "forged-fact".into(),
        payload: todo!(),
        promotion_record: todo!(),
        created_at: "2026-01-01T00:00:00Z".into(),
    };
}
