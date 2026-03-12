//! Specific concurrent editing scenarios.
//!
//! Tests for particular conflict patterns that arise in real-world
//! collaborative editing.

use s1_crdt::CollabDocument;
use s1_model::{AttributeKey, AttributeMap, Node, NodeId, NodeType, Style, StyleType};
use s1_ops::Operation;

/// Helper: synchronize all operations between two replicas.
fn sync_two(doc1: &mut CollabDocument, doc2: &mut CollabDocument) {
    let changes_1_to_2 = doc1.changes_since(doc2.state_vector());
    for op in changes_1_to_2 {
        doc2.apply_remote(op).unwrap();
    }

    let changes_2_to_1 = doc2.changes_since(doc1.state_vector());
    for op in changes_2_to_1 {
        doc1.apply_remote(op).unwrap();
    }
}

/// Helper: build a paragraph with text in a collab document.
fn add_paragraph_with_text(doc: &mut CollabDocument, text: &str) -> (NodeId, NodeId, NodeId) {
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

    (para_id, run_id, text_id)
}

// ─── Concurrent insert at same offset ────────────────────────────────────

#[test]
fn concurrent_insert_at_same_offset_both_preserved() {
    let mut doc1 = CollabDocument::new(1);
    let (_, _, text_id) = add_paragraph_with_text(&mut doc1, "ac");
    let mut doc2 = doc1.fork(2);

    // Both insert between 'a' and 'c' (offset 1)
    doc1.apply_local(Operation::insert_text(text_id, 1, "X"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 1, "Y"))
        .unwrap();

    sync_two(&mut doc1, &mut doc2);

    let text1 = doc1.to_plain_text();
    let text2 = doc2.to_plain_text();
    assert_eq!(text1, text2);
    // Both characters preserved
    assert!(text1.contains('X'));
    assert!(text1.contains('Y'));
    assert!(text1.contains('a'));
    assert!(text1.contains('c'));
    assert_eq!(text1.len(), 4);
}

#[test]
fn concurrent_insert_deterministic_order() {
    // Two independent setups with the same replica IDs produce the same text ordering.
    let mut doc1a = CollabDocument::new(1);
    let (_, _, text_id_a) = add_paragraph_with_text(&mut doc1a, "");
    let mut doc2a = doc1a.fork(2);

    let mut doc1b = CollabDocument::new(1);
    let (_, _, text_id_b) = add_paragraph_with_text(&mut doc1b, "");
    let mut doc2b = doc1b.fork(2);

    // Scenario A
    doc1a
        .apply_local(Operation::insert_text(text_id_a, 0, "X"))
        .unwrap();
    doc2a
        .apply_local(Operation::insert_text(text_id_a, 0, "Y"))
        .unwrap();
    sync_two(&mut doc1a, &mut doc2a);

    // Scenario B — same operations, independent docs
    doc1b
        .apply_local(Operation::insert_text(text_id_b, 0, "X"))
        .unwrap();
    doc2b
        .apply_local(Operation::insert_text(text_id_b, 0, "Y"))
        .unwrap();
    sync_two(&mut doc1b, &mut doc2b);

    // Both scenarios should produce the same character ordering
    assert_eq!(doc1a.to_plain_text(), doc1b.to_plain_text());
}

// ─── Concurrent formatting ──────────────────────────────────────────────

#[test]
fn concurrent_bold_and_italic_both_apply() {
    let mut doc1 = CollabDocument::new(1);
    let (_, run_id, _) = add_paragraph_with_text(&mut doc1, "hello");
    let mut doc2 = doc1.fork(2);

    // Doc1 sets bold on the run
    doc1.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().bold(true),
    ))
    .unwrap();

    // Doc2 sets italic on the run
    doc2.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().italic(true),
    ))
    .unwrap();

    sync_two(&mut doc1, &mut doc2);

    // Both formatting attributes should be present
    let attrs1 = &doc1.model().node(run_id).unwrap().attributes;
    let attrs2 = &doc2.model().node(run_id).unwrap().attributes;
    assert_eq!(attrs1.get_bool(&AttributeKey::Bold), Some(true));
    assert_eq!(attrs1.get_bool(&AttributeKey::Italic), Some(true));
    assert_eq!(attrs2.get_bool(&AttributeKey::Bold), Some(true));
    assert_eq!(attrs2.get_bool(&AttributeKey::Italic), Some(true));
}

#[test]
fn concurrent_same_attribute_lww() {
    let mut doc1 = CollabDocument::new(1);
    let (_, run_id, _) = add_paragraph_with_text(&mut doc1, "hello");
    let mut doc2 = doc1.fork(2);

    // Both set bold, but to different values — LWW decides
    doc1.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().bold(true),
    ))
    .unwrap();

    doc2.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().bold(false),
    ))
    .unwrap();

    sync_two(&mut doc1, &mut doc2);

    // Both replicas agree on the same value (LWW by OpId)
    let bold1 = doc1
        .model()
        .node(run_id)
        .unwrap()
        .attributes
        .get_bool(&AttributeKey::Bold);
    let bold2 = doc2
        .model()
        .node(run_id)
        .unwrap()
        .attributes
        .get_bool(&AttributeKey::Bold);
    assert_eq!(bold1, bold2);
}

// ─── Delete + concurrent insert ──────────────────────────────────────────

#[test]
fn delete_node_while_other_modifies_it() {
    let mut doc1 = CollabDocument::new(1);
    let (para_id, run_id, _) = add_paragraph_with_text(&mut doc1, "hello");
    let mut doc2 = doc1.fork(2);

    // Doc1 deletes the paragraph
    doc1.apply_local(Operation::delete_node(para_id)).unwrap();

    // Doc2 adds formatting to the run (in the paragraph doc1 deleted)
    doc2.apply_local(Operation::set_attributes(
        run_id,
        AttributeMap::new().bold(true),
    ))
    .unwrap();

    sync_two(&mut doc1, &mut doc2);

    // Delete should win — paragraph gone from doc1
    assert!(doc1.model().node(para_id).is_none());
}

#[test]
fn concurrent_delete_same_node() {
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

    // Both delete the same paragraph
    doc1.apply_local(Operation::delete_node(para_id)).unwrap();
    doc2.apply_local(Operation::delete_node(para_id)).unwrap();

    sync_two(&mut doc1, &mut doc2);

    // Both agree it's deleted
    assert!(doc1.model().node(para_id).is_none());
    assert!(doc2.model().node(para_id).is_none());
}

// ─── Concurrent metadata ────────────────────────────────────────────────

#[test]
fn concurrent_metadata_different_keys() {
    let mut doc1 = CollabDocument::new(1);
    let mut doc2 = doc1.fork(2);

    // Set different metadata keys
    doc1.apply_local(Operation::set_metadata("title", Some("My Doc".into())))
        .unwrap();
    doc2.apply_local(Operation::set_metadata("creator", Some("Alice".into())))
        .unwrap();

    sync_two(&mut doc1, &mut doc2);

    // Both keys should be set on both replicas
    assert_eq!(doc1.model().metadata().title.as_deref(), Some("My Doc"));
    assert_eq!(doc1.model().metadata().creator.as_deref(), Some("Alice"));
    assert_eq!(doc2.model().metadata().title.as_deref(), Some("My Doc"));
    assert_eq!(doc2.model().metadata().creator.as_deref(), Some("Alice"));
}

// ─── Concurrent styles ──────────────────────────────────────────────────

#[test]
fn concurrent_style_updates_lww() {
    let mut doc1 = CollabDocument::new(1);
    let mut doc2 = doc1.fork(2);

    let style1 = Style {
        id: "heading1".to_string(),
        name: "Heading 1 v1".to_string(),
        style_type: StyleType::Paragraph,
        parent_id: None,
        next_style_id: None,
        attributes: AttributeMap::new().bold(true),
        is_default: false,
    };
    let style2 = Style {
        id: "heading1".to_string(),
        name: "Heading 1 v2".to_string(),
        style_type: StyleType::Paragraph,
        parent_id: None,
        next_style_id: None,
        attributes: AttributeMap::new().italic(true),
        is_default: false,
    };

    doc1.apply_local(Operation::set_style(style1)).unwrap();
    doc2.apply_local(Operation::set_style(style2)).unwrap();

    sync_two(&mut doc1, &mut doc2);

    // Both should have the same style (LWW by OpId)
    let s1 = doc1.model().styles().iter().find(|s| s.id == "heading1");
    let s2 = doc2.model().styles().iter().find(|s| s.id == "heading1");
    assert_eq!(s1.map(|s| s.name.clone()), s2.map(|s| s.name.clone()));
}

// ─── Undo with remote awareness ─────────────────────────────────────────

#[test]
fn undo_only_affects_local_operations() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    // Doc1 creates a paragraph
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    let mut doc2 = doc1.fork(2);

    // Doc2 adds a run
    let run_id = doc2.next_id();
    doc2.apply_local(Operation::insert_node(
        para_id,
        0,
        Node::new(run_id, NodeType::Run),
    ))
    .unwrap();

    // Sync doc2's changes to doc1
    let changes = doc2.changes_since(doc1.state_vector());
    for op in changes {
        doc1.apply_remote(op).unwrap();
    }

    assert!(doc1.model().node(run_id).is_some());

    // Doc1 undoes — should undo its own paragraph insert, not doc2's run
    let undo_result = doc1.undo().unwrap();
    assert!(undo_result.is_some());

    // The paragraph (doc1's operation) should be undone
    assert!(doc1.model().node(para_id).is_none());
}

// ─── Multiple operations batch ───────────────────────────────────────────

#[test]
fn batch_operations_all_arrive() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();
    let mut doc2 = doc1.fork(2);

    // Doc1 creates multiple paragraphs
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

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // All paragraphs should be on doc2
    for pid in &para_ids {
        assert!(doc2.model().node(*pid).is_some());
    }
}

// ─── Operation log and state vector ──────────────────────────────────────

#[test]
fn op_log_tracks_all_operations() {
    let mut doc = CollabDocument::new(1);
    let body_id = doc.model().body_id().unwrap();

    assert_eq!(doc.op_log().len(), 0);

    let para_id = doc.next_id();
    doc.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();
    assert_eq!(doc.op_log().len(), 1);

    doc.apply_local(Operation::set_metadata("title", Some("Test".into())))
        .unwrap();
    assert_eq!(doc.op_log().len(), 2);
}

#[test]
fn state_vector_reflects_all_replicas() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();
    let mut doc2 = doc1.fork(2);

    // Doc1 makes an op
    let para_id = doc1.next_id();
    doc1.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id, NodeType::Paragraph),
    ))
    .unwrap();

    // Doc2 makes an op
    let para_id2 = doc2.next_id();
    doc2.apply_local(Operation::insert_node(
        body_id,
        0,
        Node::new(para_id2, NodeType::Paragraph),
    ))
    .unwrap();

    // Sync
    sync_two(&mut doc1, &mut doc2);

    // State vectors should include both replicas
    let sv1 = doc1.state_vector();
    let sv2 = doc2.state_vector();

    // Both should track replica 1 and replica 2
    assert!(sv1.get(1) > 0);
    assert!(sv1.get(2) > 0);
    assert!(sv2.get(1) > 0);
    assert!(sv2.get(2) > 0);
}

// ─── Awareness/cursor ────────────────────────────────────────────────────

#[test]
fn awareness_cursor_sharing() {
    use s1_ops::{Position, Selection};

    let mut doc1 = CollabDocument::new(1);
    let mut doc2 = CollabDocument::new(2);

    let sel = Selection {
        anchor: Position {
            node_id: NodeId::new(0, 1),
            offset: 5,
        },
        focus: Position {
            node_id: NodeId::new(0, 1),
            offset: 5,
        },
    };

    let update = doc1.set_cursor(sel, "Alice", "#ff0000");
    doc2.apply_awareness_update(&update);

    let remote_cursors = doc2.awareness().remote_cursors();
    assert_eq!(remote_cursors.len(), 1);
    assert_eq!(remote_cursors[0].user_name, "Alice");
    assert_eq!(remote_cursors[0].selection.anchor.offset, 5);
}

// ─── Empty operations ────────────────────────────────────────────────────

#[test]
fn empty_sync_is_noop() {
    let mut doc1 = CollabDocument::new(1);
    let mut doc2 = doc1.fork(2);

    // No operations — sync should be empty
    let changes = doc1.changes_since(doc2.state_vector());
    assert!(changes.is_empty());

    sync_two(&mut doc1, &mut doc2);
    assert_eq!(doc1.to_plain_text(), doc2.to_plain_text());
}

// ─── Multi-char text operations ──────────────────────────────────────────

#[test]
fn multi_char_insert_syncs_correctly() {
    let mut doc1 = CollabDocument::new(1);
    let (_, _, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    // Doc1 inserts a multi-character string
    doc1.apply_local(Operation::insert_text(text_id, 0, "hello world"))
        .unwrap();

    sync_two(&mut doc1, &mut doc2);

    assert_eq!(doc2.to_plain_text(), "hello world");
}

#[test]
fn concurrent_multi_char_inserts_converge() {
    let mut doc1 = CollabDocument::new(1);
    let (_, _, text_id) = add_paragraph_with_text(&mut doc1, "");
    let mut doc2 = doc1.fork(2);

    doc1.apply_local(Operation::insert_text(text_id, 0, "hello"))
        .unwrap();
    doc2.apply_local(Operation::insert_text(text_id, 0, "world"))
        .unwrap();

    sync_two(&mut doc1, &mut doc2);

    let t1 = doc1.to_plain_text();
    let t2 = doc2.to_plain_text();
    assert_eq!(t1, t2);
    assert_eq!(t1.len(), 10);
}

// ─── Transaction support ─────────────────────────────────────────────────

#[test]
fn apply_local_transaction() {
    let mut doc1 = CollabDocument::new(1);
    let body_id = doc1.model().body_id().unwrap();

    let para_id = doc1.next_id();
    let run_id = doc1.next_id();

    let ops = vec![
        Operation::insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph)),
        Operation::insert_node(para_id, 0, Node::new(run_id, NodeType::Run)),
    ];

    let crdt_ops = doc1.apply_local_transaction(ops).unwrap();
    assert_eq!(crdt_ops.len(), 2);
    assert!(doc1.model().node(para_id).is_some());
    assert!(doc1.model().node(run_id).is_some());

    // Should sync correctly
    let mut doc2 = CollabDocument::new(2);
    for op in crdt_ops {
        doc2.apply_remote(op).unwrap();
    }
    assert!(doc2.model().node(para_id).is_some());
    assert!(doc2.model().node(run_id).is_some());
}
