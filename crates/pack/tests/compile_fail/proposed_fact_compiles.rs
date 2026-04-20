/// Proves that `ProposedFact::new()` IS publicly available.
/// This is the intended path: organism proposes, Converge decides.
use converge_pack::{ContextKey, ProposedFact};

fn main() {
    let proposal = ProposedFact::new(
        ContextKey::Seeds,
        "my-proposal",
        "some content",
        "organism-planning",
    );
    assert_eq!(proposal.content, "some content");
}
