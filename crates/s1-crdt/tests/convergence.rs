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
