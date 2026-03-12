//! Tree CRDT for concurrent structural editing.
//!
//! Based on Kleppmann's replicated tree algorithm. Handles concurrent
//! insert, delete, and move operations with cycle detection.
//!
//! - **Insert**: placed under parent, siblings ordered by OpId for tiebreaking.
//! - **Delete**: tombstone (children preserved but invisible).
//! - **Move**: cycle detection — if a move would create a cycle, it is dropped.
//!   Among concurrent non-cyclic moves of the same node, the highest OpId wins (LWW).

use std::collections::{HashMap, HashSet};

use crate::error::CrdtError;
use crate::op_id::OpId;
use s1_model::NodeId;

/// State of a node in the tree CRDT.
#[derive(Debug, Clone)]
struct TreeNodeState {
    /// The operation that placed this node at its current parent.
    parent_op: OpId,
    /// Current parent node ID.
    parent: NodeId,
    /// Whether this node has been deleted (tombstone).
    tombstoned: bool,
    /// The operation that tombstoned this node, if any.
    tombstone_op: Option<OpId>,
}

/// Tracks child ordering under each parent.
#[derive(Debug, Clone)]
struct ChildList {
    /// Children sorted by their placement OpId.
    children: Vec<(OpId, NodeId)>,
}

impl ChildList {
    fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Insert a child, maintaining OpId sort order.
    fn insert(&mut self, op_id: OpId, node_id: NodeId) {
        let pos = self
            .children
            .iter()
            .position(|(id, _)| *id > op_id)
            .unwrap_or(self.children.len());
        self.children.insert(pos, (op_id, node_id));
    }

    /// Remove a child by NodeId.
    fn remove(&mut self, node_id: NodeId) {
        self.children.retain(|(_, nid)| *nid != node_id);
    }

    /// Get ordered child NodeIds.
    fn node_ids(&self) -> Vec<NodeId> {
        self.children.iter().map(|(_, nid)| *nid).collect()
    }
}

/// Tree CRDT managing node relationships and tombstones.
#[derive(Debug, Clone)]
pub struct TreeCrdt {
    /// Per-node state: parent, placement op, tombstone status.
    node_state: HashMap<NodeId, TreeNodeState>,
    /// Per-parent child ordering.
    child_lists: HashMap<NodeId, ChildList>,
}

impl TreeCrdt {
    /// Create a new empty tree CRDT.
    pub fn new() -> Self {
        Self {
            node_state: HashMap::new(),
            child_lists: HashMap::new(),
        }
    }

    /// Register a node that already exists in the model (during init).
    pub fn register_node(&mut self, node_id: NodeId, parent_id: NodeId, op_id: OpId) {
        self.node_state.insert(
            node_id,
            TreeNodeState {
                parent_op: op_id,
                parent: parent_id,
                tombstoned: false,
                tombstone_op: None,
            },
        );
        self.child_lists
            .entry(parent_id)
            .or_insert_with(ChildList::new)
            .insert(op_id, node_id);
    }

    /// Integrate a node insertion.
    ///
    /// Places the node under `parent_id`, ordered among siblings by `op_id`.
    /// Returns `Ok(())` on success.
    pub fn integrate_insert(
        &mut self,
        node_id: NodeId,
        parent_id: NodeId,
        op_id: OpId,
    ) -> Result<(), CrdtError> {
        // Record the node state
        self.node_state.insert(
            node_id,
            TreeNodeState {
                parent_op: op_id,
                parent: parent_id,
                tombstoned: false,
                tombstone_op: None,
            },
        );

        // Add to parent's child list
        self.child_lists
            .entry(parent_id)
            .or_insert_with(ChildList::new)
            .insert(op_id, node_id);

        Ok(())
    }

    /// Integrate a node deletion (tombstone).
    ///
    /// The node becomes invisible but its children are preserved.
    /// If the node is already tombstoned, this is a no-op.
    pub fn integrate_delete(&mut self, node_id: NodeId, op_id: OpId) -> Result<bool, CrdtError> {
        let state = self
            .node_state
            .get_mut(&node_id)
            .ok_or(CrdtError::NodeNotFound(node_id))?;

        if state.tombstoned {
            // Already deleted — check if this is a later delete (LWW)
            if let Some(existing_op) = state.tombstone_op {
                if op_id > existing_op {
                    state.tombstone_op = Some(op_id);
                }
            }
            return Ok(false);
        }

        state.tombstoned = true;
        state.tombstone_op = Some(op_id);
        Ok(true)
    }

    /// Integrate a node move.
    ///
    /// Moves `node_id` under `new_parent_id`. Performs cycle detection.
    /// Among concurrent moves of the same node, the one with the highest
    /// OpId wins (LWW).
    ///
    /// Returns `Ok(true)` if the move was applied, `Ok(false)` if dropped.
    pub fn integrate_move(
        &mut self,
        node_id: NodeId,
        new_parent_id: NodeId,
        op_id: OpId,
    ) -> Result<bool, CrdtError> {
        let state = self
            .node_state
            .get(&node_id)
            .ok_or(CrdtError::NodeNotFound(node_id))?;

        // LWW: only apply if this is a newer operation
        if op_id < state.parent_op {
            return Ok(false);
        }

        // Cycle detection: new_parent cannot be a descendant of node_id
        if self.is_ancestor(node_id, new_parent_id) {
            return Ok(false); // Drop cyclic move
        }

        let old_parent = state.parent;

        // Remove from old parent's child list
        if let Some(children) = self.child_lists.get_mut(&old_parent) {
            children.remove(node_id);
        }

        // Update state
        let state = self.node_state.get_mut(&node_id).unwrap();
        state.parent = new_parent_id;
        state.parent_op = op_id;

        // Add to new parent's child list
        self.child_lists
            .entry(new_parent_id)
            .or_insert_with(ChildList::new)
            .insert(op_id, node_id);

        Ok(true)
    }

    /// Check if `ancestor_id` is an ancestor of `descendant_id`.
    pub fn is_ancestor(&self, ancestor_id: NodeId, descendant_id: NodeId) -> bool {
        if ancestor_id == descendant_id {
            return true;
        }

        let mut visited = HashSet::new();
        let mut current = descendant_id;

        while let Some(state) = self.node_state.get(&current) {
            if !visited.insert(current) {
                return false; // Cycle protection
            }
            if state.parent == ancestor_id {
                return true;
            }
            current = state.parent;
        }

        false
    }

    /// Get the visible (non-tombstoned) children of a parent node.
    pub fn visible_children(&self, parent_id: NodeId) -> Vec<NodeId> {
        self.child_lists
            .get(&parent_id)
            .map(|children| {
                children
                    .node_ids()
                    .into_iter()
                    .filter(|nid| {
                        self.node_state
                            .get(nid)
                            .map(|s| !s.tombstoned)
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all children (including tombstoned) of a parent node.
    pub fn all_children(&self, parent_id: NodeId) -> Vec<NodeId> {
        self.child_lists
            .get(&parent_id)
            .map(|children| children.node_ids())
            .unwrap_or_default()
    }

    /// Check if a node is tombstoned.
    pub fn is_tombstoned(&self, node_id: NodeId) -> bool {
        self.node_state
            .get(&node_id)
            .map(|s| s.tombstoned)
            .unwrap_or(false)
    }

    /// Get a node's current parent.
    pub fn parent_of(&self, node_id: NodeId) -> Option<NodeId> {
        self.node_state.get(&node_id).map(|s| s.parent)
    }

    /// Check if a node is tracked.
    pub fn has_node(&self, node_id: NodeId) -> bool {
        self.node_state.contains_key(&node_id)
    }

    /// Number of tracked nodes.
    pub fn node_count(&self) -> usize {
        self.node_state.len()
    }
}

impl Default for TreeCrdt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(r: u64, c: u64) -> NodeId {
        NodeId::new(r, c)
    }

    #[test]
    fn insert_and_visible_children() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let child_a = nid(1, 1);
        let child_b = nid(2, 2);

        tree.integrate_insert(child_a, root, OpId::new(1, 1))
            .unwrap();
        tree.integrate_insert(child_b, root, OpId::new(2, 2))
            .unwrap();

        let children = tree.visible_children(root);
        assert_eq!(children, vec![child_a, child_b]); // ordered by OpId
    }

    #[test]
    fn insert_ordering_by_op_id() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);

        // Insert in reverse OpId order
        tree.integrate_insert(nid(1, 3), root, OpId::new(1, 10))
            .unwrap();
        tree.integrate_insert(nid(1, 1), root, OpId::new(1, 1))
            .unwrap();
        tree.integrate_insert(nid(1, 2), root, OpId::new(1, 5))
            .unwrap();

        let children = tree.visible_children(root);
        assert_eq!(children, vec![nid(1, 1), nid(1, 2), nid(1, 3)]);
    }

    #[test]
    fn delete_tombstones_node() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let child = nid(1, 1);

        tree.integrate_insert(child, root, OpId::new(1, 1)).unwrap();
        assert_eq!(tree.visible_children(root).len(), 1);

        tree.integrate_delete(child, OpId::new(1, 2)).unwrap();
        assert!(tree.visible_children(root).is_empty());
        assert!(tree.is_tombstoned(child));
    }

    #[test]
    fn delete_idempotent() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let child = nid(1, 1);

        tree.integrate_insert(child, root, OpId::new(1, 1)).unwrap();
        assert!(tree.integrate_delete(child, OpId::new(1, 2)).unwrap());
        assert!(!tree.integrate_delete(child, OpId::new(2, 3)).unwrap()); // already deleted
    }

    #[test]
    fn delete_preserves_children() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let parent = nid(1, 1);
        let child = nid(1, 2);

        tree.integrate_insert(parent, root, OpId::new(1, 1))
            .unwrap();
        tree.integrate_insert(child, parent, OpId::new(1, 2))
            .unwrap();

        tree.integrate_delete(parent, OpId::new(1, 3)).unwrap();

        // Parent is tombstoned
        assert!(tree.is_tombstoned(parent));
        // Child is NOT tombstoned
        assert!(!tree.is_tombstoned(child));
        // Child is still under parent
        assert_eq!(tree.parent_of(child), Some(parent));
    }

    #[test]
    fn move_node() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let parent_a = nid(1, 1);
        let parent_b = nid(1, 2);
        let child = nid(1, 3);

        tree.integrate_insert(parent_a, root, OpId::new(1, 1))
            .unwrap();
        tree.integrate_insert(parent_b, root, OpId::new(1, 2))
            .unwrap();
        tree.integrate_insert(child, parent_a, OpId::new(1, 3))
            .unwrap();

        assert_eq!(tree.visible_children(parent_a), vec![child]);
        assert!(tree.visible_children(parent_b).is_empty());

        tree.integrate_move(child, parent_b, OpId::new(1, 4))
            .unwrap();

        assert!(tree.visible_children(parent_a).is_empty());
        assert_eq!(tree.visible_children(parent_b), vec![child]);
    }

    #[test]
    fn move_cycle_detection() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let a = nid(1, 1);
        let b = nid(1, 2);

        tree.integrate_insert(a, root, OpId::new(1, 1)).unwrap();
        tree.integrate_insert(b, a, OpId::new(1, 2)).unwrap();

        // Try to move A under B (would create cycle: A->B->A)
        let applied = tree.integrate_move(a, b, OpId::new(1, 3)).unwrap();
        assert!(!applied); // Cycle detected, move dropped
        assert_eq!(tree.parent_of(a), Some(root)); // A still under root
    }

    #[test]
    fn concurrent_move_lww() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let dest_a = nid(1, 1);
        let dest_b = nid(1, 2);
        let child = nid(1, 3);

        tree.integrate_insert(dest_a, root, OpId::new(1, 1))
            .unwrap();
        tree.integrate_insert(dest_b, root, OpId::new(1, 2))
            .unwrap();
        tree.integrate_insert(child, root, OpId::new(1, 3)).unwrap();

        // Two concurrent moves: one to dest_a (lower OpId), one to dest_b (higher)
        tree.integrate_move(child, dest_a, OpId::new(1, 4)).unwrap();
        tree.integrate_move(child, dest_b, OpId::new(2, 5)).unwrap();

        // Higher OpId wins
        assert_eq!(tree.parent_of(child), Some(dest_b));
        assert_eq!(tree.visible_children(dest_b), vec![child]);
    }

    #[test]
    fn move_with_older_op_ignored() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let dest = nid(1, 1);
        let child = nid(1, 2);

        tree.integrate_insert(dest, root, OpId::new(1, 1)).unwrap();
        tree.integrate_insert(child, root, OpId::new(1, 5)).unwrap(); // placed at lamport 5

        // Try to move with older lamport — should be ignored
        let applied = tree.integrate_move(child, dest, OpId::new(1, 3)).unwrap();
        assert!(!applied);
        assert_eq!(tree.parent_of(child), Some(root));
    }

    #[test]
    fn is_ancestor() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let a = nid(1, 1);
        let b = nid(1, 2);
        let c = nid(1, 3);

        tree.integrate_insert(a, root, OpId::new(1, 1)).unwrap();
        tree.integrate_insert(b, a, OpId::new(1, 2)).unwrap();
        tree.integrate_insert(c, b, OpId::new(1, 3)).unwrap();

        assert!(tree.is_ancestor(a, c)); // a is ancestor of c
        assert!(tree.is_ancestor(a, b)); // a is ancestor of b
        assert!(!tree.is_ancestor(c, a)); // c is NOT ancestor of a
        assert!(tree.is_ancestor(a, a)); // self is ancestor
    }

    #[test]
    fn register_existing_node() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let child = nid(0, 1);

        tree.register_node(child, root, OpId::new(0, 1));
        assert!(tree.has_node(child));
        assert_eq!(tree.parent_of(child), Some(root));
        assert_eq!(tree.visible_children(root), vec![child]);
    }

    #[test]
    fn node_count() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        assert_eq!(tree.node_count(), 0);

        tree.integrate_insert(nid(1, 1), root, OpId::new(1, 1))
            .unwrap();
        tree.integrate_insert(nid(1, 2), root, OpId::new(1, 2))
            .unwrap();
        assert_eq!(tree.node_count(), 2);
    }

    #[test]
    fn delete_nonexistent_node() {
        let mut tree = TreeCrdt::new();
        let result = tree.integrate_delete(nid(99, 99), OpId::new(1, 1));
        assert!(result.is_err());
    }

    #[test]
    fn all_children_includes_tombstoned() {
        let mut tree = TreeCrdt::new();
        let root = nid(0, 0);
        let a = nid(1, 1);
        let b = nid(1, 2);

        tree.integrate_insert(a, root, OpId::new(1, 1)).unwrap();
        tree.integrate_insert(b, root, OpId::new(1, 2)).unwrap();
        tree.integrate_delete(a, OpId::new(1, 3)).unwrap();

        assert_eq!(tree.all_children(root), vec![a, b]);
        assert_eq!(tree.visible_children(root), vec![b]);
    }
}
