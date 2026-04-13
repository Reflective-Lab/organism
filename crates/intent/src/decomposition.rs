//! Intent decomposition — breaks an intent into a governed tree of sub-intents.
//!
//! Each node in the tree carries its own bounded authority slice. The bounds
//! never expand as the tree deepens; child authority is always a subset of
//! parent authority.

use crate::IntentPacket;

/// A node in the decomposition tree.
#[derive(Debug, Clone)]
pub struct IntentNode {
    pub intent: IntentPacket,
    pub children: Vec<IntentNode>,
}

impl IntentNode {
    pub fn leaf(intent: IntentPacket) -> Self {
        Self {
            intent,
            children: Vec::new(),
        }
    }

    /// Walk every node in the tree (depth-first).
    pub fn walk<'a>(&'a self, f: &mut impl FnMut(&'a IntentNode)) {
        f(self);
        for child in &self.children {
            child.walk(f);
        }
    }
}
