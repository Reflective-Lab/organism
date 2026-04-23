/// Proves that `Fact` has no public unchecked constructor outside Converge.
/// Organism cannot forge facts — it can only propose them.
use converge_pack::{ContextKey, Fact};

fn main() {
    // This must NOT compile: Organism cannot mint governed facts directly.
    let _fact = Fact::construct_unchecked(ContextKey::Seeds, "test-id", "test-content");
}
