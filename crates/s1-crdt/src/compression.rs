//! Operation log compression.
//!
//! Merges consecutive single-character inserts from the same replica into
//! multi-character operations, dramatically reducing op-log size for typical
//! sequential typing patterns.

use crate::crdt_op::CrdtOperation;
use crate::op_id::OpId;
use s1_ops::Operation;

/// Compress a sequence of CRDT operations by merging consecutive character inserts.
///
/// Two operations can be merged if:
/// 1. They are both `InsertText` on the same target node
/// 2. They are from the same replica
/// 3. Their Lamport timestamps are consecutive
/// 4. The second insert's offset equals the first's offset + first's text length
/// 5. The second insert's origin_left is the last character of the first insert
///
/// Returns a new vec with merged operations. Non-text operations pass through unchanged.
pub fn compress_ops(ops: &[CrdtOperation]) -> Vec<CrdtOperation> {
    if ops.is_empty() {
        return Vec::new();
    }

    let mut result: Vec<CrdtOperation> = Vec::new();

    for op in ops {
        let merged = if let Some(last) = result.last_mut() {
            try_merge(last, op)
        } else {
            false
        };

        if !merged {
            result.push(op.clone());
        }
    }

    result
}

/// Try to merge `next` into `current`. Returns `true` if merged.
fn try_merge(current: &mut CrdtOperation, next: &CrdtOperation) -> bool {
    // Must be same replica
    if current.id.replica != next.id.replica {
        return false;
    }

    // Both must be InsertText on the same node
    let (cur_target, cur_offset, cur_text) = match &current.operation {
        Operation::InsertText {
            target_id,
            offset,
            text,
        } => (*target_id, *offset, text.clone()),
        _ => return false,
    };

    let (next_target, next_offset, next_text) = match &next.operation {
        Operation::InsertText {
            target_id,
            offset,
            text,
        } => (*target_id, *offset, text.clone()),
        _ => return false,
    };

    // Same target node
    if cur_target != next_target {
        return false;
    }

    // Consecutive Lamport timestamps (accounting for multi-char operations)
    let cur_char_count = cur_text.chars().count() as u64;
    if next.id.lamport != current.id.lamport + cur_char_count {
        return false;
    }

    // Adjacent offsets (next insert is right after current)
    if next_offset != cur_offset + cur_text.len() {
        return false;
    }

    // Origin continuity: next's origin_left should be the last char of current
    let expected_origin = Some(OpId::new(
        current.id.replica,
        current.id.lamport + cur_char_count - 1,
    ));
    if next.origin_left != expected_origin {
        return false;
    }

    // Merge: extend current's text and deps
    current.operation = Operation::insert_text(cur_target, cur_offset, cur_text + &next_text);
    current.origin_right = next.origin_right;
    current.deps.merge(&next.deps);

    true
}

/// Calculate the compression ratio: `compressed_count / original_count`.
///
/// Returns 1.0 for no compression, lower values indicate better compression.
pub fn compression_ratio(original: usize, compressed: usize) -> f64 {
    if original == 0 {
        return 1.0;
    }
    compressed as f64 / original as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_vector::StateVector;
    use s1_model::NodeId;

    fn make_char_insert(
        replica: u64,
        lamport: u64,
        target: NodeId,
        offset: usize,
        ch: char,
        origin_left: Option<OpId>,
        origin_right: Option<OpId>,
    ) -> CrdtOperation {
        CrdtOperation::new(
            OpId::new(replica, lamport),
            StateVector::new(),
            Operation::insert_text(target, offset, ch.to_string()),
        )
        .with_text_origins(origin_left, origin_right)
    }

    #[test]
    fn compress_empty() {
        assert!(compress_ops(&[]).is_empty());
    }

    #[test]
    fn compress_single_op() {
        let target = NodeId::new(0, 5);
        let ops = vec![make_char_insert(1, 1, target, 0, 'a', None, None)];
        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 1);
    }

    #[test]
    fn compress_consecutive_chars() {
        let target = NodeId::new(0, 5);
        let ops = vec![
            make_char_insert(1, 1, target, 0, 'h', None, None),
            make_char_insert(1, 2, target, 1, 'e', Some(OpId::new(1, 1)), None),
            make_char_insert(1, 3, target, 2, 'l', Some(OpId::new(1, 2)), None),
            make_char_insert(1, 4, target, 3, 'l', Some(OpId::new(1, 3)), None),
            make_char_insert(1, 5, target, 4, 'o', Some(OpId::new(1, 4)), None),
        ];

        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 1);

        if let Operation::InsertText { text, offset, .. } = &compressed[0].operation {
            assert_eq!(text, "hello");
            assert_eq!(*offset, 0);
        } else {
            panic!("Expected InsertText");
        }
    }

    #[test]
    fn no_compress_different_replicas() {
        let target = NodeId::new(0, 5);
        let ops = vec![
            make_char_insert(1, 1, target, 0, 'a', None, None),
            make_char_insert(2, 2, target, 1, 'b', Some(OpId::new(1, 1)), None),
        ];

        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 2);
    }

    #[test]
    fn no_compress_non_consecutive_lamport() {
        let target = NodeId::new(0, 5);
        let ops = vec![
            make_char_insert(1, 1, target, 0, 'a', None, None),
            make_char_insert(1, 5, target, 1, 'b', Some(OpId::new(1, 1)), None), // gap
        ];

        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 2);
    }

    #[test]
    fn no_compress_different_targets() {
        let ops = vec![
            make_char_insert(1, 1, NodeId::new(0, 5), 0, 'a', None, None),
            make_char_insert(1, 2, NodeId::new(0, 6), 0, 'b', Some(OpId::new(1, 1)), None),
        ];

        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 2);
    }

    #[test]
    fn compress_partial_runs() {
        let target = NodeId::new(0, 5);
        let ops = vec![
            // Run 1: "ab"
            make_char_insert(1, 1, target, 0, 'a', None, None),
            make_char_insert(1, 2, target, 1, 'b', Some(OpId::new(1, 1)), None),
            // Different operation breaks the run
            CrdtOperation::new(
                OpId::new(1, 3),
                StateVector::new(),
                Operation::set_metadata("title", Some("Doc".into())),
            ),
            // Run 2: "cd"
            make_char_insert(1, 4, target, 2, 'c', Some(OpId::new(1, 2)), None),
            make_char_insert(1, 5, target, 3, 'd', Some(OpId::new(1, 4)), None),
        ];

        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 3); // "ab", metadata, "cd"
    }

    #[test]
    fn compression_ratio_calculation() {
        assert_eq!(compression_ratio(0, 0), 1.0);
        assert_eq!(compression_ratio(10, 10), 1.0);
        assert_eq!(compression_ratio(10, 5), 0.5);
        assert_eq!(compression_ratio(100, 1), 0.01);
    }

    #[test]
    fn no_compress_non_adjacent_offsets() {
        let target = NodeId::new(0, 5);
        let ops = vec![
            make_char_insert(1, 1, target, 0, 'a', None, None),
            make_char_insert(1, 2, target, 5, 'b', Some(OpId::new(1, 1)), None), // non-adjacent
        ];

        let compressed = compress_ops(&ops);
        assert_eq!(compressed.len(), 2);
    }
}
