//! Cursor position and selection types.
//!
//! Represents where the user's caret is in the document, and optionally
//! a selection range. Used by higher-level editing APIs.

use s1_model::{DocumentModel, NodeId, NodeType};

/// A position in the document — a specific point within a text node.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// The text node containing the cursor.
    pub node_id: NodeId,
    /// Character offset within the text node.
    pub offset: usize,
}

impl Position {
    /// Create a new position.
    pub fn new(node_id: NodeId, offset: usize) -> Self {
        Self { node_id, offset }
    }

    /// Validate that this position refers to a valid text node with a valid offset.
    pub fn validate(&self, model: &DocumentModel) -> Result<(), String> {
        let node = model
            .node(self.node_id)
            .ok_or_else(|| format!("Node {:?} not found in document", self.node_id))?;
        if node.node_type != NodeType::Text {
            return Err(format!(
                "Node {:?} is {:?}, not Text",
                self.node_id, node.node_type
            ));
        }
        let text_len = node
            .text_content
            .as_ref()
            .map(|t| t.chars().count())
            .unwrap_or(0);
        if self.offset > text_len {
            return Err(format!(
                "Offset {} exceeds text length {} at node {:?}",
                self.offset, text_len, self.node_id
            ));
        }
        Ok(())
    }
}

/// A selection range in the document.
///
/// When `anchor == focus`, this is a collapsed selection (a simple cursor).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    /// The anchor (start) of the selection.
    pub anchor: Position,
    /// The focus (end) of the selection. Equals anchor for a collapsed selection.
    pub focus: Position,
}

impl Selection {
    /// Validate that both anchor and focus refer to valid positions.
    pub fn validate(&self, model: &DocumentModel) -> Result<(), String> {
        self.anchor
            .validate(model)
            .map_err(|e| format!("Anchor: {e}"))?;
        self.focus
            .validate(model)
            .map_err(|e| format!("Focus: {e}"))?;
        Ok(())
    }

    /// A collapsed selection (cursor with no range).
    pub fn collapsed(pos: Position) -> Self {
        Self {
            anchor: pos,
            focus: pos,
        }
    }

    /// A range selection from anchor to focus.
    pub fn range(anchor: Position, focus: Position) -> Self {
        Self { anchor, focus }
    }

    /// Returns `true` if this is a collapsed selection (no range).
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }

    /// Returns the distinct node IDs referenced by this selection.
    /// For a collapsed selection, returns one ID. For a range across two different
    /// nodes, returns both anchor and focus node IDs.
    ///
    /// Note: this does NOT return intermediate nodes between anchor and focus.
    /// Use `node_ids_in_range()` with a `DocumentModel` to get all spanned nodes.
    pub fn node_ids(&self) -> Vec<NodeId> {
        if self.anchor.node_id == self.focus.node_id {
            vec![self.anchor.node_id]
        } else {
            vec![self.anchor.node_id, self.focus.node_id]
        }
    }

    /// Returns all text node IDs spanned by this selection, in document order.
    ///
    /// Requires the document model to traverse between anchor and focus.
    /// Returns an empty vec if either anchor or focus node is not found.
    pub fn node_ids_in_range(&self, model: &DocumentModel) -> Vec<NodeId> {
        if self.anchor.node_id == self.focus.node_id {
            return vec![self.anchor.node_id];
        }

        // Collect all text nodes in document order
        let all_text_nodes = Self::collect_text_nodes(model);

        // Find positions of anchor and focus in the ordered list
        let anchor_pos = all_text_nodes
            .iter()
            .position(|&id| id == self.anchor.node_id);
        let focus_pos = all_text_nodes
            .iter()
            .position(|&id| id == self.focus.node_id);

        match (anchor_pos, focus_pos) {
            (Some(a), Some(f)) => {
                let start = a.min(f);
                let end = a.max(f);
                all_text_nodes[start..=end].to_vec()
            }
            _ => vec![self.anchor.node_id, self.focus.node_id],
        }
    }

    /// Collect all text node IDs in document order via DFS.
    fn collect_text_nodes(model: &DocumentModel) -> Vec<NodeId> {
        let mut result = Vec::new();
        Self::dfs_text_nodes(model, model.root_id(), &mut result);
        result
    }

    fn dfs_text_nodes(model: &DocumentModel, node_id: NodeId, result: &mut Vec<NodeId>) {
        if let Some(node) = model.node(node_id) {
            if node.node_type == NodeType::Text {
                result.push(node_id);
            }
            for &child_id in &node.children {
                Self::dfs_text_nodes(model, child_id, result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_selection() {
        let pos = Position::new(NodeId::new(0, 5), 3);
        let sel = Selection::collapsed(pos);
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor, sel.focus);
        assert_eq!(sel.node_ids().len(), 1);
    }

    #[test]
    fn range_selection_same_node() {
        let anchor = Position::new(NodeId::new(0, 5), 0);
        let focus = Position::new(NodeId::new(0, 5), 10);
        let sel = Selection::range(anchor, focus);
        assert!(!sel.is_collapsed());
        assert_eq!(sel.node_ids().len(), 1);
    }

    #[test]
    fn range_selection_different_nodes() {
        let anchor = Position::new(NodeId::new(0, 5), 0);
        let focus = Position::new(NodeId::new(0, 8), 3);
        let sel = Selection::range(anchor, focus);
        assert!(!sel.is_collapsed());
        assert_eq!(sel.node_ids().len(), 2);
    }

    #[test]
    fn position_equality() {
        let a = Position::new(NodeId::new(0, 1), 5);
        let b = Position::new(NodeId::new(0, 1), 5);
        let c = Position::new(NodeId::new(0, 1), 6);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn node_ids_in_range_same_node() {
        use s1_model::{DocumentModel, Node, NodeType};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Hello"))
            .unwrap();

        let sel = Selection::range(Position::new(text_id, 0), Position::new(text_id, 3));
        let ids = sel.node_ids_in_range(&doc);
        assert_eq!(ids, vec![text_id]);
    }

    #[test]
    fn node_ids_in_range_multiple_nodes() {
        use s1_model::{DocumentModel, Node, NodeType};

        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Paragraph 1 with text
        let para1_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para1_id, NodeType::Paragraph))
            .unwrap();
        let run1_id = doc.next_id();
        doc.insert_node(para1_id, 0, Node::new(run1_id, NodeType::Run))
            .unwrap();
        let text1_id = doc.next_id();
        doc.insert_node(run1_id, 0, Node::text(text1_id, "Hello"))
            .unwrap();

        // Paragraph 2 with text
        let para2_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para2_id, NodeType::Paragraph))
            .unwrap();
        let run2_id = doc.next_id();
        doc.insert_node(para2_id, 0, Node::new(run2_id, NodeType::Run))
            .unwrap();
        let text2_id = doc.next_id();
        doc.insert_node(run2_id, 0, Node::text(text2_id, "Middle"))
            .unwrap();

        // Paragraph 3 with text
        let para3_id = doc.next_id();
        doc.insert_node(body_id, 2, Node::new(para3_id, NodeType::Paragraph))
            .unwrap();
        let run3_id = doc.next_id();
        doc.insert_node(para3_id, 0, Node::new(run3_id, NodeType::Run))
            .unwrap();
        let text3_id = doc.next_id();
        doc.insert_node(run3_id, 0, Node::text(text3_id, "World"))
            .unwrap();

        // Select from text1 to text3 — should include text2
        let sel = Selection::range(Position::new(text1_id, 0), Position::new(text3_id, 3));
        let ids = sel.node_ids_in_range(&doc);
        assert_eq!(ids, vec![text1_id, text2_id, text3_id]);
    }
}
