/// Proves that direct `Fact` construction is impossible from organism code.
/// The `Fact` type has private fields — only Converge's kernel can create facts
/// through the promotion gate.
use converge_pack::{ContextKey, Fact, FactPromotionRecord};

fn main() {
    // This must NOT compile: `key` field is private.
    let _fact = Fact {
        key: ContextKey::Seeds,
        id: "forged-fact".into(),
        content: "bypassed governance".into(),
        promotion_record: todo!(),
        created_at: "2026-01-01T00:00:00Z".into(),
    };
}
