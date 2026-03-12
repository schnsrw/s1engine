//! Operation types — every document mutation is an `Operation`.
//!
//! Operations are the atomic unit of change. They are applied to the document model,
//! produce an inverse (for undo), and can be serialized for collaboration.

use s1_model::{
    AttributeKey, AttributeMap, DocumentModel, ModelError, Node, NodeId, NodeType, Style,
};

/// Every possible mutation to the document model.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    /// Insert a new node as child of `parent_id` at `index`.
    InsertNode {
        parent_id: NodeId,
        index: usize,
        node: Node,
    },

    /// Delete a node and all its descendants.
    DeleteNode {
        /// The node to delete.
        target_id: NodeId,
        /// Stored on apply for undo: the deleted node's parent.
        parent_id: Option<NodeId>,
        /// Stored on apply for undo: the deleted node's index in parent.
        index: Option<usize>,
        /// Stored on apply for undo: the full deleted subtree snapshot.
        snapshot: Option<Vec<Node>>,
    },

    /// Move a node to a new parent at a given index.
    MoveNode {
        target_id: NodeId,
        new_parent_id: NodeId,
        new_index: usize,
        /// Stored on apply for undo: old parent.
        old_parent_id: Option<NodeId>,
        /// Stored on apply for undo: old index.
        old_index: Option<usize>,
    },

    /// Insert text into a Text node at a character offset.
    InsertText {
        target_id: NodeId,
        offset: usize,
        text: String,
    },

    /// Delete text from a Text node.
    DeleteText {
        target_id: NodeId,
        offset: usize,
        length: usize,
        /// Stored on apply for undo: the deleted text.
        deleted_text: Option<String>,
    },

    /// Set attributes on a node (merge with existing).
    SetAttributes {
        target_id: NodeId,
        attributes: AttributeMap,
        /// Stored on apply for undo: the previous values of changed keys.
        previous: Option<AttributeMap>,
    },

    /// Remove specific attributes from a node.
    RemoveAttributes {
        target_id: NodeId,
        keys: Vec<AttributeKey>,
        /// Stored on apply for undo: the removed key-value pairs.
        removed: Option<AttributeMap>,
    },

    /// Set document-level metadata.
    SetMetadata {
        key: String,
        value: Option<String>,
        /// Stored on apply for undo: old value.
        old_value: Option<Option<String>>,
    },

    /// Add or update a style definition.
    SetStyle {
        style: Style,
        /// Stored on apply for undo: the previous style (if replacing).
        old_style: Option<Option<Style>>,
    },

    /// Remove a style definition.
    RemoveStyle {
        style_id: String,
        /// Stored on apply for undo: the removed style.
        removed_style: Option<Style>,
    },
}

impl Operation {
    // ─── Convenience constructors (without undo fields) ─────────────────

    pub fn insert_node(parent_id: NodeId, index: usize, node: Node) -> Self {
        Self::InsertNode {
            parent_id,
            index,
            node,
        }
    }

    pub fn delete_node(target_id: NodeId) -> Self {
        Self::DeleteNode {
            target_id,
            parent_id: None,
            index: None,
            snapshot: None,
        }
    }

    pub fn move_node(target_id: NodeId, new_parent_id: NodeId, new_index: usize) -> Self {
        Self::MoveNode {
            target_id,
            new_parent_id,
            new_index,
            old_parent_id: None,
            old_index: None,
        }
    }

    pub fn insert_text(target_id: NodeId, offset: usize, text: impl Into<String>) -> Self {
        Self::InsertText {
            target_id,
            offset,
            text: text.into(),
        }
    }

    pub fn delete_text(target_id: NodeId, offset: usize, length: usize) -> Self {
        Self::DeleteText {
            target_id,
            offset,
            length,
            deleted_text: None,
        }
    }

    pub fn set_attributes(target_id: NodeId, attributes: AttributeMap) -> Self {
        Self::SetAttributes {
            target_id,
            attributes,
            previous: None,
        }
    }

    pub fn remove_attributes(target_id: NodeId, keys: Vec<AttributeKey>) -> Self {
        Self::RemoveAttributes {
            target_id,
            keys,
            removed: None,
        }
    }

    pub fn set_metadata(key: impl Into<String>, value: Option<String>) -> Self {
        Self::SetMetadata {
            key: key.into(),
            value,
            old_value: None,
        }
    }

    pub fn set_style(style: Style) -> Self {
        Self::SetStyle {
            style,
            old_style: None,
        }
    }

    pub fn remove_style(style_id: impl Into<String>) -> Self {
        Self::RemoveStyle {
            style_id: style_id.into(),
            removed_style: None,
        }
    }
}

/// Apply an operation to the document model.
///
/// Returns the **inverse operation** that will undo this change.
pub fn apply(model: &mut DocumentModel, op: &Operation) -> Result<Operation, OperationError> {
    match op {
        Operation::InsertNode {
            parent_id,
            index,
            node,
        } => {
            model
                .insert_node(*parent_id, *index, node.clone())
                .map_err(OperationError::Model)?;

            Ok(Operation::DeleteNode {
                target_id: node.id,
                parent_id: Some(*parent_id),
                index: Some(*index),
                snapshot: None, // inverse doesn't need snapshot — just delete
            })
        }

        Operation::DeleteNode {
            target_id,
            parent_id: stored_parent,
            index: stored_index,
            snapshot,
        } => {
            // If this is an "undo" (snapshot + parent + index set), re-insert the subtree
            if let (Some(parent_id), Some(index), Some(snap)) =
                (stored_parent, stored_index, snapshot)
            {
                // Re-insert root node under the original parent via model API
                // (sets parent ref, adds to parent's children list)
                let mut root_node = snap[0].clone();
                // Clear children — we'll restore them via direct insertion below
                root_node.children.clear();
                model
                    .insert_node(*parent_id, *index, root_node)
                    .map_err(OperationError::Model)?;

                // Re-insert all descendants directly into model storage.
                // The snapshot preserves each node's parent and children refs,
                // so we restore them as-is without going through insert_node
                // (which would duplicate children list entries).
                for desc in snap.iter().skip(1) {
                    model.restore_node(desc.clone());
                }

                // Restore the root's original children list from the snapshot
                if let Some(root_snap) = snap.first() {
                    if let Some(root) = model.node_mut(*target_id) {
                        root.children = root_snap.children.clone();
                    }
                }

                // Inverse of re-insert is delete again
                return Ok(Operation::DeleteNode {
                    target_id: *target_id,
                    parent_id: None,
                    index: None,
                    snapshot: None,
                });
            }

            // Normal delete: snapshot the subtree and remove it
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            let parent_id = node.parent.ok_or(OperationError::CannotDeleteRoot)?;

            let parent = model
                .node(parent_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(parent_id)))?;

            let index = parent
                .children
                .iter()
                .position(|&id| id == *target_id)
                .unwrap_or(0);

            // Snapshot the subtree for undo (root + all descendants in DFS order)
            let mut snap = Vec::new();
            snap.push(node.clone());
            for desc in model.descendants(*target_id) {
                snap.push(desc.clone());
            }

            model
                .remove_node(*target_id)
                .map_err(OperationError::Model)?;

            // Inverse: a DeleteNode with snapshot that will re-insert subtree
            Ok(Operation::DeleteNode {
                target_id: *target_id,
                parent_id: Some(parent_id),
                index: Some(index),
                snapshot: Some(snap),
            })
        }

        Operation::MoveNode {
            target_id,
            new_parent_id,
            new_index,
            ..
        } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            let old_parent_id = node.parent.ok_or(OperationError::CannotDeleteRoot)?;

            let old_parent = model.node(old_parent_id).ok_or(OperationError::Model(
                ModelError::NodeNotFound(old_parent_id),
            ))?;

            let old_index = old_parent
                .children
                .iter()
                .position(|&id| id == *target_id)
                .unwrap_or(0);

            model
                .move_node(*target_id, *new_parent_id, *new_index)
                .map_err(OperationError::Model)?;

            Ok(Operation::MoveNode {
                target_id: *target_id,
                new_parent_id: old_parent_id,
                new_index: old_index,
                old_parent_id: Some(*new_parent_id),
                old_index: Some(*new_index),
            })
        }

        Operation::InsertText {
            target_id,
            offset,
            text,
        } => {
            model
                .insert_text(*target_id, *offset, text)
                .map_err(OperationError::Model)?;

            Ok(Operation::DeleteText {
                target_id: *target_id,
                offset: *offset,
                length: text.chars().count(),
                deleted_text: Some(text.clone()),
            })
        }

        Operation::DeleteText {
            target_id,
            offset,
            length,
            ..
        } => {
            let deleted = model
                .delete_text(*target_id, *offset, *length)
                .map_err(OperationError::Model)?;

            Ok(Operation::InsertText {
                target_id: *target_id,
                offset: *offset,
                text: deleted,
            })
        }

        Operation::SetAttributes {
            target_id,
            attributes,
            previous,
        } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            // If this is an undo (previous is set), restore old values and remove added keys
            if let Some(prev) = previous {
                let node = model.node_mut(*target_id).unwrap();
                // First, remove all keys that were set in the original operation
                for (key, _) in attributes.iter() {
                    node.attributes.remove(key);
                }
                // Then restore previous values (keys that existed before the original op)
                node.attributes.merge(prev);

                // Inverse of undo is the original operation
                return Ok(Operation::SetAttributes {
                    target_id: *target_id,
                    attributes: attributes.clone(),
                    previous: None,
                });
            }

            // Normal forward apply: capture previous values for undo
            let mut prev_values = AttributeMap::new();
            for (key, _) in attributes.iter() {
                if let Some(old_val) = node.attributes.get(key) {
                    prev_values.set(key.clone(), old_val.clone());
                }
            }

            let node = model.node_mut(*target_id).unwrap();
            node.attributes.merge(attributes);

            // Inverse: a SetAttributes with `previous` set for complete undo
            Ok(Operation::SetAttributes {
                target_id: *target_id,
                attributes: attributes.clone(),
                previous: Some(prev_values),
            })
        }

        Operation::RemoveAttributes {
            target_id, keys, ..
        } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            // Capture removed values for undo
            let mut removed = AttributeMap::new();
            for key in keys {
                if let Some(val) = node.attributes.get(key) {
                    removed.set(key.clone(), val.clone());
                }
            }

            let node = model.node_mut(*target_id).unwrap();
            for key in keys {
                node.attributes.remove(key);
            }

            Ok(Operation::SetAttributes {
                target_id: *target_id,
                attributes: removed,
                previous: None,
            })
        }

        Operation::SetMetadata { key, value, .. } => {
            let meta = model.metadata();
            let old_value = match key.as_str() {
                "title" => meta.title.clone(),
                "subject" => meta.subject.clone(),
                "creator" => meta.creator.clone(),
                "description" => meta.description.clone(),
                "language" => meta.language.clone(),
                _ => meta.custom_properties.get(key).cloned(),
            };

            let meta = model.metadata_mut();
            match key.as_str() {
                "title" => meta.title = value.clone(),
                "subject" => meta.subject = value.clone(),
                "creator" => meta.creator = value.clone(),
                "description" => meta.description = value.clone(),
                "language" => meta.language = value.clone(),
                _ => {
                    if let Some(v) = value {
                        meta.custom_properties.insert(key.clone(), v.clone());
                    } else {
                        meta.custom_properties.remove(key);
                    }
                }
            }

            Ok(Operation::SetMetadata {
                key: key.clone(),
                value: old_value.clone(),
                old_value: Some(value.clone()),
            })
        }

        Operation::SetStyle { style, .. } => {
            let old_style = model.style_by_id(&style.id).cloned();
            model.set_style(style.clone());

            match old_style {
                Some(old) => Ok(Operation::SetStyle {
                    style: old,
                    old_style: Some(Some(style.clone())),
                }),
                None => Ok(Operation::RemoveStyle {
                    style_id: style.id.clone(),
                    removed_style: None,
                }),
            }
        }

        Operation::RemoveStyle { style_id, .. } => {
            let removed = model.remove_style(style_id);

            match removed {
                Some(style) => Ok(Operation::SetStyle {
                    style,
                    old_style: None,
                }),
                None => Err(OperationError::StyleNotFound(style_id.clone())),
            }
        }
    }
}

/// Validate an operation without applying it.
pub fn validate(model: &DocumentModel, op: &Operation) -> Result<(), OperationError> {
    match op {
        Operation::InsertNode {
            parent_id,
            index,
            node,
        } => {
            let parent = model
                .node(*parent_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*parent_id)))?;

            if !parent.node_type.can_contain(node.node_type) {
                return Err(OperationError::Model(ModelError::InvalidHierarchy {
                    parent_type: parent.node_type,
                    child_type: node.node_type,
                }));
            }

            if *index > parent.children.len() {
                return Err(OperationError::Model(ModelError::IndexOutOfBounds {
                    parent_id: *parent_id,
                    index: *index,
                    child_count: parent.children.len(),
                }));
            }

            Ok(())
        }

        Operation::DeleteNode { target_id, .. } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            if node.parent.is_none() {
                return Err(OperationError::CannotDeleteRoot);
            }

            Ok(())
        }

        Operation::MoveNode {
            target_id,
            new_parent_id,
            ..
        } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            let new_parent = model.node(*new_parent_id).ok_or(OperationError::Model(
                ModelError::NodeNotFound(*new_parent_id),
            ))?;

            if !new_parent.node_type.can_contain(node.node_type) {
                return Err(OperationError::Model(ModelError::InvalidHierarchy {
                    parent_type: new_parent.node_type,
                    child_type: node.node_type,
                }));
            }

            Ok(())
        }

        Operation::InsertText {
            target_id, offset, ..
        } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            if node.node_type != NodeType::Text {
                return Err(OperationError::Model(ModelError::NotATextNode(*target_id)));
            }

            let text_len = node.text_content.as_ref().map_or(0, |t| t.len());
            if *offset > text_len {
                return Err(OperationError::Model(ModelError::TextOffsetOutOfBounds {
                    node_id: *target_id,
                    offset: *offset,
                    text_len,
                }));
            }

            Ok(())
        }

        Operation::DeleteText {
            target_id,
            offset,
            length,
            ..
        } => {
            let node = model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;

            if node.node_type != NodeType::Text {
                return Err(OperationError::Model(ModelError::NotATextNode(*target_id)));
            }

            let text_len = node.text_content.as_ref().map_or(0, |t| t.len());
            if offset + length > text_len {
                return Err(OperationError::Model(ModelError::TextOffsetOutOfBounds {
                    node_id: *target_id,
                    offset: offset + length,
                    text_len,
                }));
            }

            Ok(())
        }

        Operation::SetAttributes { target_id, .. }
        | Operation::RemoveAttributes { target_id, .. } => {
            model
                .node(*target_id)
                .ok_or(OperationError::Model(ModelError::NodeNotFound(*target_id)))?;
            Ok(())
        }

        Operation::SetMetadata { .. } => Ok(()),

        Operation::SetStyle { .. } => Ok(()),

        Operation::RemoveStyle { style_id, .. } => {
            if model.style_by_id(style_id).is_none() {
                return Err(OperationError::StyleNotFound(style_id.clone()));
            }
            Ok(())
        }
    }
}

/// Error from applying or validating an operation.
#[derive(Debug, Clone, PartialEq)]
pub enum OperationError {
    /// Error from the document model layer.
    Model(ModelError),
    /// Cannot delete or move the root node.
    CannotDeleteRoot,
    /// Style not found.
    StyleNotFound(String),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Model(e) => write!(f, "{e}"),
            Self::CannotDeleteRoot => write!(f, "Cannot delete the root node"),
            Self::StyleNotFound(id) => write!(f, "Style not found: {id}"),
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{AttributeKey, StyleType};

    /// Helper: create a doc with body > paragraph > run > text
    fn setup_doc(text: &str) -> (DocumentModel, NodeId, NodeId, NodeId, NodeId) {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, text))
            .unwrap();

        (doc, body_id, para_id, run_id, text_id)
    }

    // ─── InsertNode ─────────────────────────────────────────────────────

    #[test]
    fn op_insert_node() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();
        let node = Node::new(para_id, NodeType::Paragraph);

        let inverse = apply(&mut doc, &Operation::insert_node(body_id, 0, node)).unwrap();
        assert!(doc.node(para_id).is_some());
        assert_eq!(doc.node(body_id).unwrap().children.len(), 1);

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert!(doc.node(para_id).is_none());
    }

    #[test]
    fn op_insert_invalid_hierarchy() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let run_id = doc.next_id();
        let node = Node::new(run_id, NodeType::Run);

        let result = apply(&mut doc, &Operation::insert_node(body_id, 0, node));
        assert!(result.is_err());
    }

    // ─── DeleteNode ─────────────────────────────────────────────────────

    #[test]
    fn op_delete_node() {
        let (mut doc, _body_id, para_id, run_id, text_id) = setup_doc("Hello");

        let inverse = apply(&mut doc, &Operation::delete_node(para_id)).unwrap();
        assert!(doc.node(para_id).is_none());
        assert!(doc.node(run_id).is_none());
        assert!(doc.node(text_id).is_none());

        // Undo: re-inserts the paragraph (but not deep children for this simplified inverse)
        let result = apply(&mut doc, &inverse);
        assert!(result.is_ok());
    }

    // ─── MoveNode ───────────────────────────────────────────────────────

    #[test]
    fn op_move_node() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let p1 = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(p1, NodeType::Paragraph))
            .unwrap();
        let p2 = doc.next_id();
        doc.insert_node(body_id, 1, Node::new(p2, NodeType::Paragraph))
            .unwrap();

        let run_id = doc.next_id();
        doc.insert_node(p1, 0, Node::new(run_id, NodeType::Run))
            .unwrap();

        // Move run from p1 to p2
        let inverse = apply(&mut doc, &Operation::move_node(run_id, p2, 0)).unwrap();
        assert_eq!(doc.node(run_id).unwrap().parent, Some(p2));
        assert!(doc.node(p1).unwrap().children.is_empty());
        assert_eq!(doc.node(p2).unwrap().children, vec![run_id]);

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.node(run_id).unwrap().parent, Some(p1));
    }

    // ─── InsertText ─────────────────────────────────────────────────────

    #[test]
    fn op_insert_text() {
        let (mut doc, _, _, _, text_id) = setup_doc("Hello");

        let inverse = apply(&mut doc, &Operation::insert_text(text_id, 5, " World")).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn op_insert_text_at_beginning() {
        let (mut doc, _, _, _, text_id) = setup_doc("World");

        apply(&mut doc, &Operation::insert_text(text_id, 0, "Hello ")).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );
    }

    // ─── DeleteText ─────────────────────────────────────────────────────

    #[test]
    fn op_delete_text() {
        let (mut doc, _, _, _, text_id) = setup_doc("Hello World");

        let inverse = apply(&mut doc, &Operation::delete_text(text_id, 5, 6)).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello")
        );

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello World")
        );
    }

    // ─── SetAttributes ──────────────────────────────────────────────────

    #[test]
    fn op_set_attributes() {
        let (mut doc, _, _, run_id, _) = setup_doc("Hello");

        let attrs = AttributeMap::new().bold(true).font_size(16.0);
        let inverse = apply(&mut doc, &Operation::set_attributes(run_id, attrs)).unwrap();

        let node = doc.node(run_id).unwrap();
        assert_eq!(node.attributes.get_bool(&AttributeKey::Bold), Some(true));
        assert_eq!(node.attributes.get_f64(&AttributeKey::FontSize), Some(16.0));

        // Undo: newly added attributes are removed
        apply(&mut doc, &inverse).unwrap();
        let node = doc.node(run_id).unwrap();
        assert!(!node.attributes.contains(&AttributeKey::Bold));
    }

    #[test]
    fn op_set_attributes_overwrite() {
        let (mut doc, _, _, run_id, _) = setup_doc("Hello");

        // Set initial
        apply(
            &mut doc,
            &Operation::set_attributes(run_id, AttributeMap::new().font_size(12.0)),
        )
        .unwrap();

        // Overwrite
        let inverse = apply(
            &mut doc,
            &Operation::set_attributes(run_id, AttributeMap::new().font_size(24.0)),
        )
        .unwrap();

        assert_eq!(
            doc.node(run_id)
                .unwrap()
                .attributes
                .get_f64(&AttributeKey::FontSize),
            Some(24.0)
        );

        // Undo restores old value
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(
            doc.node(run_id)
                .unwrap()
                .attributes
                .get_f64(&AttributeKey::FontSize),
            Some(12.0)
        );
    }

    // ─── RemoveAttributes ───────────────────────────────────────────────

    #[test]
    fn op_remove_attributes() {
        let (mut doc, _, _, run_id, _) = setup_doc("Hello");

        // Set some attributes first
        apply(
            &mut doc,
            &Operation::set_attributes(run_id, AttributeMap::new().bold(true).italic(true)),
        )
        .unwrap();

        // Remove bold
        let inverse = apply(
            &mut doc,
            &Operation::remove_attributes(run_id, vec![AttributeKey::Bold]),
        )
        .unwrap();

        let node = doc.node(run_id).unwrap();
        assert!(!node.attributes.contains(&AttributeKey::Bold));
        assert!(node.attributes.contains(&AttributeKey::Italic));

        // Undo: bold is restored
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(
            doc.node(run_id)
                .unwrap()
                .attributes
                .get_bool(&AttributeKey::Bold),
            Some(true)
        );
    }

    // ─── SetMetadata ────────────────────────────────────────────────────

    #[test]
    fn op_set_metadata() {
        let (mut doc, ..) = setup_doc("Hello");

        let inverse = apply(
            &mut doc,
            &Operation::set_metadata("title", Some("My Doc".into())),
        )
        .unwrap();

        assert_eq!(doc.metadata().title.as_deref(), Some("My Doc"));

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert!(doc.metadata().title.is_none());
    }

    // ─── SetStyle / RemoveStyle ─────────────────────────────────────────

    #[test]
    fn op_set_style() {
        let (mut doc, ..) = setup_doc("Hello");
        let style = Style::new("Heading1", "Heading 1", StyleType::Paragraph);

        let inverse = apply(&mut doc, &Operation::set_style(style)).unwrap();
        assert!(doc.style_by_id("Heading1").is_some());

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert!(doc.style_by_id("Heading1").is_none());
    }

    #[test]
    fn op_remove_style() {
        let (mut doc, ..) = setup_doc("Hello");
        let style = Style::new("Heading1", "Heading 1", StyleType::Paragraph);
        doc.set_style(style);

        let inverse = apply(&mut doc, &Operation::remove_style("Heading1")).unwrap();
        assert!(doc.style_by_id("Heading1").is_none());

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert!(doc.style_by_id("Heading1").is_some());
    }

    // ─── Validation ─────────────────────────────────────────────────────

    #[test]
    fn validate_insert_valid() {
        let doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para = Node::new(NodeId::new(0, 99), NodeType::Paragraph);
        assert!(validate(&doc, &Operation::insert_node(body_id, 0, para)).is_ok());
    }

    #[test]
    fn validate_insert_invalid_parent() {
        let doc = DocumentModel::new();
        let para = Node::new(NodeId::new(0, 99), NodeType::Paragraph);
        let result = validate(&doc, &Operation::insert_node(NodeId::new(0, 999), 0, para));
        assert!(result.is_err());
    }

    #[test]
    fn validate_delete_nonexistent() {
        let doc = DocumentModel::new();
        let result = validate(&doc, &Operation::delete_node(NodeId::new(0, 999)));
        assert!(result.is_err());
    }

    #[test]
    fn validate_text_op_on_non_text() {
        let (doc, _, para_id, _, _) = setup_doc("Hello");
        let result = validate(&doc, &Operation::insert_text(para_id, 0, "x"));
        assert!(result.is_err());
    }

    // ─── Inverse round-trip ─────────────────────────────────────────────

    #[test]
    fn inverse_roundtrip_insert_text() {
        let (mut doc, _, _, _, text_id) = setup_doc("Hello");
        let original = doc.to_plain_text();

        let inverse = apply(&mut doc, &Operation::insert_text(text_id, 5, " World")).unwrap();
        assert_ne!(doc.to_plain_text(), original);

        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.to_plain_text(), original);
    }

    #[test]
    fn inverse_roundtrip_delete_text() {
        let (mut doc, _, _, _, text_id) = setup_doc("Hello World");
        let original = doc.to_plain_text();

        let inverse = apply(&mut doc, &Operation::delete_text(text_id, 5, 6)).unwrap();
        assert_ne!(doc.to_plain_text(), original);

        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.to_plain_text(), original);
    }

    // ─── P0 Regression: Subtree undo ───────────────────────────────────

    #[test]
    fn subtree_undo_restores_entire_tree() {
        // body > para > run > text("Hello")
        let (mut doc, body_id, para_id, run_id, text_id) = setup_doc("Hello");
        let initial_count = doc.node_count();

        // Delete the paragraph (takes run + text with it)
        let inverse = apply(&mut doc, &Operation::delete_node(para_id)).unwrap();
        assert!(doc.node(para_id).is_none());
        assert!(doc.node(run_id).is_none());
        assert!(doc.node(text_id).is_none());
        assert_eq!(doc.node_count(), initial_count - 3);

        // Undo: must restore all 3 nodes
        apply(&mut doc, &inverse).unwrap();
        assert!(doc.node(para_id).is_some());
        assert!(doc.node(run_id).is_some());
        assert!(doc.node(text_id).is_some());
        assert_eq!(doc.node_count(), initial_count);

        // Verify text content survived
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Hello")
        );

        // Verify parent-child structure
        assert_eq!(doc.node(para_id).unwrap().parent, Some(body_id));
        assert!(doc.node(para_id).unwrap().children.contains(&run_id));
        assert!(doc.node(run_id).unwrap().children.contains(&text_id));
    }

    #[test]
    fn subtree_undo_deep_table() {
        // body > table > row > cell > para > run > text
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();

        let tbl_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(tbl_id, NodeType::Table)).unwrap();
        let row_id = doc.next_id();
        doc.insert_node(tbl_id, 0, Node::new(row_id, NodeType::TableRow)).unwrap();
        let cell_id = doc.next_id();
        doc.insert_node(row_id, 0, Node::new(cell_id, NodeType::TableCell)).unwrap();
        let para_id = doc.next_id();
        doc.insert_node(cell_id, 0, Node::new(para_id, NodeType::Paragraph)).unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run)).unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "Table text")).unwrap();

        let initial_count = doc.node_count();

        // Delete the entire table
        let inverse = apply(&mut doc, &Operation::delete_node(tbl_id)).unwrap();
        assert_eq!(doc.node_count(), initial_count - 6);

        // Undo: all 6 nodes restored
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.node_count(), initial_count);
        assert_eq!(
            doc.node(text_id).unwrap().text_content.as_deref(),
            Some("Table text")
        );
    }

    #[test]
    fn subtree_undo_redo_roundtrip() {
        let (mut doc, _body_id, para_id, _run_id, text_id) = setup_doc("Roundtrip");
        let original_text = doc.to_plain_text();
        let initial_count = doc.node_count();

        // Delete
        let undo_op = apply(&mut doc, &Operation::delete_node(para_id)).unwrap();
        assert_eq!(doc.to_plain_text(), "");

        // Undo (restore)
        let redo_op = apply(&mut doc, &undo_op).unwrap();
        assert_eq!(doc.to_plain_text(), original_text);
        assert_eq!(doc.node_count(), initial_count);

        // Redo (delete again)
        apply(&mut doc, &redo_op).unwrap();
        assert!(doc.node(text_id).is_none());
        assert_eq!(doc.to_plain_text(), "");
    }

    // ─── P0 Regression: Mixed attribute undo ────────────────────────────

    #[test]
    fn attribute_undo_mixed_add_and_overwrite() {
        let (mut doc, _, _, run_id, _) = setup_doc("Hello");

        // Set initial: bold=true, fontSize=12
        apply(
            &mut doc,
            &Operation::set_attributes(
                run_id,
                AttributeMap::new().bold(true).font_size(12.0),
            ),
        )
        .unwrap();

        // Now overwrite fontSize=24 AND add italic=true
        let mixed_attrs = AttributeMap::new().font_size(24.0).italic(true);
        let inverse = apply(
            &mut doc,
            &Operation::set_attributes(run_id, mixed_attrs),
        )
        .unwrap();

        // Verify forward apply
        let node = doc.node(run_id).unwrap();
        assert_eq!(node.attributes.get_f64(&AttributeKey::FontSize), Some(24.0));
        assert_eq!(node.attributes.get_bool(&AttributeKey::Italic), Some(true));
        assert_eq!(node.attributes.get_bool(&AttributeKey::Bold), Some(true)); // untouched

        // Undo: fontSize restored to 12, italic removed, bold untouched
        apply(&mut doc, &inverse).unwrap();
        let node = doc.node(run_id).unwrap();
        assert_eq!(node.attributes.get_f64(&AttributeKey::FontSize), Some(12.0));
        assert!(!node.attributes.contains(&AttributeKey::Italic)); // was added, now removed
        assert_eq!(node.attributes.get_bool(&AttributeKey::Bold), Some(true)); // untouched
    }

    #[test]
    fn attribute_undo_byte_exact_equality() {
        // Exit criteria from remark.md: "undo after mixed attribute edits restores
        // byte-for-byte attribute equality"
        let (mut doc, _, _, run_id, _) = setup_doc("Hello");

        // Set initial attributes
        let initial = AttributeMap::new().bold(true).font_size(16.0);
        apply(&mut doc, &Operation::set_attributes(run_id, initial)).unwrap();

        let before = doc.node(run_id).unwrap().attributes.clone();

        // Apply mixed changes
        let changes = AttributeMap::new().font_size(24.0).italic(true);
        let inverse = apply(&mut doc, &Operation::set_attributes(run_id, changes)).unwrap();

        // Undo
        apply(&mut doc, &inverse).unwrap();

        let after = doc.node(run_id).unwrap().attributes.clone();
        assert_eq!(before, after, "attributes must be exactly restored after undo");
    }

    // ─── P0 Regression: Unicode-safe text operations ────────────────────

    #[test]
    fn op_insert_text_unicode_multibyte() {
        let (mut doc, _, _, _, text_id) = setup_doc("café");

        // Insert after the 'é' (char offset 4)
        let inverse = apply(&mut doc, &Operation::insert_text(text_id, 4, "!")).unwrap();
        assert_eq!(doc.node(text_id).unwrap().text_content.as_deref(), Some("café!"));

        // Undo
        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.node(text_id).unwrap().text_content.as_deref(), Some("café"));
    }

    #[test]
    fn op_insert_text_4byte_roundtrip() {
        let (mut doc, _, _, _, text_id) = setup_doc("\u{1F600}\u{1F601}");
        let original = doc.to_plain_text();

        let inverse = apply(&mut doc, &Operation::insert_text(text_id, 1, "X")).unwrap();
        assert_eq!(doc.node(text_id).unwrap().text_content.as_deref(), Some("\u{1F600}X\u{1F601}"));

        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.to_plain_text(), original);
    }

    #[test]
    fn op_delete_text_unicode_roundtrip() {
        let (mut doc, _, _, _, text_id) = setup_doc("héllo wörld");
        let original = doc.to_plain_text();

        // Delete "éllo" (chars 1..5)
        let inverse = apply(&mut doc, &Operation::delete_text(text_id, 1, 4)).unwrap();
        assert_eq!(doc.node(text_id).unwrap().text_content.as_deref(), Some("h wörld"));

        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.to_plain_text(), original);
    }

    #[test]
    fn op_text_arabic_hindi_mixed() {
        let (mut doc, _, _, _, text_id) = setup_doc("مرحبا");
        let original = doc.to_plain_text();

        let inverse = apply(&mut doc, &Operation::insert_text(text_id, 2, "\u{2192}")).unwrap();
        let content = doc.node(text_id).unwrap().text_content.clone().unwrap();
        assert_eq!(content.chars().count(), 6);

        apply(&mut doc, &inverse).unwrap();
        assert_eq!(doc.to_plain_text(), original);
    }

    // ─── Property-based tests ───────────────────────────────────────────

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn insert_text_invert_roundtrip(
                base_text in "[a-zA-Z]{1,20}",
                insert_text in "[a-zA-Z0-9 ]{1,20}",
                offset_pct in 0.0f64..=1.0,
            ) {
                let (mut doc, _, _, _, text_id) = setup_doc(&base_text);
                let original_text = doc.to_plain_text();
                let text_len = base_text.len();
                let offset = (offset_pct * text_len as f64).floor() as usize;
                let offset = offset.min(text_len);

                let op = Operation::insert_text(text_id, offset, &insert_text);
                let inverse = apply(&mut doc, &op).unwrap();

                // Text changed
                prop_assert_ne!(doc.to_plain_text(), original_text.clone());

                // Applying inverse restores original
                apply(&mut doc, &inverse).unwrap();
                prop_assert_eq!(doc.to_plain_text(), original_text);
            }

            #[test]
            fn delete_text_invert_roundtrip(
                base_text in "[a-zA-Z]{5,30}",
                start_pct in 0.0f64..1.0,
                len_pct in 0.01f64..=0.5,
            ) {
                let (mut doc, _, _, _, text_id) = setup_doc(&base_text);
                let original_text = doc.to_plain_text();
                let text_len = base_text.len();
                let offset = (start_pct * text_len as f64).floor() as usize;
                let offset = offset.min(text_len.saturating_sub(1));
                let max_len = text_len - offset;
                let length = ((len_pct * max_len as f64).ceil() as usize).max(1).min(max_len);

                let op = Operation::delete_text(text_id, offset, length);
                let inverse = apply(&mut doc, &op).unwrap();

                // Applying inverse restores original
                apply(&mut doc, &inverse).unwrap();
                prop_assert_eq!(doc.to_plain_text(), original_text);
            }
        }
    }
}
