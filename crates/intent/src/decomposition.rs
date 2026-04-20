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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn future() -> chrono::DateTime<Utc> {
        Utc::now() + Duration::hours(1)
    }

    #[test]
    fn leaf_has_no_children() {
        let node = IntentNode::leaf(IntentPacket::new("task", future()));
        assert!(node.children.is_empty());
        assert_eq!(node.intent.outcome, "task");
    }

    #[test]
    fn walk_visits_single_leaf() {
        let node = IntentNode::leaf(IntentPacket::new("only", future()));
        let mut visited = Vec::new();
        node.walk(&mut |n| visited.push(n.intent.outcome.clone()));
        assert_eq!(visited, vec!["only"]);
    }

    #[test]
    fn walk_visits_depth_first() {
        let child_a = IntentNode::leaf(IntentPacket::new("a", future()));
        let child_b = IntentNode::leaf(IntentPacket::new("b", future()));
        let grandchild = IntentNode::leaf(IntentPacket::new("gc", future()));
        let child_c = IntentNode {
            intent: IntentPacket::new("c", future()),
            children: vec![grandchild],
        };
        let root = IntentNode {
            intent: IntentPacket::new("root", future()),
            children: vec![child_a, child_b, child_c],
        };

        let mut visited = Vec::new();
        root.walk(&mut |n| visited.push(n.intent.outcome.clone()));
        assert_eq!(visited, vec!["root", "a", "b", "c", "gc"]);
    }

    #[test]
    fn walk_empty_tree() {
        let root = IntentNode::leaf(IntentPacket::new("alone", future()));
        let mut count = 0;
        root.walk(&mut |_| count += 1);
        assert_eq!(count, 1);
    }

    #[test]
    fn walk_deep_tree() {
        let mut current = IntentNode::leaf(IntentPacket::new("leaf", future()));
        for i in 0..10 {
            current = IntentNode {
                intent: IntentPacket::new(format!("level-{i}"), future()),
                children: vec![current],
            };
        }
        let mut count = 0;
        current.walk(&mut |_| count += 1);
        assert_eq!(count, 11);
    }

    #[test]
    fn walk_wide_tree() {
        let children: Vec<IntentNode> = (0..5)
            .map(|i| IntentNode::leaf(IntentPacket::new(format!("child-{i}"), future())))
            .collect();
        let root = IntentNode {
            intent: IntentPacket::new("root", future()),
            children,
        };
        let mut count = 0;
        root.walk(&mut |_| count += 1);
        assert_eq!(count, 6);
    }

    #[test]
    fn clone_preserves_structure() {
        let child = IntentNode::leaf(IntentPacket::new("child", future()));
        let parent = IntentNode {
            intent: IntentPacket::new("parent", future()),
            children: vec![child],
        };
        let cloned = parent.clone();
        assert_eq!(cloned.children.len(), 1);
        assert_eq!(cloned.children[0].intent.outcome, "child");
    }
}
