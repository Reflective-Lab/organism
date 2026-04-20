/// Proves that `Fact::new()` does not exist outside the kernel-authority feature.
/// Organism cannot forge facts — it can only propose them.
use converge_pack::{ContextKey, Fact};

fn main() {
    // This must NOT compile: no public `new` without kernel-authority feature.
    let _fact = Fact::new(ContextKey::Seeds, "test-id", "test-content");
}
