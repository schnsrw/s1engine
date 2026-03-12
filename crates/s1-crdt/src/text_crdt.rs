//! Fugue-based text CRDT for concurrent character-level editing.
//!
//! Each character in a text node is tracked individually with its [`OpId`] and
//! origin references (left/right neighbors at insertion time). The integration
//! algorithm deterministically resolves concurrent inserts at the same position.
//!
//! Deletions are handled via tombstones — characters are marked as deleted but
//! retained for conflict resolution of concurrent operations.

use std::collections::HashMap;

use crate::op_id::OpId;
use s1_model::NodeId;

/// A single character tracked by the text CRDT.
#[derive(Debug, Clone, PartialEq)]
struct TextItem {
    /// Unique ID for this character insertion.
    id: OpId,
    /// The character to the left at insertion time.
    origin_left: Option<OpId>,
    /// The character to the right at insertion time.
    origin_right: Option<OpId>,
    /// The character content.
    ch: char,
    /// Whether this character has been deleted (tombstone).
    deleted: bool,
}

/// An ordered sequence of characters (including tombstones) for a single text node.
#[derive(Debug, Clone)]
struct TextSequence {
    items: Vec<TextItem>,
}

impl TextSequence {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Find the raw index of an item by its OpId.
    fn find_index(&self, id: OpId) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    /// Integrate a new character insert using Fugue-style ordering.
    ///
    /// Returns the visible (non-tombstone) offset of the inserted character.
    fn integrate_insert(&mut self, item: TextItem) -> usize {
        if self.items.is_empty() {
            self.items.push(item);
            return 0;
        }

        // Find the raw position after origin_left
        let scan_start = match item.origin_left {
            None => 0,
            Some(left_id) => self.find_index(left_id).map(|p| p + 1).unwrap_or(0),
        };

        // Find the raw position of origin_right (exclusive bound)
        let scan_end = match item.origin_right {
            None => self.items.len(),
            Some(right_id) => self.find_index(right_id).unwrap_or(self.items.len()),
        };

        // Position of the new item's origin_left in the current sequence.
        // None (no origin) is conceptually before everything, so use -1.
        let new_origin_pos: i64 = match item.origin_left {
            None => -1,
            Some(id) => self.find_index(id).map(|i| i as i64).unwrap_or(-1),
        };

        // Scan between origins to find correct insertion point.
        // Among items sharing the same origin_left, sort by OpId (lower first).
        // For items with different origin_left, compare origin positions (YATA rule):
        //   - If existing's origin is strictly LEFT of new's origin → skip past it
        //   - Otherwise → stop (new item goes before it)
        let mut insert_pos = scan_start;
        for i in scan_start..scan_end {
            let existing = &self.items[i];
            if existing.origin_left == item.origin_left {
                if existing.id < item.id {
                    insert_pos = i + 1;
                } else {
                    break;
                }
            } else {
                // Different origin: compare positions to ensure deterministic ordering.
                let existing_origin_pos: i64 = match existing.origin_left {
                    None => -1,
                    Some(id) => self.find_index(id).map(|p| p as i64).unwrap_or(-1),
                };
                if existing_origin_pos < new_origin_pos {
                    // Existing's origin is further left → skip past it
                    insert_pos = i + 1;
                } else {
                    // Existing's origin is at or right of ours → stop
                    break;
                }
            }
        }

        self.items.insert(insert_pos, item);

        // Return visible offset
        self.items[..=insert_pos]
            .iter()
            .filter(|i| !i.deleted)
            .count()
            - 1
    }

    /// Mark a character as deleted. Returns the visible offset before deletion.
    fn integrate_delete(&mut self, char_id: OpId) -> Option<usize> {
        let raw_idx = self.find_index(char_id)?;

        if self.items[raw_idx].deleted {
            return None; // Already deleted
        }

        // Calculate visible offset before marking deleted
        let visible_offset = self.items[..=raw_idx].iter().filter(|i| !i.deleted).count() - 1;

        self.items[raw_idx].deleted = true;

        Some(visible_offset)
    }

    /// Get the visible (non-tombstone) text.
    fn visible_text(&self) -> String {
        self.items
            .iter()
            .filter(|i| !i.deleted)
            .map(|i| i.ch)
            .collect()
    }

    /// Convert a visible offset to the OpId at that position.
    fn offset_to_op_id(&self, offset: usize) -> Option<OpId> {
        let mut visible = 0;
        for item in &self.items {
            if !item.deleted {
                if visible == offset {
                    return Some(item.id);
                }
                visible += 1;
            }
        }
        None
    }

    /// Get the left and right neighbor OpIds at a visible offset.
    ///
    /// Used when creating a CrdtOperation for a local insert at `offset`.
    fn neighbors_at_offset(&self, offset: usize) -> (Option<OpId>, Option<OpId>) {
        let mut visible = 0;
        let mut left = None;

        for item in &self.items {
            if !item.deleted {
                if visible == offset {
                    return (left, Some(item.id));
                }
                left = Some(item.id);
                visible += 1;
            }
        }

        // Offset is at the end
        (left, None)
    }

    /// Total number of items (including tombstones).
    fn total_len(&self) -> usize {
        self.items.len()
    }

    /// Number of visible (non-tombstone) items.
    fn visible_len(&self) -> usize {
        self.items.iter().filter(|i| !i.deleted).count()
    }
}

/// Text CRDT managing character sequences for all text nodes.
#[derive(Debug, Clone)]
pub struct TextCrdt {
    sequences: HashMap<NodeId, TextSequence>,
}

impl TextCrdt {
    /// Create a new empty text CRDT.
    pub fn new() -> Self {
        Self {
            sequences: HashMap::new(),
        }
    }

    /// Initialize tracking for a text node with existing content.
    ///
    /// Each character gets a synthetic OpId from the given replica, starting
    /// at `start_lamport`. Returns the lamport value after the last character.
    pub fn init_text(
        &mut self,
        node_id: NodeId,
        replica: u64,
        start_lamport: u64,
        content: &str,
    ) -> u64 {
        let seq = self
            .sequences
            .entry(node_id)
            .or_insert_with(TextSequence::new);
        let mut lamport = start_lamport;
        let mut prev_id = None;

        for ch in content.chars() {
            let id = OpId::new(replica, lamport);
            seq.items.push(TextItem {
                id,
                origin_left: prev_id,
                origin_right: None, // Will be set for proper interleaving, but for init it's fine
                ch,
                deleted: false,
            });
            prev_id = Some(id);
            lamport += 1;
        }

        lamport
    }

    /// Integrate a character insert into the text CRDT.
    ///
    /// Returns the visible offset where the character was placed.
    pub fn integrate_insert(
        &mut self,
        node_id: NodeId,
        id: OpId,
        origin_left: Option<OpId>,
        origin_right: Option<OpId>,
        ch: char,
    ) -> usize {
        let seq = self
            .sequences
            .entry(node_id)
            .or_insert_with(TextSequence::new);
        let item = TextItem {
            id,
            origin_left,
            origin_right,
            ch,
            deleted: false,
        };
        seq.integrate_insert(item)
    }

    /// Integrate a character delete into the text CRDT.
    ///
    /// Returns the visible offset of the deleted character (before deletion),
    /// or `None` if already deleted.
    pub fn integrate_delete(&mut self, node_id: NodeId, char_id: OpId) -> Option<usize> {
        self.sequences
            .get_mut(&node_id)
            .and_then(|seq| seq.integrate_delete(char_id))
    }

    /// Get the visible text for a node.
    pub fn visible_text(&self, node_id: NodeId) -> String {
        self.sequences
            .get(&node_id)
            .map(|seq| seq.visible_text())
            .unwrap_or_default()
    }

    /// Convert a visible offset to the OpId at that position.
    pub fn offset_to_op_id(&self, node_id: NodeId, offset: usize) -> Option<OpId> {
        self.sequences
            .get(&node_id)
            .and_then(|seq| seq.offset_to_op_id(offset))
    }

    /// Get the left and right neighbor OpIds at a visible offset.
    pub fn neighbors_at_offset(
        &self,
        node_id: NodeId,
        offset: usize,
    ) -> (Option<OpId>, Option<OpId>) {
        self.sequences
            .get(&node_id)
            .map(|seq| seq.neighbors_at_offset(offset))
            .unwrap_or((None, None))
    }

    /// Check if a text node is being tracked.
    pub fn has_node(&self, node_id: NodeId) -> bool {
        self.sequences.contains_key(&node_id)
    }

    /// Remove tracking for a text node (when the node is deleted).
    pub fn remove_node(&mut self, node_id: NodeId) {
        self.sequences.remove(&node_id);
    }

    /// Get the visible length of a text node.
    pub fn visible_len(&self, node_id: NodeId) -> usize {
        self.sequences
            .get(&node_id)
            .map(|seq| seq.visible_len())
            .unwrap_or(0)
    }

    /// Get the total length including tombstones.
    pub fn total_len(&self, node_id: NodeId) -> usize {
        self.sequences
            .get(&node_id)
            .map(|seq| seq.total_len())
            .unwrap_or(0)
    }
}

impl Default for TextCrdt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(counter: u64) -> NodeId {
        NodeId::new(0, counter)
    }

    #[test]
    fn insert_single_char() {
        let mut crdt = TextCrdt::new();
        let n = node(1);
        let offset = crdt.integrate_insert(n, OpId::new(1, 1), None, None, 'a');
        assert_eq!(offset, 0);
        assert_eq!(crdt.visible_text(n), "a");
    }

    #[test]
    fn insert_sequential_chars() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        crdt.integrate_insert(n, id_a, None, None, 'a');

        let id_b = OpId::new(1, 2);
        crdt.integrate_insert(n, id_b, Some(id_a), None, 'b');

        let id_c = OpId::new(1, 3);
        crdt.integrate_insert(n, id_c, Some(id_b), None, 'c');

        assert_eq!(crdt.visible_text(n), "abc");
    }

    #[test]
    fn insert_at_beginning() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_b = OpId::new(1, 1);
        crdt.integrate_insert(n, id_b, None, None, 'b');

        // Insert 'a' before 'b'
        let id_a = OpId::new(1, 2);
        crdt.integrate_insert(n, id_a, None, Some(id_b), 'a');

        assert_eq!(crdt.visible_text(n), "ab");
    }

    #[test]
    fn insert_in_middle() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        crdt.integrate_insert(n, id_a, None, None, 'a');

        let id_c = OpId::new(1, 2);
        crdt.integrate_insert(n, id_c, Some(id_a), None, 'c');

        // Insert 'b' between 'a' and 'c'
        let id_b = OpId::new(1, 3);
        crdt.integrate_insert(n, id_b, Some(id_a), Some(id_c), 'b');

        assert_eq!(crdt.visible_text(n), "abc");
    }

    #[test]
    fn concurrent_insert_same_position() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        // Both users insert after nothing (at position 0)
        // User 1 inserts 'x' with lamport=1
        // User 2 inserts 'y' with lamport=2
        // OpId ordering: (1,1) < (2,2), so 'x' goes first
        crdt.integrate_insert(n, OpId::new(1, 1), None, None, 'x');
        crdt.integrate_insert(n, OpId::new(2, 2), None, None, 'y');

        assert_eq!(crdt.visible_text(n), "xy");
    }

    #[test]
    fn concurrent_insert_same_position_reverse_order() {
        // Same scenario but applied in reverse order — must produce same result
        let mut crdt = TextCrdt::new();
        let n = node(1);

        crdt.integrate_insert(n, OpId::new(2, 2), None, None, 'y');
        crdt.integrate_insert(n, OpId::new(1, 1), None, None, 'x');

        assert_eq!(crdt.visible_text(n), "xy");
    }

    #[test]
    fn concurrent_insert_between_same_chars() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_c = OpId::new(1, 2);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_c, Some(id_a), None, 'c');

        // Two users concurrently insert between 'a' and 'c'
        // User 1 inserts 'x' (lamport=3)
        // User 2 inserts 'y' (lamport=4)
        crdt.integrate_insert(n, OpId::new(1, 3), Some(id_a), Some(id_c), 'x');
        crdt.integrate_insert(n, OpId::new(2, 4), Some(id_a), Some(id_c), 'y');

        assert_eq!(crdt.visible_text(n), "axyc");
    }

    #[test]
    fn concurrent_insert_between_same_chars_reverse() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_c = OpId::new(1, 2);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_c, Some(id_a), None, 'c');

        // Applied in reverse order — must produce same result
        crdt.integrate_insert(n, OpId::new(2, 4), Some(id_a), Some(id_c), 'y');
        crdt.integrate_insert(n, OpId::new(1, 3), Some(id_a), Some(id_c), 'x');

        assert_eq!(crdt.visible_text(n), "axyc");
    }

    #[test]
    fn delete_char() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_b = OpId::new(1, 2);
        let id_c = OpId::new(1, 3);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_b, Some(id_a), None, 'b');
        crdt.integrate_insert(n, id_c, Some(id_b), None, 'c');
        assert_eq!(crdt.visible_text(n), "abc");

        let offset = crdt.integrate_delete(n, id_b);
        assert_eq!(offset, Some(1));
        assert_eq!(crdt.visible_text(n), "ac");
    }

    #[test]
    fn delete_already_deleted() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id = OpId::new(1, 1);
        crdt.integrate_insert(n, id, None, None, 'a');

        assert!(crdt.integrate_delete(n, id).is_some());
        assert!(crdt.integrate_delete(n, id).is_none()); // idempotent
    }

    #[test]
    fn offset_to_op_id_conversion() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_b = OpId::new(1, 2);
        let id_c = OpId::new(1, 3);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_b, Some(id_a), None, 'b');
        crdt.integrate_insert(n, id_c, Some(id_b), None, 'c');

        assert_eq!(crdt.offset_to_op_id(n, 0), Some(id_a));
        assert_eq!(crdt.offset_to_op_id(n, 1), Some(id_b));
        assert_eq!(crdt.offset_to_op_id(n, 2), Some(id_c));
        assert_eq!(crdt.offset_to_op_id(n, 3), None);
    }

    #[test]
    fn offset_skips_tombstones() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_b = OpId::new(1, 2);
        let id_c = OpId::new(1, 3);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_b, Some(id_a), None, 'b');
        crdt.integrate_insert(n, id_c, Some(id_b), None, 'c');

        crdt.integrate_delete(n, id_b); // delete 'b'

        // Now visible text is "ac"
        assert_eq!(crdt.offset_to_op_id(n, 0), Some(id_a));
        assert_eq!(crdt.offset_to_op_id(n, 1), Some(id_c));
    }

    #[test]
    fn neighbors_at_offset() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_b = OpId::new(1, 2);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_b, Some(id_a), None, 'b');

        // At offset 0: left=None, right=a
        assert_eq!(crdt.neighbors_at_offset(n, 0), (None, Some(id_a)));
        // At offset 1: left=a, right=b
        assert_eq!(crdt.neighbors_at_offset(n, 1), (Some(id_a), Some(id_b)));
        // At offset 2 (end): left=b, right=None
        assert_eq!(crdt.neighbors_at_offset(n, 2), (Some(id_b), None));
    }

    #[test]
    fn init_text_node() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let end_lamport = crdt.init_text(n, 0, 1, "hello");
        assert_eq!(end_lamport, 6);
        assert_eq!(crdt.visible_text(n), "hello");
        assert_eq!(crdt.visible_len(n), 5);
        assert_eq!(crdt.total_len(n), 5);
    }

    #[test]
    fn visible_and_total_len() {
        let mut crdt = TextCrdt::new();
        let n = node(1);

        let id_a = OpId::new(1, 1);
        let id_b = OpId::new(1, 2);
        crdt.integrate_insert(n, id_a, None, None, 'a');
        crdt.integrate_insert(n, id_b, Some(id_a), None, 'b');

        assert_eq!(crdt.visible_len(n), 2);
        assert_eq!(crdt.total_len(n), 2);

        crdt.integrate_delete(n, id_a);
        assert_eq!(crdt.visible_len(n), 1);
        assert_eq!(crdt.total_len(n), 2); // tombstone still there
    }

    #[test]
    fn has_and_remove_node() {
        let mut crdt = TextCrdt::new();
        let n = node(1);
        assert!(!crdt.has_node(n));

        crdt.integrate_insert(n, OpId::new(1, 1), None, None, 'a');
        assert!(crdt.has_node(n));

        crdt.remove_node(n);
        assert!(!crdt.has_node(n));
    }

    #[test]
    fn empty_node_text() {
        let crdt = TextCrdt::new();
        let n = node(1);
        assert_eq!(crdt.visible_text(n), "");
        assert_eq!(crdt.visible_len(n), 0);
    }
}
