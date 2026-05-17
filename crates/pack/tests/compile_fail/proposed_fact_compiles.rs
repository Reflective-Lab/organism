/// Proves that `ProposedFact::new()` IS publicly available.
/// This is the intended path: organism proposes, Converge decides.
use converge_pack::{ContextKey, ProposedFact, TextPayload};

fn main() {
    let proposal = ProposedFact::new(
        ContextKey::Seeds,
        "my-proposal",
        TextPayload::new("some content"),
        "organism-planning",
    );
    assert_eq!(proposal.text(), Some("some content"));
}
