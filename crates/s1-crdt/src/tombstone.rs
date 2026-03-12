//! Tombstone management for deleted CRDT items.
//!
//! In a CRDT, deleted items are not physically removed — they are marked as
//! tombstones. This module provides garbage collection for tombstones that
//! all replicas have acknowledged.

use std::collections::{HashMap, HashSet};

use crate::op_id::OpId;
use crate::state_vector::StateVector;
use s1_model::NodeId;

/// Tracks tombstoned items across the CRDT subsystem.
#[derive(Debug, Clone)]
pub struct TombstoneTracker {
    /// Tombstoned text characters: (node_id, char_op_id) -> deleting OpId.
    text_tombstones: HashMap<(NodeId, OpId), OpId>,
    /// Tombstoned tree nodes: node_id -> deleting OpId.
    tree_tombstones: HashMap<NodeId, OpId>,
}

impl TombstoneTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            text_tombstones: HashMap::new(),
            tree_tombstones: HashMap::new(),
        }
    }

    /// Record a text character tombstone.
    pub fn add_text_tombstone(&mut self, node_id: NodeId, char_id: OpId, deleted_by: OpId) {
        self.text_tombstones.insert((node_id, char_id), deleted_by);
    }

    /// Record a tree node tombstone.
    pub fn add_tree_tombstone(&mut self, node_id: NodeId, deleted_by: OpId) {
        self.tree_tombstones.insert(node_id, deleted_by);
    }

    /// Check if a tree node is tombstoned.
    pub fn is_tree_tombstoned(&self, node_id: NodeId) -> bool {
        self.tree_tombstones.contains_key(&node_id)
    }

    /// Check if a text character is tombstoned.
    pub fn is_text_tombstoned(&self, node_id: NodeId, char_id: OpId) -> bool {
        self.text_tombstones.contains_key(&(node_id, char_id))
    }

    /// Number of text tombstones.
    pub fn text_tombstone_count(&self) -> usize {
        self.text_tombstones.len()
    }

    /// Number of tree tombstones.
    pub fn tree_tombstone_count(&self) -> usize {
        self.tree_tombstones.len()
    }

    /// Garbage-collect tombstones that all replicas have acknowledged.
    ///
    /// An item can be GC'd when the minimum state vector across all known
    /// replicas includes the operation that deleted it. Returns the number
    /// of tombstones removed.
    pub fn gc(&mut self, min_state: &StateVector) -> usize {
        let mut removed = 0;

        self.text_tombstones.retain(|_, deleted_by| {
            if min_state.includes(*deleted_by) {
                removed += 1;
                false
            } else {
                true
            }
        });

        self.tree_tombstones.retain(|_, deleted_by| {
            if min_state.includes(*deleted_by) {
                removed += 1;
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get all tombstoned tree node IDs.
    pub fn tombstoned_tree_nodes(&self) -> HashSet<NodeId> {
        self.tree_tombstones.keys().copied().collect()
    }
}

impl Default for TombstoneTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_text_tombstone() {
        let mut tracker = TombstoneTracker::new();
        let node_id = NodeId::new(0, 5);
        let char_id = OpId::new(1, 3);
        let deleter = OpId::new(2, 7);

        tracker.add_text_tombstone(node_id, char_id, deleter);
        assert!(tracker.is_text_tombstoned(node_id, char_id));
        assert!(!tracker.is_text_tombstoned(node_id, OpId::new(1, 4)));
        assert_eq!(tracker.text_tombstone_count(), 1);
    }

    #[test]
    fn track_tree_tombstone() {
        let mut tracker = TombstoneTracker::new();
        let node_id = NodeId::new(1, 5);
        let deleter = OpId::new(2, 3);

        tracker.add_tree_tombstone(node_id, deleter);
        assert!(tracker.is_tree_tombstoned(node_id));
        assert!(!tracker.is_tree_tombstoned(NodeId::new(1, 6)));
        assert_eq!(tracker.tree_tombstone_count(), 1);
    }

    #[test]
    fn gc_removes_acknowledged() {
        let mut tracker = TombstoneTracker::new();
        let node_a = NodeId::new(0, 1);
        let node_b = NodeId::new(0, 2);

        // Tombstone A was deleted by op(1, 5), B by op(2, 10)
        tracker.add_tree_tombstone(node_a, OpId::new(1, 5));
        tracker.add_tree_tombstone(node_b, OpId::new(2, 10));

        // Min state: all replicas have seen up to (1:5, 2:8)
        let mut min_state = StateVector::new();
        min_state.set(1, 5);
        min_state.set(2, 8);

        let removed = tracker.gc(&min_state);
        assert_eq!(removed, 1); // Only A (deleted by op(1,5) <= state[1]=5)
        assert!(!tracker.is_tree_tombstoned(node_a));
        assert!(tracker.is_tree_tombstoned(node_b)); // op(2,10) > state[2]=8
    }

    #[test]
    fn gc_text_tombstones() {
        let mut tracker = TombstoneTracker::new();
        let node = NodeId::new(0, 5);
        tracker.add_text_tombstone(node, OpId::new(1, 1), OpId::new(1, 3));
        tracker.add_text_tombstone(node, OpId::new(1, 2), OpId::new(2, 5));

        let mut min_state = StateVector::new();
        min_state.set(1, 10);
        min_state.set(2, 10);

        let removed = tracker.gc(&min_state);
        assert_eq!(removed, 2);
        assert_eq!(tracker.text_tombstone_count(), 0);
    }
}
