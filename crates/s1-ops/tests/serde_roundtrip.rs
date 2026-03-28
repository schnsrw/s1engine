//! Serde serialization round-trip tests for s1-ops types.
//! Only compiled when the "serde" feature is enabled.

#![cfg(feature = "serde")]

use s1_model::*;
use s1_ops::*;

#[test]
fn operation_insert_text_serde_roundtrip() {
    let op = Operation::InsertText {
        target_id: NodeId::new(0, 5),
        offset: 3,
        text: "hello".to_string(),
    };

    let json = serde_json::to_string(&op).expect("serialize");
    let rt: Operation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(op, rt);
}

#[test]
fn operation_set_attributes_serde_roundtrip() {
    let mut attrs = AttributeMap::new();
    attrs.set(AttributeKey::Bold, AttributeValue::Bool(true));
    attrs.set(AttributeKey::FontSize, AttributeValue::Float(14.0));
    attrs.set(
        AttributeKey::Color,
        AttributeValue::Color(Color::new(255, 0, 0)),
    );

    let op = Operation::SetAttributes {
        target_id: NodeId::new(1, 10),
        attributes: attrs,
        previous: None,
    };

    let json = serde_json::to_string(&op).expect("serialize");
    let rt: Operation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(op, rt);
}

#[test]
fn operation_insert_node_serde_roundtrip() {
    let mut node = Node::new(NodeId::new(0, 42), NodeType::Paragraph);
    node.attributes
        .set(AttributeKey::Alignment, AttributeValue::Alignment(Alignment::Center));

    let op = Operation::InsertNode {
        parent_id: NodeId::new(0, 1),
        index: 0,
        node,
    };

    let json = serde_json::to_string_pretty(&op).expect("serialize");
    let rt: Operation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(op, rt);
}

#[test]
fn operation_delete_node_serde_roundtrip() {
    let op = Operation::DeleteNode {
        target_id: NodeId::new(0, 7),
        parent_id: Some(NodeId::new(0, 1)),
        index: Some(2),
        snapshot: Some(vec![
            Node::new(NodeId::new(0, 7), NodeType::Run),
            Node::text(NodeId::new(0, 8), "deleted text"),
        ]),
    };

    let json = serde_json::to_string(&op).expect("serialize");
    let rt: Operation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(op, rt);
}

#[test]
fn transaction_serde_roundtrip() {
    let mut txn = Transaction::with_label("Bold + size change");
    txn.push(Operation::SetAttributes {
        target_id: NodeId::new(0, 5),
        attributes: AttributeMap::new().bold(true).font_size(16.0),
        previous: None,
    });
    txn.push(Operation::InsertText {
        target_id: NodeId::new(0, 10),
        offset: 0,
        text: "prefix".to_string(),
    });

    let json = serde_json::to_string(&txn).expect("serialize");
    let rt: Transaction = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(txn, rt);
}

#[test]
fn position_selection_serde_roundtrip() {
    let pos = Position::new(NodeId::new(0, 5), 3);
    let json = serde_json::to_string(&pos).expect("serialize");
    let rt: Position = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(pos, rt);

    let sel = Selection::range(
        Position::new(NodeId::new(0, 5), 1),
        Position::new(NodeId::new(0, 5), 8),
    );
    let json = serde_json::to_string(&sel).expect("serialize");
    let rt: Selection = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(sel, rt);
}

#[test]
fn complex_attribute_values_serde_roundtrip() {
    let mut attrs = AttributeMap::new();
    attrs.set(
        AttributeKey::LineSpacing,
        AttributeValue::LineSpacing(LineSpacing::Multiple(1.5)),
    );
    attrs.set(
        AttributeKey::ParagraphBorders,
        AttributeValue::Borders(Borders {
            top: Some(BorderSide {
                style: BorderStyle::Single,
                width: 1.0,
                color: Color::BLACK,
                spacing: 0.0,
            }),
            bottom: None,
            left: None,
            right: None,
        }),
    );
    attrs.set(
        AttributeKey::TabStops,
        AttributeValue::TabStops(vec![TabStop {
            position: 72.0,
            alignment: TabAlignment::Right,
            leader: TabLeader::Dot,
        }]),
    );
    attrs.set(
        AttributeKey::ListInfo,
        AttributeValue::ListInfo(ListInfo {
            level: 1,
            num_format: ListFormat::Decimal,
            num_id: 3,
            start: Some(5),
        }),
    );

    let op = Operation::SetAttributes {
        target_id: NodeId::new(0, 1),
        attributes: attrs,
        previous: None,
    };

    let json = serde_json::to_string(&op).expect("serialize");
    let rt: Operation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(op, rt);
}
