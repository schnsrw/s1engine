//! Central conflict resolution coordinator.
//!
//! The [`CrdtResolver`] ties together the text, tree, attribute, and metadata
//! CRDTs. It accepts a [`CrdtOperation`], delegates to the appropriate sub-CRDT,
//! and produces the effective [`Operation`] to apply to the document model.

use crate::attr_crdt::AttrCrdt;
use crate::crdt_op::CrdtOperation;
use crate::error::CrdtError;
use crate::metadata_crdt::MetadataCrdt;
use crate::op_id::OpId;
use crate::text_crdt::TextCrdt;
use crate::tombstone::TombstoneTracker;
use crate::tree_crdt::TreeCrdt;
use s1_model::{DocumentModel, Node, NodeId, NodeType};
use s1_ops::Operation;

/// The central CRDT resolver that coordinates all sub-CRDTs.
#[derive(Debug, Clone)]
pub struct CrdtResolver {
    /// Text CRDT for character-level editing.
    pub text: TextCrdt,
    /// Tree CRDT for structural operations.
    pub tree: TreeCrdt,
    /// Attribute CRDT for formatting.
    pub attr: AttrCrdt,
    /// Metadata CRDT for document properties and styles.
    pub metadata: MetadataCrdt,
    /// Tombstone tracker for garbage collection.
    pub tombstones: TombstoneTracker,
}

impl CrdtResolver {
    /// Create a new empty resolver.
    pub fn new() -> Self {
        Self {
            text: TextCrdt::new(),
            tree: TreeCrdt::new(),
            attr: AttrCrdt::new(),
            metadata: MetadataCrdt::new(),
            tombstones: TombstoneTracker::new(),
        }
    }

    /// Initialize the resolver from an existing document model.
    ///
    /// Registers all existing nodes, text content, attributes, metadata,
    /// and styles with the appropriate sub-CRDTs.
    pub fn init_from_model(&mut self, model: &DocumentModel) -> u64 {
        let replica = model.replica_id();
        let mut lamport: u64 = 1;

        // Register all nodes in the tree CRDT
        let root_id = model.root_id();
        self.register_subtree(model, root_id, replica, &mut lamport);

        // Register metadata
        let meta = model.metadata();
        if let Some(title) = &meta.title {
            let op_id = OpId::new(replica, lamport);
            lamport += 1;
            self.metadata
                .register_metadata("title", Some(title.clone()), op_id);
        }
        if let Some(creator) = &meta.creator {
            let op_id = OpId::new(replica, lamport);
            lamport += 1;
            self.metadata
                .register_metadata("creator", Some(creator.clone()), op_id);
        }
        if let Some(subject) = &meta.subject {
            let op_id = OpId::new(replica, lamport);
            lamport += 1;
            self.metadata
                .register_metadata("subject", Some(subject.clone()), op_id);
        }
        if let Some(description) = &meta.description {
            let op_id = OpId::new(replica, lamport);
            lamport += 1;
            self.metadata
                .register_metadata("description", Some(description.clone()), op_id);
        }
        if let Some(language) = &meta.language {
            let op_id = OpId::new(replica, lamport);
            lamport += 1;
            self.metadata
                .register_metadata("language", Some(language.clone()), op_id);
        }

        // Register styles
        for style in model.styles() {
            let op_id = OpId::new(replica, lamport);
            lamport += 1;
            self.metadata.register_style(style, op_id);
        }

        lamport
    }

    /// Recursively register a subtree in the tree and text CRDTs.
    fn register_subtree(
        &mut self,
        model: &DocumentModel,
        node_id: NodeId,
        replica: u64,
        lamport: &mut u64,
    ) {
        let node = match model.node(node_id) {
            Some(n) => n,
            None => return,
        };

        // Register in tree CRDT (skip root, it has no parent)
        if let Some(parent_id) = node.parent {
            let op_id = OpId::new(replica, *lamport);
            *lamport += 1;
            self.tree.register_node(node_id, parent_id, op_id);
        }

        // Register attributes
        if !node.attributes.is_empty() {
            let op_id = OpId::new(replica, *lamport);
            *lamport += 1;
            self.attr
                .register_attributes(node_id, &node.attributes, op_id);
        }

        // Register text content in text CRDT
        if node.node_type == NodeType::Text {
            if let Some(content) = &node.text_content {
                if !content.is_empty() {
                    *lamport = self.text.init_text(node_id, replica, *lamport, content);
                }
            }
        }

        // Recurse into children
        let children: Vec<NodeId> = node.children.clone();
        for child_id in children {
            self.register_subtree(model, child_id, replica, lamport);
        }
    }

    /// Integrate a CRDT operation and produce the effective model operations.
    ///
    /// This is the main entry point for applying remote operations. It:
    /// 1. Delegates to the appropriate sub-CRDT
    /// 2. Resolves conflicts using CRDT semantics
    /// 3. Returns a list of operations to apply to the model (may be empty for no-ops,
    ///    or multiple for multi-character text inserts).
    pub fn integrate(
        &mut self,
        model: &DocumentModel,
        crdt_op: &CrdtOperation,
    ) -> Result<Vec<Operation>, CrdtError> {
        let opt = match &crdt_op.operation {
            Operation::InsertNode {
                parent_id,
                index: _,
                node,
            } => self.integrate_insert_node(model, crdt_op.id, *parent_id, node)?,

            Operation::DeleteNode { target_id, .. } => {
                self.integrate_delete_node(*target_id, crdt_op.id)?
            }

            Operation::MoveNode {
                target_id,
                new_parent_id,
                new_index: _,
                ..
            } => self.integrate_move_node(*target_id, *new_parent_id, crdt_op.id)?,

            Operation::InsertText {
                target_id,
                offset: _,
                text,
            } => {
                // Multi-char text inserts return individual per-character operations
                // to ensure correct interleaving with concurrent inserts.
                return self.integrate_insert_text(
                    *target_id,
                    crdt_op.id,
                    crdt_op.origin_left,
                    crdt_op.origin_right,
                    text,
                );
            }

            Operation::DeleteText {
                target_id,
                offset,
                length,
                ..
            } => self.integrate_delete_text(*target_id, *offset, *length, crdt_op.id)?,

            Operation::SetAttributes {
                target_id,
                attributes,
                ..
            } => {
                let effective = self.attr.integrate_set(*target_id, attributes, crdt_op.id);
                if effective.is_empty() {
                    None
                } else {
                    Some(Operation::set_attributes(*target_id, effective))
                }
            }

            Operation::RemoveAttributes {
                target_id, keys, ..
            } => {
                let effective = self.attr.integrate_remove(*target_id, keys, crdt_op.id);
                if effective.is_empty() {
                    None
                } else {
                    Some(Operation::remove_attributes(*target_id, effective))
                }
            }

            Operation::SetMetadata { key, value, .. } => {
                if self
                    .metadata
                    .integrate_set_metadata(key, value.clone(), crdt_op.id)
                {
                    Some(Operation::set_metadata(key.clone(), value.clone()))
                } else {
                    None
                }
            }

            Operation::SetStyle { style, .. } => {
                if self.metadata.integrate_set_style(style, crdt_op.id) {
                    Some(Operation::set_style(style.clone()))
                } else {
                    None
                }
            }

            Operation::RemoveStyle { style_id, .. } => {
                if self.metadata.integrate_remove_style(style_id, crdt_op.id) {
                    Some(Operation::remove_style(style_id.clone()))
                } else {
                    None
                }
            }

            _ => {
                return Err(CrdtError::InvalidOperation(
                    "Unsupported operation type for CRDT integration".to_string(),
                ));
            }
        };
        Ok(opt.into_iter().collect())
    }

    fn integrate_insert_node(
        &mut self,
        model: &DocumentModel,
        op_id: OpId,
        parent_id: NodeId,
        node: &Node,
    ) -> Result<Option<Operation>, CrdtError> {
        // Integrate into tree CRDT
        self.tree.integrate_insert(node.id, parent_id, op_id)?;

        // Register attributes if present
        if !node.attributes.is_empty() {
            self.attr
                .register_attributes(node.id, &node.attributes, op_id);
        }

        // Calculate the effective index from the tree CRDT's ordering
        let siblings = self.tree.visible_children(parent_id);
        let index = siblings
            .iter()
            .position(|id| *id == node.id)
            .unwrap_or(siblings.len());

        // Check if parent is tombstoned — if so, don't insert into model
        if self.tree.is_tombstoned(parent_id) {
            return Ok(None);
        }

        // Check if parent exists in model
        if model.node(parent_id).is_none() {
            return Ok(None);
        }

        Ok(Some(Operation::insert_node(parent_id, index, node.clone())))
    }

    fn integrate_delete_node(
        &mut self,
        target_id: NodeId,
        op_id: OpId,
    ) -> Result<Option<Operation>, CrdtError> {
        let was_visible = !self.tree.is_tombstoned(target_id);

        let actually_deleted = self
            .tree
            .integrate_delete(target_id, op_id)
            .unwrap_or(false);

        if actually_deleted {
            self.tombstones.add_tree_tombstone(target_id, op_id);
        }

        if was_visible && actually_deleted {
            Ok(Some(Operation::delete_node(target_id)))
        } else {
            Ok(None) // Already deleted or was already invisible
        }
    }

    fn integrate_move_node(
        &mut self,
        target_id: NodeId,
        new_parent_id: NodeId,
        op_id: OpId,
    ) -> Result<Option<Operation>, CrdtError> {
        let applied = self.tree.integrate_move(target_id, new_parent_id, op_id)?;

        if !applied {
            return Ok(None);
        }

        // Calculate effective index
        let siblings = self.tree.visible_children(new_parent_id);
        let index = siblings
            .iter()
            .position(|id| *id == target_id)
            .unwrap_or(siblings.len());

        Ok(Some(Operation::move_node(target_id, new_parent_id, index)))
    }

    fn integrate_insert_text(
        &mut self,
        target_id: NodeId,
        op_id: OpId,
        origin_left: Option<OpId>,
        origin_right: Option<OpId>,
        text: &str,
    ) -> Result<Vec<Operation>, CrdtError> {
        // Each character is integrated individually with sequential OpIds.
        // Each produces its own InsertText operation at the CRDT-determined offset
        // to ensure correct interleaving with concurrent inserts.
        let mut ops = Vec::new();
        let mut prev_id = origin_left;

        for (i, ch) in text.chars().enumerate() {
            let char_id = OpId::new(op_id.replica, op_id.lamport + i as u64);
            let offset = self.text.integrate_insert(
                target_id,
                char_id,
                prev_id,
                if i == 0 { origin_right } else { None },
                ch,
            );

            ops.push(Operation::insert_text(target_id, offset, ch.to_string()));
            prev_id = Some(char_id);
        }

        Ok(ops)
    }

    fn integrate_delete_text(
        &mut self,
        target_id: NodeId,
        offset: usize,
        length: usize,
        op_id: OpId,
    ) -> Result<Option<Operation>, CrdtError> {
        // Collect the OpIds of characters at the given range
        let mut char_ids = Vec::new();
        for i in 0..length {
            if let Some(char_id) = self.text.offset_to_op_id(target_id, offset + i) {
                char_ids.push(char_id);
            }
        }

        if char_ids.is_empty() {
            return Ok(None);
        }

        // Delete each character and track the effective range
        let mut min_offset = usize::MAX;
        let mut deleted_count = 0;

        for char_id in &char_ids {
            if let Some(del_offset) = self.text.integrate_delete(target_id, *char_id) {
                min_offset = min_offset.min(del_offset);
                deleted_count += 1;
                self.tombstones
                    .add_text_tombstone(target_id, *char_id, op_id);
            }
        }

        if deleted_count > 0 {
            Ok(Some(Operation::delete_text(
                target_id,
                min_offset,
                deleted_count,
            )))
        } else {
            Ok(None) // All characters already deleted
        }
    }

    /// Prepare a local operation for broadcast.
    ///
    /// Converts model offsets to CRDT positions and returns the CrdtOperation
    /// metadata (origin_left, origin_right) needed for the text CRDT.
    pub fn prepare_local_text_insert(
        &self,
        target_id: NodeId,
        offset: usize,
    ) -> (Option<OpId>, Option<OpId>) {
        self.text.neighbors_at_offset(target_id, offset)
    }
}

impl Default for CrdtResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_vector::StateVector;
    use s1_model::{AttributeMap, Node, NodeType};

    fn setup_doc() -> (DocumentModel, NodeId) {
        let doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        (doc, body_id)
    }

    #[test]
    fn integrate_insert_node() {
        let (doc, body_id) = setup_doc();
        let mut resolver = CrdtResolver::new();

        let para_id = NodeId::new(1, 1);
        let node = Node::new(para_id, NodeType::Paragraph);
        let crdt_op = CrdtOperation::new(
            OpId::new(1, 1),
            StateVector::new(),
            Operation::insert_node(body_id, 0, node),
        );

        let result = resolver.integrate(&doc, &crdt_op).unwrap();
        assert!(!result.is_empty());

        if let Operation::InsertNode { parent_id, .. } = &result[0] {
            assert_eq!(*parent_id, body_id);
        } else {
            panic!("Expected InsertNode");
        }
    }

    #[test]
    fn integrate_delete_node() {
        let (doc, body_id) = setup_doc();
        let mut resolver = CrdtResolver::new();

        // First insert
        let para_id = NodeId::new(1, 1);
        let node = Node::new(para_id, NodeType::Paragraph);
        let insert_op = CrdtOperation::new(
            OpId::new(1, 1),
            StateVector::new(),
            Operation::insert_node(body_id, 0, node),
        );
        resolver.integrate(&doc, &insert_op).unwrap();

        // Then delete
        let delete_op = CrdtOperation::new(
            OpId::new(1, 2),
            StateVector::new(),
            Operation::delete_node(para_id),
        );
        let result = resolver.integrate(&doc, &delete_op).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn integrate_delete_already_deleted() {
        let (doc, body_id) = setup_doc();
        let mut resolver = CrdtResolver::new();

        let para_id = NodeId::new(1, 1);
        let node = Node::new(para_id, NodeType::Paragraph);
        let insert_op = CrdtOperation::new(
            OpId::new(1, 1),
            StateVector::new(),
            Operation::insert_node(body_id, 0, node),
        );
        resolver.integrate(&doc, &insert_op).unwrap();

        // Delete twice
        let del1 = CrdtOperation::new(
            OpId::new(1, 2),
            StateVector::new(),
            Operation::delete_node(para_id),
        );
        let del2 = CrdtOperation::new(
            OpId::new(2, 3),
            StateVector::new(),
            Operation::delete_node(para_id),
        );

        let r1 = resolver.integrate(&doc, &del1).unwrap();
        assert!(!r1.is_empty());

        let r2 = resolver.integrate(&doc, &del2).unwrap();
        assert!(r2.is_empty()); // Already deleted
    }

    #[test]
    fn integrate_set_attributes() {
        let (doc, _body_id) = setup_doc();
        let mut resolver = CrdtResolver::new();

        let node_id = NodeId::new(1, 1);
        let attrs = AttributeMap::new().bold(true);
        let crdt_op = CrdtOperation::new(
            OpId::new(1, 1),
            StateVector::new(),
            Operation::set_attributes(node_id, attrs),
        );

        let result = resolver.integrate(&doc, &crdt_op).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn integrate_set_metadata() {
        let (doc, _) = setup_doc();
        let mut resolver = CrdtResolver::new();

        let crdt_op = CrdtOperation::new(
            OpId::new(1, 1),
            StateVector::new(),
            Operation::set_metadata("title", Some("Test".into())),
        );

        let result = resolver.integrate(&doc, &crdt_op).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn integrate_text_insert() {
        let (doc, _) = setup_doc();
        let mut resolver = CrdtResolver::new();

        let text_id = NodeId::new(1, 5);
        let crdt_op = CrdtOperation::new(
            OpId::new(1, 1),
            StateVector::new(),
            Operation::insert_text(text_id, 0, "hi"),
        )
        .with_text_origins(None, None);

        let result = resolver.integrate(&doc, &crdt_op).unwrap();
        assert!(!result.is_empty());

        // Verify text CRDT state
        assert_eq!(resolver.text.visible_text(text_id), "hi");
    }

    #[test]
    fn init_from_model() {
        let mut doc = DocumentModel::new();
        let body_id = doc.body_id().unwrap();
        let para_id = doc.next_id();
        doc.insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();
        let run_id = doc.next_id();
        doc.insert_node(para_id, 0, Node::new(run_id, NodeType::Run))
            .unwrap();
        let text_id = doc.next_id();
        doc.insert_node(run_id, 0, Node::text(text_id, "hello"))
            .unwrap();

        let mut resolver = CrdtResolver::new();
        let lamport = resolver.init_from_model(&doc);
        assert!(lamport > 1);

        // Tree CRDT should know about the nodes
        assert!(resolver.tree.has_node(para_id));
        assert!(resolver.tree.has_node(run_id));
        assert!(resolver.tree.has_node(text_id));

        // Text CRDT should have the content
        assert_eq!(resolver.text.visible_text(text_id), "hello");
    }

    #[test]
    fn prepare_local_text_insert() {
        let mut resolver = CrdtResolver::new();
        let text_id = NodeId::new(0, 5);

        // Init text with "ab"
        resolver.text.init_text(text_id, 0, 1, "ab");

        // Get neighbors for inserting at offset 1 (between 'a' and 'b')
        let (left, right) = resolver.prepare_local_text_insert(text_id, 1);
        assert!(left.is_some()); // 'a'
        assert!(right.is_some()); // 'b'
    }
}
