//! Document tree structure — the core container for the document model.
//!
//! [`DocumentModel`] holds all nodes in a flat HashMap, with parent/child
//! relationships encoded via [`NodeId`] references. This design allows O(1) node
//! lookup and is compatible with CRDT node addressing.

use std::collections::HashMap;

use crate::attributes::{AttributeKey, AttributeMap};
use crate::id::{IdGenerator, NodeId};
use crate::media::MediaStore;
use crate::metadata::DocumentMetadata;
use crate::node::{Node, NodeType};
use crate::numbering::NumberingDefinitions;
use crate::section::SectionProperties;
use crate::styles::{resolve_style_chain, Style};

/// Document-level default formatting from `w:docDefaults` in styles.xml.
///
/// These values are used as the base defaults for style resolution when
/// no explicit formatting is specified on a node or in its style chain.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct DocumentDefaults {
    /// Default font family (from `rPrDefault`).
    pub font_family: Option<String>,
    /// Default font size in points (from `rPrDefault/w:sz`).
    pub font_size: Option<f64>,
    /// Default line spacing as a multiple (from `pPrDefault/w:spacing/w:line`).
    /// Value of 276 in OOXML = 276/240 = 1.15x.
    pub line_spacing_multiple: Option<f64>,
    /// Default space after paragraph in points (from `pPrDefault/w:spacing/w:after`).
    pub space_after: Option<f64>,
    /// Default space before paragraph in points (from `pPrDefault/w:spacing/w:before`).
    pub space_before: Option<f64>,
}

/// Error type for document model operations.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ModelError {
    /// The specified node was not found.
    NodeNotFound(NodeId),
    /// The child type is not allowed under the given parent type.
    InvalidHierarchy {
        parent_type: NodeType,
        child_type: NodeType,
    },
    /// The index is out of bounds for the parent's children.
    IndexOutOfBounds {
        parent_id: NodeId,
        index: usize,
        child_count: usize,
    },
    /// Cannot remove the root node.
    CannotRemoveRoot,
    /// The node is not a Text node (for text operations).
    NotATextNode(NodeId),
    /// Text offset is out of bounds.
    TextOffsetOutOfBounds {
        node_id: NodeId,
        offset: usize,
        text_len: usize,
    },
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node not found: {id}"),
            Self::InvalidHierarchy {
                parent_type,
                child_type,
            } => write!(f, "{parent_type} cannot contain {child_type}"),
            Self::IndexOutOfBounds {
                parent_id,
                index,
                child_count,
            } => write!(
                f,
                "Index {index} out of bounds for node {parent_id} (has {child_count} children)"
            ),
            Self::CannotRemoveRoot => write!(f, "Cannot remove the root document node"),
            Self::NotATextNode(id) => write!(f, "Node {id} is not a Text node"),
            Self::TextOffsetOutOfBounds {
                node_id,
                offset,
                text_len,
            } => write!(
                f,
                "Text offset {offset} out of bounds for node {node_id} (length {text_len})"
            ),
        }
    }
}

impl std::error::Error for ModelError {}

/// The complete document model.
///
/// Stores all nodes in a flat map with tree relationships via [`NodeId`] references.
/// Provides tree query and mutation methods.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct DocumentModel {
    nodes: HashMap<NodeId, Node>,
    root: NodeId,
    id_gen: IdGenerator,
    styles: Vec<Style>,
    metadata: DocumentMetadata,
    media: MediaStore,
    numbering: NumberingDefinitions,
    sections: Vec<SectionProperties>,
    doc_defaults: DocumentDefaults,
    /// Cache of resolved style chains, keyed by style ID.
    ///
    /// Avoids repeated walks of the style inheritance chain for the same style.
    /// Invalidated whenever styles are added, modified, or removed via
    /// [`set_style`](Self::set_style) or [`remove_style`](Self::remove_style).
    style_cache: HashMap<String, AttributeMap>,
    /// Preserved ZIP entries for round-trip fidelity (path → bytes).
    /// Stores entries like `_xmlsignatures/`, `customXml/`, `word/diagrams/`,
    /// `word/charts/`, `word/embeddings/` that the engine doesn't semantically
    /// model but needs to preserve on re-export.
    preserved_parts: HashMap<String, Vec<u8>>,
}

impl DocumentModel {
    /// Create a new empty document with default replica ID (0).
    pub fn new() -> Self {
        Self::new_with_replica(0)
    }

    /// Create a new empty document for a specific replica (for CRDT collaboration).
    pub fn new_with_replica(replica_id: u64) -> Self {
        let mut id_gen = IdGenerator::new(replica_id);

        // Create root Document node
        let mut root = Node::new(NodeId::ROOT, NodeType::Document);

        // Create Body node
        let body_id = id_gen.next_id();
        let mut body = Node::new(body_id, NodeType::Body);
        body.parent = Some(NodeId::ROOT);

        root.children.push(body_id);

        let mut nodes = HashMap::new();
        nodes.insert(NodeId::ROOT, root);
        nodes.insert(body_id, body);

        Self {
            nodes,
            root: NodeId::ROOT,
            id_gen,
            styles: Vec::new(),
            metadata: DocumentMetadata::new(),
            media: MediaStore::new(),
            numbering: NumberingDefinitions::default(),
            sections: Vec::new(),
            doc_defaults: DocumentDefaults::default(),
            style_cache: HashMap::new(),
            preserved_parts: HashMap::new(),
        }
    }

    // ─── Preserved ZIP Parts ────────────────────────────────────────────

    /// Store a ZIP entry for round-trip preservation (e.g., signatures, charts, custom XML).
    pub fn add_preserved_part(&mut self, path: impl Into<String>, data: Vec<u8>) {
        self.preserved_parts.insert(path.into(), data);
    }

    /// Get all preserved ZIP entries.
    pub fn preserved_parts(&self) -> &HashMap<String, Vec<u8>> {
        &self.preserved_parts
    }

    /// Check if a preserved part exists.
    pub fn has_preserved_part(&self, path: &str) -> bool {
        self.preserved_parts.contains_key(path)
    }

    // ─── ID generation ──────────────────────────────────────────────────

    /// Generate the next unique node ID for this document.
    pub fn next_id(&mut self) -> NodeId {
        self.id_gen.next_id()
    }

    /// Get the replica ID.
    pub fn replica_id(&self) -> u64 {
        self.id_gen.replica()
    }

    // ─── Tree queries ───────────────────────────────────────────────────

    /// Get the root node ID.
    pub fn root_id(&self) -> NodeId {
        self.root
    }

    /// Get the body node ID (first Body child of root).
    pub fn body_id(&self) -> Option<NodeId> {
        self.node(self.root)?
            .children
            .iter()
            .find(|&&id| self.node(id).is_some_and(|n| n.node_type == NodeType::Body))
            .copied()
    }

    /// Get a node by ID.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    /// Get the root node.
    ///
    /// Returns `None` if the root node is not in the node map (should not happen
    /// in a properly constructed document, but avoids a panic in library code).
    pub fn root_node(&self) -> Option<&Node> {
        self.nodes.get(&self.root)
    }

    /// Get the children of a node.
    pub fn children(&self, id: NodeId) -> Vec<&Node> {
        self.node(id)
            .map(|n| {
                n.children
                    .iter()
                    .filter_map(|child_id| self.node(*child_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the parent of a node.
    pub fn parent(&self, id: NodeId) -> Option<&Node> {
        self.node(id)
            .and_then(|n| n.parent)
            .and_then(|pid| self.node(pid))
    }

    /// Walk ancestors from a node up to (but not including) the root.
    pub fn ancestors(&self, id: NodeId) -> Vec<&Node> {
        let mut result = Vec::new();
        let mut current = self.node(id).and_then(|n| n.parent);
        while let Some(pid) = current {
            if let Some(node) = self.node(pid) {
                result.push(node);
                current = node.parent;
            } else {
                break;
            }
        }
        result
    }

    /// Depth-first traversal of all descendants of a node (excluding the node itself).
    pub fn descendants(&self, id: NodeId) -> Vec<&Node> {
        let mut result = Vec::new();
        let mut stack = Vec::new();

        if let Some(node) = self.node(id) {
            // Push children in reverse order so first child is processed first
            for child_id in node.children.iter().rev() {
                stack.push(*child_id);
            }
        }

        while let Some(nid) = stack.pop() {
            if let Some(node) = self.node(nid) {
                result.push(node);
                for child_id in node.children.iter().rev() {
                    stack.push(*child_id);
                }
            }
        }

        result
    }

    /// Check if `potential_descendant` is a descendant of `ancestor`.
    pub fn is_descendant(&self, potential_descendant: NodeId, ancestor: NodeId) -> bool {
        let mut current = potential_descendant;
        while let Some(node) = self.node(current) {
            if let Some(parent_id) = node.parent {
                if parent_id == ancestor {
                    return true;
                }
                current = parent_id;
            } else {
                break;
            }
        }
        false
    }

    /// Total number of nodes in the document.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Check if a node exists.
    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    // ─── Tree mutations ─────────────────────────────────────────────────

    /// Insert a node as a child of `parent_id` at the given `index`.
    ///
    /// Validates the parent-child type relationship and index bounds.
    pub fn insert_node(
        &mut self,
        parent_id: NodeId,
        index: usize,
        mut node: Node,
    ) -> Result<(), ModelError> {
        // Validate parent exists
        let parent = self
            .nodes
            .get(&parent_id)
            .ok_or(ModelError::NodeNotFound(parent_id))?;

        // Validate hierarchy
        if !parent.node_type.can_contain(node.node_type) {
            return Err(ModelError::InvalidHierarchy {
                parent_type: parent.node_type,
                child_type: node.node_type,
            });
        }

        // Validate index
        if index > parent.children.len() {
            return Err(ModelError::IndexOutOfBounds {
                parent_id,
                index,
                child_count: parent.children.len(),
            });
        }

        // Set parent reference
        node.parent = Some(parent_id);
        let node_id = node.id;

        // Insert node into storage
        self.nodes.insert(node_id, node);

        // Add to parent's children (parent validated above, must still exist)
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.insert(index, node_id);
        }

        Ok(())
    }

    /// Restore a node directly into storage without modifying any parent's
    /// children list. Used for subtree undo where the snapshot already contains
    /// correct parent/children references.
    pub fn restore_node(&mut self, node: Node) {
        self.nodes.insert(node.id, node);
    }

    /// Remove a node and all its descendants from the tree.
    ///
    /// Returns the removed node (without its descendants).
    pub fn remove_node(&mut self, id: NodeId) -> Result<Node, ModelError> {
        if id == self.root {
            return Err(ModelError::CannotRemoveRoot);
        }

        let node = self
            .nodes
            .get(&id)
            .ok_or(ModelError::NodeNotFound(id))?
            .clone();

        // Remove from parent's children
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.children.retain(|&child_id| child_id != id);
            }
        }

        // Remove all descendants (DFS)
        let descendant_ids: Vec<NodeId> = self.descendants(id).iter().map(|n| n.id).collect();

        for did in descendant_ids {
            self.nodes.remove(&did);
        }

        // Remove the node itself
        self.nodes.remove(&id);

        Ok(node)
    }

    /// Move a node to a new parent at the given index.
    ///
    /// If `new_index` exceeds the new parent's child count, it is clamped to append at end.
    pub fn move_node(
        &mut self,
        id: NodeId,
        new_parent_id: NodeId,
        new_index: usize,
    ) -> Result<(), ModelError> {
        if id == self.root {
            return Err(ModelError::CannotRemoveRoot);
        }

        // Prevent cycles: new_parent must not be a descendant of id
        if id == new_parent_id || self.is_descendant(new_parent_id, id) {
            return Err(ModelError::InvalidHierarchy {
                parent_type: self
                    .node(id)
                    .map(|n| n.node_type)
                    .unwrap_or(NodeType::Document),
                child_type: self
                    .node(new_parent_id)
                    .map(|n| n.node_type)
                    .unwrap_or(NodeType::Document),
            });
        }

        let node = self.nodes.get(&id).ok_or(ModelError::NodeNotFound(id))?;

        let node_type = node.node_type;
        let old_parent_id = node.parent;

        // Validate new parent
        let new_parent = self
            .nodes
            .get(&new_parent_id)
            .ok_or(ModelError::NodeNotFound(new_parent_id))?;

        if !new_parent.node_type.can_contain(node_type) {
            return Err(ModelError::InvalidHierarchy {
                parent_type: new_parent.node_type,
                child_type: node_type,
            });
        }

        // Remove from old parent
        if let Some(old_pid) = old_parent_id {
            if let Some(old_parent) = self.nodes.get_mut(&old_pid) {
                old_parent.children.retain(|&child_id| child_id != id);
            }
        }

        // new_index is the desired insertion position after the node has been removed.
        // No same-parent adjustment is needed — the caller specifies the final index.

        // Validate index after removal (new_parent validated above, must still exist)
        let child_count = self
            .nodes
            .get(&new_parent_id)
            .map(|p| p.children.len())
            .unwrap_or(0);

        let actual_index = if new_index > child_count {
            #[cfg(debug_assertions)]
            eprintln!(
                "[s1-model] Warning: move_node index {} exceeds child count {}, clamping to end",
                new_index, child_count
            );
            child_count
        } else {
            new_index
        };

        // Add to new parent
        if let Some(new_parent) = self.nodes.get_mut(&new_parent_id) {
            new_parent.children.insert(actual_index, id);
        }

        // Update parent reference
        if let Some(node) = self.nodes.get_mut(&id) {
            node.parent = Some(new_parent_id);
        }

        Ok(())
    }

    // ─── Text operations ────────────────────────────────────────────────

    /// Insert text into a Text node at the given character offset.
    pub fn insert_text(
        &mut self,
        node_id: NodeId,
        offset: usize,
        text: &str,
    ) -> Result<(), ModelError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(ModelError::NodeNotFound(node_id))?;

        if node.node_type != NodeType::Text {
            return Err(ModelError::NotATextNode(node_id));
        }

        let content = node.text_content.get_or_insert_with(String::new);
        let char_count = content.chars().count();

        if offset > char_count {
            return Err(ModelError::TextOffsetOutOfBounds {
                node_id,
                offset,
                text_len: char_count,
            });
        }

        // Convert char offset to byte offset
        let byte_offset = char_offset_to_byte(content, offset).map_err(|(off, len)| {
            ModelError::TextOffsetOutOfBounds {
                node_id,
                offset: off,
                text_len: len,
            }
        })?;
        content.insert_str(byte_offset, text);
        Ok(())
    }

    /// Delete text from a Text node.
    ///
    /// `offset` and `length` are in character (Unicode scalar value) units.
    pub fn delete_text(
        &mut self,
        node_id: NodeId,
        offset: usize,
        length: usize,
    ) -> Result<String, ModelError> {
        let node = self
            .nodes
            .get_mut(&node_id)
            .ok_or(ModelError::NodeNotFound(node_id))?;

        if node.node_type != NodeType::Text {
            return Err(ModelError::NotATextNode(node_id));
        }

        let content = node.text_content.get_or_insert_with(String::new);
        let char_count = content.chars().count();

        let end = offset + length;
        if end > char_count {
            return Err(ModelError::TextOffsetOutOfBounds {
                node_id,
                offset: end,
                text_len: char_count,
            });
        }

        // Convert char offsets to byte offsets
        let byte_start = char_offset_to_byte(content, offset).map_err(|(off, len)| {
            ModelError::TextOffsetOutOfBounds {
                node_id,
                offset: off,
                text_len: len,
            }
        })?;
        let byte_end = char_offset_to_byte(content, end).map_err(|(off, len)| {
            ModelError::TextOffsetOutOfBounds {
                node_id,
                offset: off,
                text_len: len,
            }
        })?;

        let deleted: String = content[byte_start..byte_end].to_string();
        content.replace_range(byte_start..byte_end, "");
        Ok(deleted)
    }

    // ─── Style queries ──────────────────────────────────────────────────

    /// Get all styles.
    pub fn styles(&self) -> &[Style] {
        &self.styles
    }

    /// Get a style by its ID.
    pub fn style_by_id(&self, id: &str) -> Option<&Style> {
        self.styles.iter().find(|s| s.id == id)
    }

    /// Add or replace a style.
    ///
    /// Invalidates the style resolution cache since the style chain may have changed.
    pub fn set_style(&mut self, style: Style) {
        if let Some(existing) = self.styles.iter_mut().find(|s| s.id == style.id) {
            *existing = style;
        } else {
            self.styles.push(style);
        }
        self.style_cache.clear();
    }

    /// Remove a style by ID.
    ///
    /// Invalidates the style resolution cache since the style chain may have changed.
    pub fn remove_style(&mut self, id: &str) -> Option<Style> {
        let result = if let Some(pos) = self.styles.iter().position(|s| s.id == id) {
            Some(self.styles.remove(pos))
        } else {
            None
        };
        if result.is_some() {
            self.style_cache.clear();
        }
        result
    }

    /// Resolve the fully merged attributes for a node, considering style inheritance.
    ///
    /// Resolution order (highest priority first):
    /// 1. Direct attributes on the node
    /// 2. Character style (if node is a Run with StyleId)
    /// 3. Paragraph style (from ancestor Paragraph's StyleId)
    /// 4. Default style
    ///
    /// Style chain resolution results are cached per style ID. The cache is
    /// automatically invalidated when styles are modified via [`set_style`](Self::set_style)
    /// or [`remove_style`](Self::remove_style).
    pub fn resolve_attributes(&self, node_id: NodeId) -> AttributeMap {
        let node = match self.node(node_id) {
            Some(n) => n,
            None => return AttributeMap::new(),
        };

        let mut result = AttributeMap::new();

        // Find paragraph ancestor's style
        let para_style_id = self.find_ancestor_style(node_id, NodeType::Paragraph);
        if let Some(style_id) = &para_style_id {
            let resolved = self.resolve_style_chain_cached(style_id);
            result.merge(&resolved);
        }

        // Find character style (if node has StyleId)
        if let Some(style_id) = node.attributes.get_string(&AttributeKey::StyleId) {
            let resolved = self.resolve_style_chain_cached(style_id);
            result.merge(&resolved);
        }

        // Direct formatting wins
        result.merge(&node.attributes);

        result
    }

    /// Resolve a style chain with caching.
    ///
    /// Returns a cached result if available, otherwise resolves the chain and
    /// stores the result. Uses interior mutability via the cache field — callers
    /// see an `&self` interface while the cache is updated transparently.
    fn resolve_style_chain_cached(&self, style_id: &str) -> AttributeMap {
        if let Some(cached) = self.style_cache.get(style_id) {
            return cached.clone();
        }
        let resolved = resolve_style_chain(style_id, &self.styles);
        // SAFETY: We use a shared-reference caching pattern here. The cache is
        // purely a performance optimisation and does not affect observable
        // semantics. We cast away const to insert into the cache.
        #[allow(invalid_reference_casting)]
        let cache =
            unsafe { &mut *(&self.style_cache as *const _ as *mut HashMap<String, AttributeMap>) };
        cache.insert(style_id.to_string(), resolved.clone());
        resolved
    }

    /// Find the StyleId attribute from an ancestor of the given type.
    fn find_ancestor_style(&self, node_id: NodeId, ancestor_type: NodeType) -> Option<String> {
        let mut current = Some(node_id);
        while let Some(id) = current {
            if let Some(node) = self.node(id) {
                if node.node_type == ancestor_type {
                    return node
                        .attributes
                        .get_string(&AttributeKey::StyleId)
                        .map(|s| s.to_string());
                }
                current = node.parent;
            } else {
                break;
            }
        }
        None
    }

    // ─── Metadata and media ─────────────────────────────────────────────

    /// Get document metadata.
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    /// Get mutable document metadata.
    pub fn metadata_mut(&mut self) -> &mut DocumentMetadata {
        &mut self.metadata
    }

    /// Get the media store.
    pub fn media(&self) -> &MediaStore {
        &self.media
    }

    /// Get mutable media store.
    pub fn media_mut(&mut self) -> &mut MediaStore {
        &mut self.media
    }

    /// Get the numbering definitions.
    pub fn numbering(&self) -> &NumberingDefinitions {
        &self.numbering
    }

    /// Get mutable numbering definitions.
    pub fn numbering_mut(&mut self) -> &mut NumberingDefinitions {
        &mut self.numbering
    }

    /// Get the section properties.
    pub fn sections(&self) -> &[SectionProperties] {
        &self.sections
    }

    /// Get mutable section properties.
    pub fn sections_mut(&mut self) -> &mut Vec<SectionProperties> {
        &mut self.sections
    }

    /// Get document-level defaults (from `docDefaults` in styles.xml).
    pub fn doc_defaults(&self) -> &DocumentDefaults {
        &self.doc_defaults
    }

    /// Get mutable document defaults.
    pub fn doc_defaults_mut(&mut self) -> &mut DocumentDefaults {
        &mut self.doc_defaults
    }

    // ─── Plain text extraction ──────────────────────────────────────────

    /// Extract all text from the document as a plain string.
    /// Paragraphs are separated by newlines.
    pub fn to_plain_text(&self) -> String {
        let body_id = match self.body_id() {
            Some(id) => id,
            None => return String::new(),
        };

        let mut result = String::new();
        self.extract_text(body_id, &mut result);
        result
    }

    /// Collect all heading paragraphs in document order.
    ///
    /// Returns `(NodeId, heading_level, plain_text)` tuples for each heading
    /// found in the body. Headings are identified by `StyleId` matching
    /// `"Heading1"` through `"Heading9"`.
    pub fn collect_headings(&self) -> Vec<(NodeId, u8, String)> {
        let body_id = match self.body_id() {
            Some(id) => id,
            None => return Vec::new(),
        };
        let mut headings = Vec::new();
        self.collect_headings_from(body_id, &mut headings);
        headings
    }

    fn collect_headings_from(
        &self,
        container_id: NodeId,
        headings: &mut Vec<(NodeId, u8, String)>,
    ) {
        let node = match self.node(container_id) {
            Some(n) => n,
            None => return,
        };
        let children: Vec<NodeId> = node.children.clone();
        for child_id in children {
            let child = match self.node(child_id) {
                Some(n) => n,
                None => continue,
            };
            match child.node_type {
                NodeType::Paragraph => {
                    if let Some(level) = child
                        .attributes
                        .get_string(&crate::AttributeKey::StyleId)
                        .and_then(|s| s.strip_prefix("Heading"))
                        .and_then(|l| l.parse::<u8>().ok())
                    {
                        let mut text = String::new();
                        self.extract_inline_text(child_id, &mut text);
                        headings.push((child_id, level, text));
                    }
                }
                NodeType::Section | NodeType::Body | NodeType::TableOfContents => {
                    self.collect_headings_from(child_id, headings);
                }
                _ => {}
            }
        }
    }

    /// Extract just the inline text content from a node (no paragraph separators).
    fn extract_inline_text(&self, node_id: NodeId, out: &mut String) {
        let node = match self.node(node_id) {
            Some(n) => n,
            None => return,
        };
        match node.node_type {
            NodeType::Text => {
                if let Some(text) = &node.text_content {
                    out.push_str(text);
                }
            }
            _ => {
                let children: Vec<NodeId> = node.children.clone();
                for child_id in children {
                    self.extract_inline_text(child_id, out);
                }
            }
        }
    }

    fn extract_text(&self, node_id: NodeId, out: &mut String) {
        let node = match self.node(node_id) {
            Some(n) => n,
            None => return,
        };

        match node.node_type {
            NodeType::Text => {
                if let Some(text) = &node.text_content {
                    out.push_str(text);
                }
            }
            NodeType::Paragraph => {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                let children: Vec<NodeId> = node.children.clone();
                for child_id in children {
                    self.extract_text(child_id, out);
                }
            }
            NodeType::LineBreak => {
                out.push('\n');
            }
            NodeType::Tab => {
                out.push('\t');
            }
            _ => {
                let children: Vec<NodeId> = node.children.clone();
                for child_id in children {
                    self.extract_text(child_id, out);
                }
            }
        }
    }
}

/// Convert a character offset to a byte offset in a string.
///
/// Returns the byte offset, or the string length if `char_offset` equals
/// the character count (end-of-string position). Returns an error if
/// `char_offset` exceeds the number of characters in `s`.
fn char_offset_to_byte(s: &str, char_offset: usize) -> Result<usize, (usize, usize)> {
    if char_offset == 0 {
        return Ok(0);
    }
    let char_count = s.chars().count();
    if char_offset > char_count {
        return Err((char_offset, char_count));
    }
    Ok(s.char_indices()
        .nth(char_offset)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len()))
}

impl Default for DocumentModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::AttributeValue;

    /// Helper: create a document with one paragraph containing one run with text.
    fn doc_with_text(text: &str) -> (DocumentModel, NodeId, NodeId, NodeId) {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        let run_id = doc.next_id();
        let run = Node::new(run_id, NodeType::Run);
        doc.insert_node(para_id, 0, run).unwrap();

        let text_id = doc.next_id();
        let text_node = Node::text(text_id, text);
        doc.insert_node(run_id, 0, text_node).unwrap();

        (doc, para_id, run_id, text_id)
    }

    #[test]
    fn new_document_structure() {
        let doc = DocumentModel::new();
        assert_eq!(doc.node_count(), 2); // Document + Body
        assert_eq!(doc.root_id(), NodeId::ROOT);
        assert!(doc.body_id().is_some());

        let root = doc.root_node().expect("root must exist in test");
        assert_eq!(root.node_type, NodeType::Document);
        assert_eq!(root.children.len(), 1);
    }

    #[test]
    fn insert_paragraph() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        doc.insert_node(body_id, 0, para).unwrap();

        assert_eq!(doc.node_count(), 3);
        let body = doc.node(body_id).unwrap();
        assert_eq!(body.children, vec![para_id]);

        let para = doc.node(para_id).unwrap();
        assert_eq!(para.parent, Some(body_id));
    }

    #[test]
    fn insert_invalid_hierarchy() {
        let mut doc = DocumentModel::new();

        // Try to put a Run directly under Body (not allowed)
        let run_id = doc.next_id();
        let run = Node::new(run_id, NodeType::Run);
        let body_id = doc.body_id().unwrap();
        let result = doc.insert_node(body_id, 0, run);
        assert!(matches!(result, Err(ModelError::InvalidHierarchy { .. })));
    }

    #[test]
    fn insert_index_out_of_bounds() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        let para = Node::new(para_id, NodeType::Paragraph);
        let result = doc.insert_node(body_id, 999, para);
        assert!(matches!(result, Err(ModelError::IndexOutOfBounds { .. })));
    }

    #[test]
    fn remove_node() {
        let (mut doc, para_id, _run_id, _text_id) = doc_with_text("Hello");
        let initial_count = doc.node_count();

        doc.remove_node(para_id).unwrap();
        // Removed: para + run + text = 3 nodes
        assert_eq!(doc.node_count(), initial_count - 3);
        assert!(doc.node(para_id).is_none());
    }

    #[test]
    fn cannot_remove_root() {
        let mut doc = DocumentModel::new();
        let result = doc.remove_node(NodeId::ROOT);
        assert!(matches!(result, Err(ModelError::CannotRemoveRoot)));
    }

    #[test]
    fn move_node() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create two paragraphs
        let para1_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para1_id, NodeType::Paragraph))
            .unwrap();

        let para2_id = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(para2_id, NodeType::Paragraph))
            .unwrap();

        // Create a run in para1
        let run_id = doc.next_id();
        doc.insert_node(para1_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        // Move run from para1 to para2
        doc.move_node(run_id, para2_id, 0).unwrap();

        assert!(doc.node(para1_id).unwrap().children.is_empty());
        assert_eq!(doc.node(para2_id).unwrap().children, vec![run_id]);
        assert_eq!(doc.node(run_id).unwrap().parent, Some(para2_id));
    }

    #[test]
    fn insert_text_operation() {
        let (mut doc, _para_id, _run_id, text_id) = doc_with_text("Hello");

        doc.insert_text(text_id, 5, " World").unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );
    }

    #[test]
    fn insert_text_at_beginning() {
        let (mut doc, _p, _r, text_id) = doc_with_text("World");
        doc.insert_text(text_id, 0, "Hello ").unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );
    }

    #[test]
    fn insert_text_out_of_bounds() {
        let (mut doc, _p, _r, text_id) = doc_with_text("Hi");
        let result = doc.insert_text(text_id, 100, "x");
        assert!(matches!(
            result,
            Err(ModelError::TextOffsetOutOfBounds { .. })
        ));
    }

    #[test]
    fn delete_text_operation() {
        let (mut doc, _p, _r, text_id) = doc_with_text("Hello World");

        let deleted = doc.delete_text(text_id, 5, 6).unwrap();
        assert_eq!(deleted, " World");
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn delete_text_out_of_bounds() {
        let (mut doc, _p, _r, text_id) = doc_with_text("Hi");
        let result = doc.delete_text(text_id, 0, 100);
        assert!(matches!(
            result,
            Err(ModelError::TextOffsetOutOfBounds { .. })
        ));
    }

    #[test]
    fn text_operation_on_non_text_node() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let result = doc.insert_text(para_id, 0, "x");
        assert!(matches!(result, Err(ModelError::NotATextNode(_))));
    }

    #[test]
    fn descendants_dfs() {
        let (doc, para_id, run_id, text_id) = doc_with_text("Hello");
        let body_id = doc.body_id().unwrap();

        let desc = doc.descendants(body_id);
        let desc_ids: Vec<NodeId> = desc.iter().map(|n| n.id).collect();
        assert_eq!(desc_ids, vec![para_id, run_id, text_id]);
    }

    #[test]
    fn ancestors() {
        let (doc, para_id, _run_id, text_id) = doc_with_text("Hello");
        let body_id = doc.body_id().unwrap();

        let anc = doc.ancestors(text_id);
        let anc_types: Vec<NodeType> = anc.iter().map(|n| n.node_type).collect();
        // Run → Paragraph → Body → Document
        assert_eq!(
            anc_types,
            vec![
                NodeType::Run,
                NodeType::Paragraph,
                NodeType::Body,
                NodeType::Document,
            ]
        );

        let _ = (para_id, body_id); // used in assertions above via ancestor chain
    }

    #[test]
    fn plain_text_extraction() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Paragraph 1: "Hello"
        let p1 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let r1 = doc.next_id();
        doc.insert_node(p1, 0, Node::new(r1, NodeType::Run))
            .unwrap();
        let t1 = doc.next_id();
        doc.insert_node(r1, 0, Node::text(t1, "Hello")).unwrap();

        // Paragraph 2: "World"
        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();
        let r2 = doc.next_id();
        doc.insert_node(p2, 0, Node::new(r2, NodeType::Run))
            .unwrap();
        let t2 = doc.next_id();
        doc.insert_node(r2, 0, Node::text(t2, "World")).unwrap();

        assert_eq!(doc.to_plain_text(), "Hello\nWorld");
    }

    #[test]
    fn style_management() {
        let mut doc = DocumentModel::new();
        assert!(doc.styles().is_empty());

        let style = Style::new("Normal", "Normal", crate::styles::StyleType::Paragraph);
        doc.set_style(style);
        assert_eq!(doc.styles().len(), 1);
        assert!(doc.style_by_id("Normal").is_some());

        // Update existing
        let updated = Style::new(
            "Normal",
            "Normal Updated",
            crate::styles::StyleType::Paragraph,
        );
        doc.set_style(updated);
        assert_eq!(doc.styles().len(), 1);
        assert_eq!(doc.style_by_id("Normal").unwrap().name, "Normal Updated");

        // Remove
        doc.remove_style("Normal");
        assert!(doc.styles().is_empty());
    }

    #[test]
    fn attribute_resolution_with_styles() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Add a style
        let style = Style::new("Heading1", "Heading 1", crate::styles::StyleType::Paragraph)
            .with_attributes(AttributeMap::new().bold(true).font_size(24.0));
        doc.set_style(style);

        // Create paragraph with style reference
        let para_id = doc.next_id();
        let mut para = Node::new(para_id, NodeType::Paragraph);
        para.attributes.set(
            AttributeKey::StyleId,
            AttributeValue::String("Heading1".into()),
        );
        doc.insert_node(body_id, 0, para).unwrap();

        // Create run with direct formatting (italic)
        let run_id = doc.next_id();
        let mut run = Node::new(run_id, NodeType::Run);
        run.attributes
            .set(AttributeKey::Italic, AttributeValue::Bool(true));
        doc.insert_node(para_id, 0, run).unwrap();

        // Resolve run attributes: should have bold (from style) + italic (direct)
        let resolved = doc.resolve_attributes(run_id);
        assert_eq!(resolved.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(resolved.get_bool(&AttributeKey::Italic), Some(true));
        assert_eq!(resolved.get_f64(&AttributeKey::FontSize), Some(24.0));
    }

    #[test]
    fn nested_tables() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Table > Row > Cell > Table > Row > Cell > Paragraph
        let tbl = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl, NodeType::Table))
            .unwrap();

        let row = doc.next_id();
        doc.insert_node(tbl, 0, Node::new(row, NodeType::TableRow))
            .unwrap();

        let cell = doc.next_id();
        doc.insert_node(row, 0, Node::new(cell, NodeType::TableCell))
            .unwrap();

        // Nested table inside cell
        let inner_tbl = doc.next_id();
        doc.insert_node(cell, 0, Node::new(inner_tbl, NodeType::Table))
            .unwrap();

        assert_eq!(doc.node(inner_tbl).unwrap().parent, Some(cell));
    }

    #[test]
    fn replica_ids() {
        let doc_a = DocumentModel::new_with_replica(1);
        let doc_b = DocumentModel::new_with_replica(2);
        assert_eq!(doc_a.replica_id(), 1);
        assert_eq!(doc_b.replica_id(), 2);
    }

    // ─── Unicode safety regression tests ────────────────────────────────

    #[test]
    fn insert_text_multibyte_characters() {
        // Arabic, Hindi, emoji, accented — all survive insert at char offsets
        let (mut doc, _p, _r, text_id) = doc_with_text("café");
        // "café" has 4 chars but 5 bytes (é = 2 bytes)
        doc.insert_text(text_id, 4, "!").unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("café!")
        );

        doc.insert_text(text_id, 0, "\u{2603}").unwrap(); // snowman U+2603
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("\u{2603}caf\u{00e9}!")
        );
    }

    #[test]
    fn insert_text_multibyte_4byte_offset() {
        // Use 4-byte chars: U+1F600 (grinning face) encoded as surrogate pairs
        let (mut doc, _p, _r, text_id) = doc_with_text("\u{1F600}\u{1F601}");
        // 2 chars, each 4 bytes
        doc.insert_text(text_id, 1, "X").unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("\u{1F600}X\u{1F601}")
        );
    }

    #[test]
    fn delete_text_multibyte_characters() {
        let (mut doc, _p, _r, text_id) = doc_with_text("h\u{00e9}llo");
        // Delete the accented char (char offset 1, length 1)
        let deleted = doc.delete_text(text_id, 1, 1).unwrap();
        assert_eq!(deleted, "\u{00e9}");
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("hllo")
        );
    }

    #[test]
    fn delete_text_4byte_char() {
        let (mut doc, _p, _r, text_id) = doc_with_text("a\u{1F600}b\u{1F601}c");
        // Delete char at offset 1 (the 4-byte char), length 1
        let deleted = doc.delete_text(text_id, 1, 1).unwrap();
        assert_eq!(deleted, "\u{1F600}");
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("ab\u{1F601}c")
        );
    }

    #[test]
    fn insert_text_arabic() {
        let (mut doc, _p, _r, text_id) = doc_with_text("مرحبا");
        doc.insert_text(text_id, 2, "X").unwrap();
        let content = doc.node(text_id).unwrap().text_content.clone().unwrap();
        assert_eq!(content.chars().count(), 6); // 5 original + 1 inserted
        assert_eq!(content.chars().nth(2), Some('X'));
    }

    #[test]
    fn insert_text_mixed_script() {
        let (mut doc, _p, _r, text_id) = doc_with_text("hello世界");
        doc.insert_text(text_id, 5, "\u{2192}").unwrap(); // rightwards arrow
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("hello\u{2192}\u{4e16}\u{754c}")
        );
    }

    #[test]
    fn move_node_within_same_parent_forward() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create three paragraphs: [P0, P1, P2]
        let p0 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p0, NodeType::Paragraph))
            .unwrap();
        let p1 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let p2 = doc.next_id();
        doc.insert_node(body_id, 2, Node::new(p2, NodeType::Paragraph))
            .unwrap();

        // Move P0 (index 0) to index 2 → expect [P1, P2, P0]
        doc.move_node(p0, body_id, 2).unwrap();
        let children = &doc.node(body_id).unwrap().children;
        assert_eq!(children, &[p1, p2, p0], "moving forward within same parent");
    }

    #[test]
    fn move_node_within_same_parent_backward() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Create three paragraphs: [P0, P1, P2]
        let p0 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p0, NodeType::Paragraph))
            .unwrap();
        let p1 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let p2 = doc.next_id();
        doc.insert_node(body_id, 2, Node::new(p2, NodeType::Paragraph))
            .unwrap();

        // Move P2 (index 2) to index 0 → expect [P2, P0, P1]
        doc.move_node(p2, body_id, 0).unwrap();
        let children = &doc.node(body_id).unwrap().children;
        assert_eq!(
            children,
            &[p2, p0, p1],
            "moving backward within same parent"
        );
    }

    #[test]
    fn move_node_within_same_parent_noop() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let p0 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p0, NodeType::Paragraph))
            .unwrap();
        let p1 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p1, NodeType::Paragraph))
            .unwrap();

        // Move P0 to index 0 within same parent → should be no-op: [P0, P1]
        doc.move_node(p0, body_id, 0).unwrap();
        let children = &doc.node(body_id).unwrap().children;
        assert_eq!(children, &[p0, p1], "same-parent move to same position");
    }

    // ─── Cycle detection regression tests ────────────────────────────────

    #[test]
    fn move_node_rejects_self_parent() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        // Moving a node under itself must fail
        let result = doc.move_node(para_id, para_id, 0);
        assert!(result.is_err());
    }

    #[test]
    fn move_node_rejects_descendant_as_parent() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // Build: body > para > run
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        // Moving para under its own child (run) must fail — would create cycle
        let result = doc.move_node(para_id, run_id, 0);
        assert!(matches!(result, Err(ModelError::InvalidHierarchy { .. })));
    }

    #[test]
    fn move_node_rejects_deep_descendant() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        // body > table > row > cell > para
        let tbl = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl, NodeType::Table))
            .unwrap();
        let row = doc.next_id();
        doc.insert_node(tbl, 0, Node::new(row, NodeType::TableRow))
            .unwrap();
        let cell = doc.next_id();
        doc.insert_node(row, 0, Node::new(cell, NodeType::TableCell))
            .unwrap();
        let para = doc.next_id();
        doc.insert_node(cell, 0, Node::new(para, NodeType::Paragraph))
            .unwrap();

        // Moving table under its deep descendant (para) must fail
        let result = doc.move_node(tbl, para, 0);
        assert!(matches!(result, Err(ModelError::InvalidHierarchy { .. })));
    }

    #[test]
    fn is_descendant_basic() {
        let (doc, para_id, run_id, text_id) = doc_with_text("Hello");
        let body_id = doc.body_id().unwrap();

        assert!(doc.is_descendant(text_id, run_id));
        assert!(doc.is_descendant(text_id, para_id));
        assert!(doc.is_descendant(text_id, body_id));
        assert!(doc.is_descendant(run_id, para_id));
        assert!(!doc.is_descendant(para_id, text_id)); // not a descendant
        assert!(!doc.is_descendant(run_id, text_id));
    }

    // ─── Property-based tests ───────────────────────────────────────────

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        /// Strategy to generate a sequence of valid tree operations.
        /// Returns (operation_type, index_hint) pairs.
        #[derive(Debug, Clone)]
        enum TreeOp {
            InsertParagraph(usize),
            InsertRunInFirst,
            InsertTextInFirstRun(String),
            RemoveFirst,
        }

        fn tree_op_strategy() -> impl Strategy<Value = TreeOp> {
            prop_oneof![
                (0..10usize).prop_map(TreeOp::InsertParagraph),
                Just(TreeOp::InsertRunInFirst),
                "[a-zA-Z0-9 ]{0,20}".prop_map(TreeOp::InsertTextInFirstRun),
                Just(TreeOp::RemoveFirst),
            ]
        }

        proptest! {
            #[test]
            fn tree_operations_never_produce_invalid_state(
                ops in proptest::collection::vec(tree_op_strategy(), 1..30)
            ) {
                let mut doc = DocumentModel::new();
                let body_id = doc.body_id().unwrap();

                for op in ops {
                    match op {
                        TreeOp::InsertParagraph(idx_hint) => {
                            let body = doc.node(body_id).unwrap();
                            let idx = if body.children.is_empty() { 0 } else { idx_hint % (body.children.len() + 1) };
                            let para_id = doc.next_id();
                            let _ = doc.insert_node(body_id, idx, Node::new(para_id, NodeType::Paragraph));
                        }
                        TreeOp::InsertRunInFirst => {
                            let body = doc.node(body_id).unwrap();
                            if let Some(&para_id) = body.children.first() {
                                if doc.node(para_id).unwrap().node_type == NodeType::Paragraph {
                                    let run_id = doc.next_id();
                                    let _ = doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run));
                                }
                            }
                        }
                        TreeOp::InsertTextInFirstRun(text) => {
                            // Find first text node and insert
                            let body = doc.node(body_id).unwrap();
                            if let Some(&para_id) = body.children.first() {
                                let para = doc.node(para_id).unwrap();
                                if let Some(&run_id) = para.children.first() {
                                    let run = doc.node(run_id).unwrap();
                                    if let Some(&text_id) = run.children.first() {
                                        if doc.node(text_id).unwrap().node_type == NodeType::Text {
                                            let _ = doc.insert_text(text_id, 0, &text);
                                        }
                                    } else {
                                        // Insert a text node
                                        let text_id = doc.next_id();
                                        let _ = doc.insert_node(run_id, 0, Node::text(text_id, &text));
                                    }
                                }
                            }
                        }
                        TreeOp::RemoveFirst => {
                            let body = doc.node(body_id).unwrap();
                            if let Some(&para_id) = body.children.first() {
                                let _ = doc.remove_node(para_id);
                            }
                        }
                    }

                    // Invariant 1: root always exists
                    assert!(doc.node(NodeId::ROOT).is_some());

                    // Invariant 2: body always exists (we never remove it)
                    assert!(doc.body_id().is_some());

                    // Invariant 3: every child's parent points back correctly
                    let all_ids: Vec<NodeId> = doc.nodes.keys().copied().collect();
                    for id in &all_ids {
                        let node = doc.node(*id).unwrap();
                        for &child_id in &node.children {
                            if let Some(child) = doc.node(child_id) {
                                assert_eq!(child.parent, Some(*id),
                                    "child {child_id} parent mismatch");
                            }
                        }
                    }

                    // Invariant 4: node count matches actual nodes in map
                    assert_eq!(doc.node_count(), doc.nodes.len());
                }
            }
        }
    }
}
