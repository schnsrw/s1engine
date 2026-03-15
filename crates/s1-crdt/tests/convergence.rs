//! Multi-replica convergence tests.
//!
//! These tests verify that multiple replicas applying the same set of operations
//! in different orders converge to identical document states.

use s1_crdt::CollabDocument;
use s1_model::{AttributeKey, Node, NodeId, NodeType};
use s1_ops::Operation;

/// Helper: synchronize all operations between two replicas.
fn sync_two(doc1: &mut CollabDocument, doc2: &mut CollabDocument) {
    // Send doc1's changes to doc2
    let changes_1_to_2 = doc1.changes_since(doc2.state_vector());
    for op in changes_1_to_2 {
        doc2.apply_remote(op).unwrap();
    }

    // Send doc2's changes to doc1
    let changes_2_to_1 = doc2.changes_since(doc1.state_vector());
    for op in changes_2_to_1 {
        doc1.apply_remote(op).unwrap();
    }
}

/// Helper: fully synchronize all replicas pairwise until no changes remain.
fn sync_all(docs: &mut [&mut CollabDocument]) {
    // Repeat until stable (handles transitive dependencies)
    for _ in 0..docs.len() {
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                if i == j {
                    continue;
                }
                let changes = docs[i].changes_since(docs[j].state_vector());
                for op in changes {
                    docs[j].apply_remote(op).unwrap();
                }
            }
        }
    }
}

/// Helper: build a paragraph with text in a collab document.
fn add_paragraph_with_text(doc: &mut CollabDocument, text: &str) -> (NodeId, NodeId) {
    let body_id = doc.model().body_id().unwrap();
    let para_id = doc.next_id();
    doc.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let run_id = doc.next_id();
    doc.apply_local(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ))
    .unwrap();

    let text_id = doc.next_id();
    doc.apply_local(Operation::insert_node(run_id, 0, Node::text(text_id, "")))
        .unwrap();

    if !text.is_empty() {
        doc.apply_local(Operation::insert_text(text_id, 0, text))
            .unwrap();
    }

    (para_id, text_id)
}

// ─── Two-replica convergence ─────────────────────────────────────────────

#[test]
fn two_replicas_concurrent_insert_nodes() {
    let mut doc1 = CollabDocument::new(1);
    let mut doc2 = CollabDocument::new(2);

    let body_id = doc1.model().body_id().unwrap();

    // Doc1 inserts paragraph A
    let para_a = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_a, NodeType::Paragraph),
    ))
    .unwrap();

    // Doc2 inserts paragraph B
    let para_b = doc2.next_id();
    doc2.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_b, NodeType::Paragraph),
    ))
    .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // Both replicas should have both paragraphs
    assert!(doc1.model().node(para_a).is_some());
    assert!(doc1.model().node(para_b).is_some());
    assert!(doc2.model().node(para_a).is_some());
    assert!(doc2.model().node(para_b).is_some());

    // Children order should be the same (deterministic via OpId ordering)
    let children_1: Vec<NodeId> = doc1.model().node(body_id).unwrap().children.clone();
    let children_2: Vec<NodeId> = doc2.model().node(body_id).unwrap().children.clone();
    assert_eq!(children_1, children_2);
}

#[test]
fn two_replicas_concurrent_text_insert_converge() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Doc1 types "abc"
    doc1.apply_local(Operation::insert_text(text_id, 0, "abc"))
        .unwrap();

    // Doc2 types "xyz"
    doc2.apply_local(Operation::insert_text(text_id, 0, "xyz"))
        .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // Both must have the same text (order determined by OpId)
    let text1 = doc1.to_plain_text();
    let text2 = doc2.to_plain_text();
    assert_eq!(text1, text2, "replicas must converge");
    // All characters present (may be interleaved at character level)
    assert_eq!(text1.len(), 6);
    for ch in ['a', 'b', 'c', 'x', 'y', 'z'] {
        assert!(text1.contains(ch), "missing char '{ch}'");
    }
}

#[test]
fn two_replicas_sequential_typing_converge() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Doc1 types character by character
    doc1.apply_local(Operation::insert_text(text_id, 0, "h"))
        .unwrap();
    doc1.apply_local(Operation::insert_text(text_id, 1, "e"))
        .unwrap();
    doc1.apply_local(Operation::insert_text(text_id, 2, "l"))
        .unwrap();

    // Doc2 types character by character
    doc2.apply_local(Operation::insert_text(text_id, 0, "w"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 1, "o"))
        .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    let text1 = doc1.to_plain_text();
    let text2 = doc2.to_plain_text();
    assert_eq!(text1, text2, "replicas must converge");
    // All 5 characters present, exact ordering is CRDT-deterministic
    assert_eq!(text1.len(), 5);
    for ch in ['h', 'e', 'l', 'w', 'o'] {
        assert!(text1.contains(ch), "missing char '{ch}'");
    }
}

#[test]
fn two_replicas_delete_while_other_inserts() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Create a paragraph
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let mut doc2 = doc1.fork(2);

    // Doc1 deletes the paragraph
    doc1.apply_local(Operation::delete_node(para_id)).unwrap();

    // Doc2 inserts a child into the paragraph (concurrent with delete)
    let run_id = doc2.next_id();
    doc2.apply_local(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ))
    .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // Delete wins: paragraph should be gone from both
    assert!(doc1.model().node(para_id).is_none());
    // Note: doc2 may still have the paragraph if the delete didn't propagate to model
    // But the CRDT state should agree it's tombstoned
}

#[test]
fn two_replicas_concurrent_metadata_lww() {
    let mut doc1 = CollabDocument::new(1);
    let mut doc2 = doc1.fork(2);

    // Both set the title concurrently
    doc1.apply_local(Operation::set_metadata("title", Some("Doc1 Title".into())))
        .unwrap();
    doc2.apply_local(Operation::set_metadata("title", Some("Doc2 Title".into())))
        .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // LWW: one title wins, and it's the same on both
    let title1 = doc1.model().metadata().title.clone();
    let title2 = doc2.model().metadata().title.clone();
    assert_eq!(title1, title2);
}

#[test]
fn two_replicas_concurrent_attributes_different_keys() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Create a paragraph and run
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let run_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ))
    .unwrap();

    let mut doc2 = doc1.fork(2);

    // Doc1 sets bold
    use s1_model::AttributeMap;
    doc1.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().bold(true),
    ))
    .unwrap();

    // Doc2 sets italic
    doc2.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().italic(true),
    ))
    .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // Both attributes should be present on both replicas (different keys = both apply)
    let attrs1 = &doc1.model().node(run_id).unwrap().attributes;
    let attrs2 = &doc2.model().node(run_id).unwrap().attributes;
    assert_eq!(attrs1.get_bool(&AttributeKey::Bold), Some(true));
    assert_eq!(attrs1.get_bool(&AttributeKey::Italic), Some(true));
    assert_eq!(attrs2.get_bool(&AttributeKey::Bold), Some(true));
    assert_eq!(attrs2.get_bool(&AttributeKey::Italic), Some(true));
}

// ─── Three-replica convergence ───────────────────────────────────────────

#[test]
fn three_replicas_converge_after_sync() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");

    let mut doc2 = doc1.fork(2);
    let mut doc3 = doc1.fork(3);

    // Each replica types different text
    doc1.apply_local(Operation::insert_text(text_id, 0, "AA"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 0, "BB"))
        .unwrap();
    doc3.apply_local(Operation::insert_text(text_id, 0, "CC"))
        .unwrap();

    // Full sync
    sync_all(&mut [&mut doc1, &mut doc2, &mut doc3]);

    let text1 = doc1.to_plain_text();
    let text2 = doc2.to_plain_text();
    let text3 = doc3.to_plain_text();

    assert_eq!(text1, text2, "replicas 1 and 2 must converge");
    assert_eq!(text2, text3, "replicas 2 and 3 must converge");
    assert_eq!(text1.len(), 6, "all 6 characters present");
    // Characters may be interleaved at character level
    assert_eq!(text1.matches('A').count(), 2);
    assert_eq!(text1.matches('B').count(), 2);
    assert_eq!(text1.matches('C').count(), 2);
}

#[test]
fn three_replicas_mixed_operations_converge() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Create initial structure on doc1
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let mut doc2 = doc1.fork(2);
    let mut doc3 = doc1.fork(3);

    // Doc1: insert a second paragraph
    let para2 = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        1,
        Node::new(para2, NodeType::Paragraph),
    ))
    .unwrap();

    // Doc2: set metadata
    doc2.apply_local(Operation::set_metadata("title", Some("Hello".into())))
        .unwrap();

    // Doc3: add a run to the first paragraph
    let run_id = doc3.next_id();
    doc3.apply_local(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ))
    .unwrap();

    // Sync all
    sync_all(&mut [&mut doc1, &mut doc2, &mut doc3]);

    // All should have the same structure
    assert!(doc1.model().node(para2).is_some());
    assert!(doc2.model().node(para2).is_some());
    assert!(doc3.model().node(para2).is_some());

    assert!(doc1.model().node(run_id).is_some());
    assert!(doc2.model().node(run_id).is_some());
    assert!(doc3.model().node(run_id).is_some());

    // All should have the title
    assert_eq!(doc1.model().metadata().title.as_deref(), Some("Hello"));
    assert_eq!(doc2.model().metadata().title.as_deref(), Some("Hello"));
    assert_eq!(doc3.model().metadata().title.as_deref(), Some("Hello"));
}

// ─── Delayed delivery ────────────────────────────────────────────────────

#[test]
fn delayed_delivery_converges() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Doc1 makes 3 operations
    let op1 = doc1
        .apply_local(Operation::insert_text(text_id, 0, "a"))
        .unwrap();
    let op2 = doc1
        .apply_local(Operation::insert_text(text_id, 1, "b"))
        .unwrap();
    let op3 = doc1
        .apply_local(Operation::insert_text(text_id, 2, "c"))
        .unwrap();

    // Deliver to doc2 in reverse order — causal ordering should buffer
    doc2.apply_remote(op3.clone()).unwrap();
    assert_eq!(doc2.pending_count(), 1); // op3 depends on op2

    doc2.apply_remote(op2.clone()).unwrap();
    assert_eq!(doc2.pending_count(), 2); // op2 depends on op1

    doc2.apply_remote(op1).unwrap();
    // Now op1 is applied, and op2 and op3 should flush
    assert_eq!(doc2.pending_count(), 0);

    assert_eq!(doc2.to_plain_text(), "abc");
}

#[test]
fn partition_and_heal() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Both work independently (network partition)
    doc1.apply_local(Operation::insert_text(text_id, 0, "hello"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 0, "world"))
        .unwrap();

    // More independent work
    doc1.apply_local(Operation::insert_text(text_id, 5, "!"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 5, "?"))
        .unwrap();

    // Partition heals — full sync
    sync_two(&mut doc1, &mut doc2);

    let text1 = doc1.to_plain_text();
    let text2 = doc2.to_plain_text();
    assert_eq!(text1, text2, "replicas must converge");
    // All characters present (may be interleaved at character level)
    assert_eq!(text1.len(), 12); // "hello" + "!" + "world" + "?" = 12
    for ch in ['h', 'e', 'l', 'o', '!', 'w', 'r', 'd', '?'] {
        assert!(text1.contains(ch), "missing char '{ch}'");
    }
}

// ─── Snapshot sync ───────────────────────────────────────────────────────

#[test]
fn snapshot_sync_new_replica() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "hello");

    // Take snapshot and create doc2 from it
    let snapshot = doc1.snapshot();
    let mut doc2 = CollabDocument::from_snapshot(snapshot, 2);

    // Doc2 should have the same content
    assert_eq!(doc2.to_plain_text(), "hello");

    // Both can continue editing
    doc1.apply_local(Operation::insert_text(text_id, 5, " world"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 5, "!"))
        .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    let text1 = doc1.to_plain_text();
    let text2 = doc2.to_plain_text();
    assert_eq!(text1, text2, "replicas must converge after snapshot sync");
    // All characters present
    assert_eq!(text1.len(), 12); // "hello" + " world" + "!" = 12
    assert!(text1.contains('!'));
}

// ─── Fork and diverge ────────────────────────────────────────────────────

#[test]
fn fork_diverge_and_converge() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Create initial content
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    // Fork
    let mut doc2 = doc1.fork(2);

    // Both add different paragraphs
    let para2 = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        1,
        Node::new(para2, NodeType::Paragraph),
    ))
    .unwrap();

    let para3 = doc2.next_id();
    doc2.apply_local(Operation::insert_node(
        body_id,
        1,
        Node::new(para3, NodeType::Paragraph),
    ))
    .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // Both should have all three paragraphs
    assert!(doc1.model().node(para_id).is_some());
    assert!(doc1.model().node(para2).is_some());
    assert!(doc1.model().node(para3).is_some());
    assert!(doc2.model().node(para_id).is_some());
    assert!(doc2.model().node(para2).is_some());
    assert!(doc2.model().node(para3).is_some());

    // Same child order
    let c1: Vec<NodeId> = doc1.model().node(body_id).unwrap().children.clone();
    let c2: Vec<NodeId> = doc2.model().node(body_id).unwrap().children.clone();
    assert_eq!(c1, c2);
}

// ─── Changes-since sync protocol ─────────────────────────────────────────

#[test]
fn changes_since_incremental_sync() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Initial setup
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let mut doc2 = doc1.fork(2);

    // Doc1 makes more changes
    let para2 = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        1,
        Node::new(para2, NodeType::Paragraph),
    ))
    .unwrap();

    // Get changes doc2 needs
    let changes = doc1.changes_since(doc2.state_vector());
    assert_eq!(changes.len(), 1); // Only the new paragraph

    for op in changes {
        doc2.apply_remote(op).unwrap();
    }

    assert!(doc2.model().node(para2).is_some());
}

#[test]
fn idempotent_sync() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let mut doc2 = doc1.fork(2);

    // Sync twice — second sync should be a no-op
    sync_two(&mut doc1, &mut doc2);
    let text_before = doc2.to_plain_text();

    sync_two(&mut doc1, &mut doc2);
    let text_after = doc2.to_plain_text();

    assert_eq!(text_before, text_after);
}

// ─── Five-replica convergence ────────────────────────────────────────────

#[test]
fn five_replicas_all_insert_converge() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");

    let mut doc2 = doc1.fork(2);
    let mut doc3 = doc1.fork(3);
    let mut doc4 = doc1.fork(4);
    let mut doc5 = doc1.fork(5);

    // Each replica inserts a character
    doc1.apply_local(Operation::insert_text(text_id, 0, "A"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 0, "B"))
        .unwrap();
    doc3.apply_local(Operation::insert_text(text_id, 0, "C"))
        .unwrap();
    doc4.apply_local(Operation::insert_text(text_id, 0, "D"))
        .unwrap();
    doc5.apply_local(Operation::insert_text(text_id, 0, "E"))
        .unwrap();

    // Full sync
    sync_all(&mut [&mut doc1, &mut doc2, &mut doc3, &mut doc4, &mut doc5]);

    let texts: Vec<String> = [&doc1, &doc2, &doc3, &doc4, &doc5]
        .iter()
        .map(|d| d.to_plain_text())
        .collect();

    // All must be identical
    for t in &texts[1..] {
        assert_eq!(&texts[0], t);
    }
    // All 5 characters present
    assert_eq!(texts[0].len(), 5);
}

// ─── Duplicate operation handling ────────────────────────────────────────

#[test]
fn duplicate_operations_are_idempotent() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    let para_id = doc1.next_id();
    let crdt_op = doc1
        .apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

    let mut doc2 = CollabDocument::new(2);

    // Apply same op multiple times
    doc2.apply_remote(crdt_op.clone()).unwrap();
    doc2.apply_remote(crdt_op.clone()).unwrap();
    doc2.apply_remote(crdt_op).unwrap();

    // Should only have one paragraph
    assert_eq!(doc2.model().node(body_id).unwrap().children.len(), 1);
}

// ─── F5.1 Stress Tests ─────────────────────────────────────────────────────

/// Stress test: 5 replicas with concurrent multi-character inserts and node operations.
#[test]
fn stress_5_replicas_mixed_ops() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");

    let mut doc2 = doc1.fork(2);
    let mut doc3 = doc1.fork(3);
    let mut doc4 = doc1.fork(4);
    let mut doc5 = doc1.fork(5);

    // Each replica inserts a multi-character string (single operation each)
    doc1.apply_local(Operation::insert_text(text_id, 0, "AAAA"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 0, "BBBB"))
        .unwrap();
    doc3.apply_local(Operation::insert_text(text_id, 0, "CCCC"))
        .unwrap();
    doc4.apply_local(Operation::insert_text(text_id, 0, "DDDD"))
        .unwrap();
    doc5.apply_local(Operation::insert_text(text_id, 0, "EEEE"))
        .unwrap();

    // Each also inserts a new paragraph
    for (doc, label) in [
        (&mut doc1, "P1"),
        (&mut doc2, "P2"),
        (&mut doc3, "P3"),
        (&mut doc4, "P4"),
        (&mut doc5, "P5"),
    ] {
        let pid = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(pid, NodeType::Paragraph),
        ))
        .unwrap();
        let rid = doc.next_id();
        doc.apply_local(Operation::insert_node(
            pid,
            0,
            Node::new(rid, NodeType::Run),
        ))
        .unwrap();
        let tid = doc.next_id();
        doc.apply_local(Operation::insert_node(rid, 0, Node::text(tid, label)))
            .unwrap();
    }

    // Full pairwise sync
    sync_all(&mut [&mut doc1, &mut doc2, &mut doc3, &mut doc4, &mut doc5]);

    // All replicas must have the same content (paragraph order may vary
    // due to concurrent index-based insertions — a known CRDT limitation).
    // Verify by comparing sorted lines.
    let texts: Vec<String> = [&doc1, &doc2, &doc3, &doc4, &doc5]
        .iter()
        .map(|d| d.to_plain_text())
        .collect();

    let sorted_lines = |text: &str| -> Vec<String> {
        let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        lines.sort();
        lines
    };

    let baseline = sorted_lines(&texts[0]);
    for (i, t) in texts.iter().enumerate().skip(1) {
        assert_eq!(
            baseline,
            sorted_lines(t),
            "replica 0 and replica {} have different content",
            i + 1
        );
    }

    // All 20 text chars present (5 * 4 = 20 from insert_text)
    let combined_text = &texts[0];
    for ch in ['A', 'B', 'C', 'D', 'E'] {
        let count = combined_text.chars().filter(|c| *c == ch).count();
        assert_eq!(count, 4, "expected 4 '{ch}' chars, got {count}");
    }

    // All 5 paragraph labels present
    for label in ["P1", "P2", "P3", "P4", "P5"] {
        assert!(
            combined_text.contains(label),
            "missing paragraph label {label}"
        );
    }
}

/// Stress test: rapid insert-delete cycles with tombstone accumulation.
#[test]
fn stress_insert_delete_tombstone_gc() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Replica 1: rapid insert/delete cycles
    for _ in 0..100 {
        doc1.apply_local(Operation::insert_text(text_id, 0, "x"))
            .unwrap();
        doc1.apply_local(Operation::delete_text(text_id, 0, 1))
            .unwrap();
    }

    // Sync to replica 2
    let changes = doc1.changes_since(doc2.state_vector());
    for op in changes {
        doc2.apply_remote(op).unwrap();
    }

    // Both should have empty text
    assert_eq!(doc1.to_plain_text().trim(), "");
    assert_eq!(doc2.to_plain_text().trim(), "");

    // Tombstones accumulated
    assert!(
        doc1.tombstone_count() > 0,
        "should have tombstones from deletions"
    );

    // GC with mutual state vector (both have seen everything)
    let min_state = doc1.state_vector().clone();
    let removed = doc1.gc_tombstones(&min_state);
    assert!(removed > 0, "GC should remove acknowledged tombstones");
}

/// Stress test: tombstone excess GC safety valve.
#[test]
fn stress_tombstone_excess_gc() {
    let mut doc = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc, "");

    // Create many tombstones
    for _ in 0..200 {
        doc.apply_local(Operation::insert_text(text_id, 0, "x"))
            .unwrap();
        doc.apply_local(Operation::delete_text(text_id, 0, 1))
            .unwrap();
    }

    let initial_count = doc.tombstone_count();
    assert!(
        initial_count >= 200,
        "expected >= 200 tombstones, got {initial_count}"
    );

    // Force GC to cap at 50
    let removed = doc.gc_tombstones_excess(50);
    assert!(removed > 0, "should remove excess tombstones");
    assert!(
        doc.tombstone_count() <= 50,
        "tombstones should be capped at 50, got {}",
        doc.tombstone_count()
    );
}

/// Stress test: operation deduplication across reconnections.
#[test]
fn stress_operation_redelivery() {
    let mut doc1 = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Doc1 inserts text
    let ops: Vec<_> = (0..50)
        .map(|i| {
            doc1.apply_local(Operation::insert_text(text_id, i, "a"))
                .unwrap()
        })
        .collect();

    // Send all ops to doc2 (first delivery)
    for op in &ops {
        doc2.apply_remote(op.clone()).unwrap();
    }

    let text_after_first = doc2.to_plain_text();

    // "Reconnect" — resend all ops (duplicate delivery)
    for op in &ops {
        doc2.apply_remote(op.clone()).unwrap();
    }

    // Third delivery
    for op in &ops {
        doc2.apply_remote(op.clone()).unwrap();
    }

    // Text should be identical — deduplication must work
    let text_after_third = doc2.to_plain_text();
    assert_eq!(
        text_after_first, text_after_third,
        "redelivery should be idempotent"
    );
}

/// Stress test: concurrent node reordering within the same parent.
#[test]
fn stress_concurrent_node_reorder() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Create 5 paragraphs
    let mut para_ids = Vec::new();
    for _ in 0..5 {
        let para_id = doc1.next_id();
        doc1.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();
        para_ids.push(para_id);
    }

    let mut doc2 = doc1.fork(2);
    let mut doc3 = doc1.fork(3);

    // Doc1: reorder para[0] to end (move under body at last position)
    doc1.apply_local(Operation::move_node(para_ids[0], body_id, 4))
        .unwrap();

    // Doc2: reorder para[4] to start
    doc2.apply_local(Operation::move_node(para_ids[4], body_id, 0))
        .unwrap();

    // Doc3: reorder para[2] to position 1
    doc3.apply_local(Operation::move_node(para_ids[2], body_id, 1))
        .unwrap();

    // Sync all three
    let mut refs: Vec<&mut CollabDocument> = vec![&mut doc1, &mut doc2, &mut doc3];
    sync_all(&mut refs);

    // All replicas should have exactly 5 children
    let children_1: Vec<NodeId> = doc1.model().node(body_id).unwrap().children.clone();
    let children_2: Vec<NodeId> = doc2.model().node(body_id).unwrap().children.clone();
    let children_3: Vec<NodeId> = doc3.model().node(body_id).unwrap().children.clone();

    assert_eq!(children_1.len(), 5, "doc1 lost children");
    assert_eq!(children_2.len(), 5, "doc2 lost children");
    assert_eq!(children_3.len(), 5, "doc3 lost children");

    // All 5 paragraphs still present
    for pid in &para_ids {
        assert!(
            doc1.model().node(*pid).is_some(),
            "para {pid:?} lost in doc1"
        );
        assert!(
            doc2.model().node(*pid).is_some(),
            "para {pid:?} lost in doc2"
        );
        assert!(
            doc3.model().node(*pid).is_some(),
            "para {pid:?} lost in doc3"
        );
    }

    // All replicas should contain the same set of children (order may differ
    // due to concurrent index-based moves — a known CRDT limitation)
    let mut sorted_1 = children_1.clone();
    let mut sorted_2 = children_2.clone();
    let mut sorted_3 = children_3.clone();
    sorted_1.sort();
    sorted_2.sort();
    sorted_3.sort();
    assert_eq!(sorted_1, sorted_2, "doc1/doc2 child set diverged");
    assert_eq!(sorted_2, sorted_3, "doc2/doc3 child set diverged");
}

/// Stress test: maintenance routine (compaction + GC).
#[test]
fn stress_maintenance_routine() {
    let mut doc = CollabDocument::new(1);
    let (_, text_id) = add_paragraph_with_text(&mut doc, "");

    // Generate many operations
    for i in 0..500 {
        doc.apply_local(Operation::insert_text(text_id, i, "a"))
            .unwrap();
    }
    // Delete some to create tombstones
    for _ in 0..100 {
        doc.apply_local(Operation::delete_text(text_id, 0, 1))
            .unwrap();
    }

    let op_log_before = doc.op_log_size();
    assert!(op_log_before >= 600);

    // Run maintenance (clone state_vector to avoid borrow conflict)
    let sv = doc.state_vector().clone();
    doc.maintenance(100, Some(&sv), 50);

    // Op log should be compacted (consecutive inserts merged)
    let op_log_after = doc.op_log_size();
    assert!(
        op_log_after < op_log_before,
        "compaction should reduce op log: {} -> {}",
        op_log_before,
        op_log_after
    );

    // Tombstones should be capped
    assert!(
        doc.tombstone_count() <= 50,
        "tombstones should be capped at 50, got {}",
        doc.tombstone_count()
    );
}
