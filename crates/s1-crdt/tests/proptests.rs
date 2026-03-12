//! Property-based tests for CRDT convergence.
//!
//! Uses proptest to generate random concurrent editing scenarios and
//! verify that replicas always converge after synchronization.

use proptest::prelude::*;
use s1_crdt::CollabDocument;
use s1_model::{Node, NodeId, NodeType};
use s1_ops::Operation;

/// Synchronize all operations between two replicas.
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

/// Build a paragraph with a text node in a collab document.
fn add_text_paragraph(doc: &mut CollabDocument, text: &str) -> NodeId {
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

    text_id
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(30))]

    #[test]
    fn concurrent_text_inserts_converge(
        text1 in "[a-z]{1,10}",
        text2 in "[A-Z]{1,10}",
    ) {
        // Two replicas start with the same initial text
        let mut doc1 = CollabDocument::new(1);
        let text_id = add_text_paragraph(&mut doc1, "base");

        // Fork doc2 from doc1
        let mut doc2 = doc1.fork(2);

        // Both insert concurrently at end of text
        doc1.apply_local(Operation::insert_text(text_id, 4, &text1)).unwrap();
        doc2.apply_local(Operation::insert_text(text_id, 4, &text2)).unwrap();

        // Sync
        sync_two(&mut doc1, &mut doc2);

        // Both replicas must have identical text (convergence)
        let plain1 = doc1.model().to_plain_text();
        let plain2 = doc2.model().to_plain_text();
        prop_assert_eq!(&plain1, &plain2, "replicas diverged");

        // All characters from both inserts must be present
        let result = &plain1;
        for ch in text1.chars() {
            prop_assert!(result.contains(ch), "char '{}' from text1 missing", ch);
        }
        for ch in text2.chars() {
            prop_assert!(result.contains(ch), "char '{}' from text2 missing", ch);
        }

        // Base text must still be present
        prop_assert!(result.contains("base"), "base text missing");
    }
}
