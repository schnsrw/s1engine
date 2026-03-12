//! Node types and the core `Node` struct.

use crate::attributes::AttributeMap;
use crate::id::NodeId;
use std::fmt;

/// The type of a document node. Maps to constructs in both OOXML and ODF.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeType {
    // Root
    /// The root document node. Exactly one per document.
    Document,

    // Structural
    /// The document body container.
    Body,
    /// A section with page layout properties (margins, orientation, columns).
    Section,

    // Block-level
    /// A paragraph containing inline content (runs, breaks, etc.).
    Paragraph,
    /// A table containing rows.
    Table,
    /// A table row containing cells.
    TableRow,
    /// A table cell containing block content (paragraphs, nested tables).
    TableCell,

    // Inline-level
    /// A run of text with uniform formatting.
    Run,
    /// Raw text content (leaf node, always child of Run).
    Text,
    /// A line break within a paragraph.
    LineBreak,
    /// A page break.
    PageBreak,
    /// A column break.
    ColumnBreak,
    /// A tab character.
    Tab,

    // Generated content
    /// A Table of Contents block. Contains cached entry paragraphs.
    TableOfContents,

    // Objects
    /// An inline or floating image.
    Image,
    /// A vector drawing or shape.
    Drawing,

    // Headers/Footers
    /// A page header.
    Header,
    /// A page footer.
    Footer,

    // Fields
    /// A dynamic field (page number, date, TOC, etc.).
    Field,

    // Annotations
    /// Start of a bookmark range.
    BookmarkStart,
    /// End of a bookmark range.
    BookmarkEnd,
    /// Start of a comment range.
    CommentStart,
    /// End of a comment range.
    CommentEnd,
    /// Comment content container.
    CommentBody,
}

impl NodeType {
    /// Returns true if this node type can contain children.
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            NodeType::Document
                | NodeType::Body
                | NodeType::Section
                | NodeType::Paragraph
                | NodeType::Table
                | NodeType::TableRow
                | NodeType::TableCell
                | NodeType::Run
                | NodeType::Header
                | NodeType::Footer
                | NodeType::CommentBody
                | NodeType::TableOfContents
        )
    }

    /// Returns true if this is a leaf node (no children).
    pub fn is_leaf(&self) -> bool {
        !self.is_container()
    }

    /// Returns true if this is a block-level node.
    pub fn is_block(&self) -> bool {
        matches!(
            self,
            NodeType::Paragraph | NodeType::Table | NodeType::Section | NodeType::TableOfContents
        )
    }

    /// Returns true if this is an inline-level node.
    pub fn is_inline(&self) -> bool {
        matches!(
            self,
            NodeType::Run
                | NodeType::Text
                | NodeType::LineBreak
                | NodeType::PageBreak
                | NodeType::ColumnBreak
                | NodeType::Tab
                | NodeType::Image
                | NodeType::Drawing
                | NodeType::Field
                | NodeType::BookmarkStart
                | NodeType::BookmarkEnd
                | NodeType::CommentStart
                | NodeType::CommentEnd
        )
    }

    /// Returns the allowed child node types for this node type.
    pub fn allowed_children(&self) -> &'static [NodeType] {
        match self {
            NodeType::Document => &[
                NodeType::Body,
                NodeType::Header,
                NodeType::Footer,
                NodeType::CommentBody,
            ],
            NodeType::Body => &[
                NodeType::Section,
                NodeType::Paragraph,
                NodeType::Table,
                NodeType::Image,
                NodeType::TableOfContents,
            ],
            NodeType::Section => &[
                NodeType::Paragraph,
                NodeType::Table,
                NodeType::Image,
                NodeType::TableOfContents,
            ],
            NodeType::TableOfContents => &[NodeType::Paragraph],
            NodeType::Paragraph => &[
                NodeType::Run,
                NodeType::LineBreak,
                NodeType::PageBreak,
                NodeType::ColumnBreak,
                NodeType::Tab,
                NodeType::Image,
                NodeType::Field,
                NodeType::BookmarkStart,
                NodeType::BookmarkEnd,
                NodeType::CommentStart,
                NodeType::CommentEnd,
            ],
            NodeType::Run => &[NodeType::Text],
            NodeType::Table => &[NodeType::TableRow],
            NodeType::TableRow => &[NodeType::TableCell],
            NodeType::TableCell => &[NodeType::Paragraph, NodeType::Table],
            NodeType::Header | NodeType::Footer => &[NodeType::Paragraph, NodeType::Table],
            NodeType::CommentBody => &[NodeType::Paragraph],
            // Leaf nodes
            _ => &[],
        }
    }

    /// Check if `child_type` is allowed as a child of this node type.
    pub fn can_contain(&self, child_type: NodeType) -> bool {
        self.allowed_children().contains(&child_type)
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// A single node in the document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Globally unique identifier (CRDT-ready).
    pub id: NodeId,
    /// The type of this node.
    pub node_type: NodeType,
    /// Formatting and properties.
    pub attributes: AttributeMap,
    /// Ordered child node IDs. Empty for leaf nodes.
    pub children: Vec<NodeId>,
    /// Parent node ID. `None` only for the Document root.
    pub parent: Option<NodeId>,
    /// Text content for `Text` nodes. `None` for all other node types.
    pub text_content: Option<String>,
}

impl Node {
    /// Create a new node with the given ID and type.
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            node_type,
            attributes: AttributeMap::new(),
            children: Vec::new(),
            parent: None,
            text_content: None,
        }
    }

    /// Create a new text node with content.
    pub fn text(id: NodeId, content: impl Into<String>) -> Self {
        Self {
            id,
            node_type: NodeType::Text,
            attributes: AttributeMap::new(),
            children: Vec::new(),
            parent: None,
            text_content: Some(content.into()),
        }
    }

    /// Returns `true` if this node has no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Returns the text content length, or 0 for non-text nodes.
    pub fn text_len(&self) -> usize {
        self.text_content.as_ref().map_or(0, |t| t.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_type_container() {
        assert!(NodeType::Document.is_container());
        assert!(NodeType::Paragraph.is_container());
        assert!(NodeType::Run.is_container());
        assert!(!NodeType::Text.is_leaf().eq(&false)); // Text is leaf
        assert!(NodeType::Text.is_leaf());
        assert!(NodeType::LineBreak.is_leaf());
        assert!(NodeType::Image.is_leaf());
    }

    #[test]
    fn node_type_hierarchy() {
        assert!(NodeType::Document.can_contain(NodeType::Body));
        assert!(!NodeType::Document.can_contain(NodeType::Paragraph));
        assert!(NodeType::Body.can_contain(NodeType::Paragraph));
        assert!(NodeType::Body.can_contain(NodeType::Table));
        assert!(!NodeType::Body.can_contain(NodeType::Run));
        assert!(NodeType::Paragraph.can_contain(NodeType::Run));
        assert!(!NodeType::Paragraph.can_contain(NodeType::Paragraph));
        assert!(NodeType::Run.can_contain(NodeType::Text));
        assert!(!NodeType::Run.can_contain(NodeType::Run));
        assert!(NodeType::Table.can_contain(NodeType::TableRow));
        assert!(NodeType::TableRow.can_contain(NodeType::TableCell));
        assert!(NodeType::TableCell.can_contain(NodeType::Paragraph));
        assert!(NodeType::TableCell.can_contain(NodeType::Table)); // nested tables
    }

    #[test]
    fn create_node() {
        let id = NodeId::new(0, 1);
        let node = Node::new(id, NodeType::Paragraph);
        assert_eq!(node.id, id);
        assert_eq!(node.node_type, NodeType::Paragraph);
        assert!(node.is_empty());
        assert_eq!(node.child_count(), 0);
        assert!(node.parent.is_none());
    }

    #[test]
    fn create_text_node() {
        let id = NodeId::new(0, 5);
        let node = Node::text(id, "Hello world");
        assert_eq!(node.node_type, NodeType::Text);
        assert_eq!(node.text_content.as_deref(), Some("Hello world"));
        assert_eq!(node.text_len(), 11);
    }

    #[test]
    fn non_text_node_has_no_text() {
        let node = Node::new(NodeId::new(0, 1), NodeType::Paragraph);
        assert_eq!(node.text_len(), 0);
        assert!(node.text_content.is_none());
    }
}
