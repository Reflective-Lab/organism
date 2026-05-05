/// Proves that `ContextFact` has no public unchecked constructor outside Converge.
/// Organism cannot forge context facts — it can only propose via `ProposedFact`.
use converge_pack::{ContextFact, ContextKey};

fn main() {
    // This must NOT compile: Organism cannot mint governed context facts directly.
    let _fact =
        ContextFact::construct_unchecked(ContextKey::Seeds, "test-id", "test-content");
}
