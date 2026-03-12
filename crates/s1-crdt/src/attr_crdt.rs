//! Per-key Last-Writer-Wins attribute registers.
//!
//! Each (node, attribute_key) pair is an independent LWW register. Concurrent
//! `SetAttributes` on different keys both apply. Same key: the highest [`OpId`] wins.

use std::collections::HashMap;

use crate::op_id::OpId;
use s1_model::{AttributeKey, AttributeMap, AttributeValue, NodeId};

/// Entry in the LWW register: a value and the OpId that set it.
#[derive(Debug, Clone)]
struct AttrEntry {
    op_id: OpId,
    value: AttributeValue,
}

/// LWW attribute registers for all nodes.
#[derive(Debug, Clone)]
pub struct AttrCrdt {
    /// (node_id, key) -> (op_id, value)
    entries: HashMap<(NodeId, AttributeKey), AttrEntry>,
    /// Tracks removed attributes: (node_id, key) -> op_id of removal.
    removed: HashMap<(NodeId, AttributeKey), OpId>,
}

impl AttrCrdt {
    /// Create a new empty attribute CRDT.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            removed: HashMap::new(),
        }
    }

    /// Integrate a `SetAttributes` operation.
    ///
    /// For each key in the attribute map, apply LWW against existing entries.
    /// Returns the keys that were actually updated (for producing the effective op).
    pub fn integrate_set(
        &mut self,
        node_id: NodeId,
        attributes: &AttributeMap,
        op_id: OpId,
    ) -> AttributeMap {
        let mut effective = AttributeMap::new();

        for (key, value) in attributes.iter() {
            let map_key = (node_id, key.clone());

            // Check if this key was removed by a later operation
            if let Some(&remove_op) = self.removed.get(&map_key) {
                if remove_op > op_id {
                    continue; // Removal wins
                }
                // Our set is newer, remove the removal record
                self.removed.remove(&map_key);
            }

            let should_apply = match self.entries.get(&map_key) {
                Some(existing) => op_id > existing.op_id,
                None => true,
            };

            if should_apply {
                self.entries.insert(
                    map_key,
                    AttrEntry {
                        op_id,
                        value: value.clone(),
                    },
                );
                effective.set(key.clone(), value.clone());
            }
        }

        effective
    }

    /// Integrate a `RemoveAttributes` operation.
    ///
    /// For each key, check LWW against existing set operations.
    /// Returns the keys that were actually removed.
    pub fn integrate_remove(
        &mut self,
        node_id: NodeId,
        keys: &[AttributeKey],
        op_id: OpId,
    ) -> Vec<AttributeKey> {
        let mut effective = Vec::new();

        for key in keys {
            let map_key = (node_id, key.clone());

            let should_remove = match self.entries.get(&map_key) {
                Some(existing) => op_id > existing.op_id,
                None => false, // Nothing to remove
            };

            if should_remove {
                self.entries.remove(&map_key);
                self.removed.insert(map_key, op_id);
                effective.push(key.clone());
            }
        }

        effective
    }

    /// Get the current value of an attribute on a node.
    pub fn get(&self, node_id: NodeId, key: &AttributeKey) -> Option<&AttributeValue> {
        self.entries
            .get(&(node_id, key.clone()))
            .map(|entry| &entry.value)
    }

    /// Get all current attributes for a node.
    pub fn node_attributes(&self, node_id: NodeId) -> AttributeMap {
        let mut result = AttributeMap::new();
        for ((nid, key), entry) in &self.entries {
            if *nid == node_id {
                result.set(key.clone(), entry.value.clone());
            }
        }
        result
    }

    /// Register existing attributes from the model (during init).
    pub fn register_attributes(&mut self, node_id: NodeId, attributes: &AttributeMap, op_id: OpId) {
        for (key, value) in attributes.iter() {
            self.entries.insert(
                (node_id, key.clone()),
                AttrEntry {
                    op_id,
                    value: value.clone(),
                },
            );
        }
    }

    /// Remove all entries for a node (when the node is deleted).
    pub fn remove_node(&mut self, node_id: NodeId) {
        self.entries.retain(|(nid, _), _| *nid != node_id);
        self.removed.retain(|(nid, _), _| *nid != node_id);
    }
}

impl Default for AttrCrdt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(c: u64) -> NodeId {
        NodeId::new(0, c)
    }

    #[test]
    fn set_single_attribute() {
        let mut crdt = AttrCrdt::new();
        let attrs = AttributeMap::new().bold(true);
        let effective = crdt.integrate_set(nid(1), &attrs, OpId::new(1, 1));

        assert!(!effective.is_empty());
        assert_eq!(
            crdt.get(nid(1), &AttributeKey::Bold),
            Some(&AttributeValue::Bool(true))
        );
    }

    #[test]
    fn lww_newer_wins() {
        let mut crdt = AttrCrdt::new();

        // Set font size to 12
        let attrs1 = AttributeMap::new().font_size(12.0);
        crdt.integrate_set(nid(1), &attrs1, OpId::new(1, 1));

        // Set font size to 24 (newer)
        let attrs2 = AttributeMap::new().font_size(24.0);
        crdt.integrate_set(nid(1), &attrs2, OpId::new(2, 2));

        assert_eq!(
            crdt.get(nid(1), &AttributeKey::FontSize),
            Some(&AttributeValue::Float(24.0))
        );
    }

    #[test]
    fn lww_older_ignored() {
        let mut crdt = AttrCrdt::new();

        // Set font size to 24 (newer first)
        let attrs2 = AttributeMap::new().font_size(24.0);
        crdt.integrate_set(nid(1), &attrs2, OpId::new(2, 2));

        // Set font size to 12 (older, should be ignored)
        let attrs1 = AttributeMap::new().font_size(12.0);
        let effective = crdt.integrate_set(nid(1), &attrs1, OpId::new(1, 1));

        assert!(effective.is_empty()); // Nothing applied
        assert_eq!(
            crdt.get(nid(1), &AttributeKey::FontSize),
            Some(&AttributeValue::Float(24.0))
        );
    }

    #[test]
    fn concurrent_different_keys_both_apply() {
        let mut crdt = AttrCrdt::new();

        let bold = AttributeMap::new().bold(true);
        crdt.integrate_set(nid(1), &bold, OpId::new(1, 1));

        let italic = AttributeMap::new().italic(true);
        crdt.integrate_set(nid(1), &italic, OpId::new(2, 1));

        // Both should be present
        assert_eq!(
            crdt.get(nid(1), &AttributeKey::Bold),
            Some(&AttributeValue::Bool(true))
        );
        assert_eq!(
            crdt.get(nid(1), &AttributeKey::Italic),
            Some(&AttributeValue::Bool(true))
        );
    }

    #[test]
    fn remove_attribute() {
        let mut crdt = AttrCrdt::new();

        let attrs = AttributeMap::new().bold(true);
        crdt.integrate_set(nid(1), &attrs, OpId::new(1, 1));

        let removed = crdt.integrate_remove(nid(1), &[AttributeKey::Bold], OpId::new(1, 2));
        assert_eq!(removed, vec![AttributeKey::Bold]);
        assert!(crdt.get(nid(1), &AttributeKey::Bold).is_none());
    }

    #[test]
    fn remove_older_than_set_ignored() {
        let mut crdt = AttrCrdt::new();

        // Set bold at lamport 5
        let attrs = AttributeMap::new().bold(true);
        crdt.integrate_set(nid(1), &attrs, OpId::new(1, 5));

        // Try to remove at lamport 3 (older)
        let removed = crdt.integrate_remove(nid(1), &[AttributeKey::Bold], OpId::new(1, 3));
        assert!(removed.is_empty());
        assert!(crdt.get(nid(1), &AttributeKey::Bold).is_some());
    }

    #[test]
    fn set_after_remove_wins() {
        let mut crdt = AttrCrdt::new();

        // Set bold
        let attrs = AttributeMap::new().bold(true);
        crdt.integrate_set(nid(1), &attrs, OpId::new(1, 1));

        // Remove bold
        crdt.integrate_remove(nid(1), &[AttributeKey::Bold], OpId::new(1, 2));
        assert!(crdt.get(nid(1), &AttributeKey::Bold).is_none());

        // Set bold again (newer)
        let attrs2 = AttributeMap::new().bold(true);
        crdt.integrate_set(nid(1), &attrs2, OpId::new(1, 3));
        assert!(crdt.get(nid(1), &AttributeKey::Bold).is_some());
    }

    #[test]
    fn node_attributes() {
        let mut crdt = AttrCrdt::new();

        let attrs = AttributeMap::new().bold(true).font_size(12.0);
        crdt.integrate_set(nid(1), &attrs, OpId::new(1, 1));

        let node_attrs = crdt.node_attributes(nid(1));
        assert_eq!(node_attrs.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(node_attrs.get_f64(&AttributeKey::FontSize), Some(12.0));
    }

    #[test]
    fn register_and_overwrite() {
        let mut crdt = AttrCrdt::new();
        let attrs = AttributeMap::new().bold(true);
        crdt.register_attributes(nid(1), &attrs, OpId::new(0, 1));

        assert_eq!(
            crdt.get(nid(1), &AttributeKey::Bold),
            Some(&AttributeValue::Bool(true))
        );

        // Overwrite with newer
        let attrs2 = AttributeMap::new().bold(false);
        crdt.integrate_set(nid(1), &attrs2, OpId::new(1, 2));
        assert_eq!(
            crdt.get(nid(1), &AttributeKey::Bold),
            Some(&AttributeValue::Bool(false))
        );
    }

    #[test]
    fn remove_node_cleans_up() {
        let mut crdt = AttrCrdt::new();
        let attrs = AttributeMap::new().bold(true);
        crdt.integrate_set(nid(1), &attrs, OpId::new(1, 1));

        crdt.remove_node(nid(1));
        assert!(crdt.get(nid(1), &AttributeKey::Bold).is_none());
        assert!(crdt.node_attributes(nid(1)).is_empty());
    }
}
