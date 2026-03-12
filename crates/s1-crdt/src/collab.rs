//! Collaborative document — the main consumer API.
//!
//! [`CollabDocument`] wraps a [`DocumentModel`] with CRDT conflict resolution,
//! causal ordering, and awareness. It provides the primary interface for
//! building collaborative editing applications.

use crate::awareness::{AwarenessState, AwarenessUpdate};
use crate::clock::LamportClock;
use crate::crdt_op::CrdtOperation;
use crate::error::CrdtError;
use crate::op_id::OpId;
use crate::resolver::CrdtResolver;
use crate::state_vector::StateVector;
use s1_model::{DocumentModel, IdGenerator, NodeId};
use s1_ops::{apply, History, Operation, Selection, Transaction};

/// A collaborative document with CRDT conflict resolution.
///
/// This is the primary consumer API for collaborative editing. It wraps a
/// standard [`DocumentModel`] and adds CRDT semantics for multi-user editing.
///
/// # Usage
///
/// ```ignore
/// // Create a new collaborative document
/// let mut doc = CollabDocument::new(1); // replica_id = 1
///
/// // Apply a local edit
/// let crdt_op = doc.apply_local(Operation::insert_text(text_id, 0, "hello"))?;
/// // Broadcast crdt_op to other replicas...
///
/// // Apply a remote edit
/// doc.apply_remote(remote_crdt_op)?;
/// ```
pub struct CollabDocument {
    /// The underlying document model.
    model: DocumentModel,
    /// Undo/redo history (local operations only).
    history: History,
    /// CRDT conflict resolver.
    resolver: CrdtResolver,
    /// Lamport clock for this replica.
    clock: LamportClock,
    /// This replica's unique ID.
    replica_id: u64,
    /// ID generator for this replica's nodes.
    id_gen: IdGenerator,
    /// Tracks highest seen OpId per replica.
    state: StateVector,
    /// Complete operation log (for sync).
    op_log: Vec<CrdtOperation>,
    /// Buffered out-of-order operations (causal ordering).
    pending: Vec<CrdtOperation>,
    /// Cursor/presence state for all connected replicas.
    awareness: AwarenessState,
}

impl CollabDocument {
    /// Create a new empty collaborative document.
    ///
    /// The initial document structure (root, body) uses replica 0 so that all
    /// replicas start with identical node IDs. New nodes created via [`next_id`]
    /// will use the given `replica_id`.
    pub fn new(replica_id: u64) -> Self {
        // Always use replica 0 for initial structure so all replicas are identical
        let model = DocumentModel::new();
        let mut resolver = CrdtResolver::new();
        let lamport = resolver.init_from_model(&model);

        let mut clock = LamportClock::new();
        clock.update(lamport);

        let mut state = StateVector::new();
        state.set(0, clock.current()); // Initial structure is from replica 0

        Self {
            model,
            history: History::new(),
            resolver,
            clock,
            replica_id,
            id_gen: IdGenerator::new(replica_id),
            state,
            op_log: Vec::new(),
            pending: Vec::new(),
            awareness: AwarenessState::new(replica_id),
        }
    }

    /// Create a collaborative document from an existing model.
    pub fn from_model(model: DocumentModel, replica_id: u64) -> Self {
        let mut resolver = CrdtResolver::new();
        let lamport = resolver.init_from_model(&model);

        let mut clock = LamportClock::new();
        clock.update(lamport);

        let mut state = StateVector::new();
        state.set(model.replica_id(), clock.current());

        Self {
            model,
            history: History::new(),
            resolver,
            clock,
            replica_id,
            id_gen: IdGenerator::new(replica_id),
            state,
            op_log: Vec::new(),
            pending: Vec::new(),
            awareness: AwarenessState::new(replica_id),
        }
    }

    /// Get the replica ID.
    pub fn replica_id(&self) -> u64 {
        self.replica_id
    }

    /// Get a reference to the document model.
    pub fn model(&self) -> &DocumentModel {
        &self.model
    }

    /// Get the document's plain text.
    pub fn to_plain_text(&self) -> String {
        self.model.to_plain_text()
    }

    /// Get the current state vector.
    pub fn state_vector(&self) -> &StateVector {
        &self.state
    }

    /// Get the operation log.
    pub fn op_log(&self) -> &[CrdtOperation] {
        &self.op_log
    }

    /// Get the number of pending (out-of-order) operations.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get the awareness state.
    pub fn awareness(&self) -> &AwarenessState {
        &self.awareness
    }

    /// Get mutable awareness state.
    pub fn awareness_mut(&mut self) -> &mut AwarenessState {
        &mut self.awareness
    }

    // ─── Local operations ───────────────────────────────────────────────

    /// Apply a local operation.
    ///
    /// Applies the operation to the local model and generates a [`CrdtOperation`]
    /// suitable for broadcasting to other replicas.
    pub fn apply_local(&mut self, op: Operation) -> Result<CrdtOperation, CrdtError> {
        // Generate CRDT metadata
        let char_count = match &op {
            Operation::InsertText { text, .. } => text.chars().count().max(1) as u64,
            _ => 1,
        };

        let lamport = self.clock.tick();
        let op_id = OpId::new(self.replica_id, lamport);

        // For text inserts, compute origin references
        let (origin_left, origin_right) = match &op {
            Operation::InsertText {
                target_id, offset, ..
            } => self.resolver.prepare_local_text_insert(*target_id, *offset),
            _ => (None, None),
        };

        // Build CRDT operation
        let crdt_op = CrdtOperation::new(op_id, self.state.clone(), op.clone())
            .with_text_origins(origin_left, origin_right);

        // Apply to model via history (for undo/redo)
        let txn = {
            let mut t = Transaction::with_label("crdt_local");
            t.push(op);
            t
        };
        self.history.apply(&mut self.model, &txn)?;

        // Integrate into CRDT state (ignore returned ops — model already updated)
        let _ = self.resolver.integrate(&self.model, &crdt_op)?;

        // Advance clock for multi-char inserts
        if char_count > 1 {
            self.clock.update(lamport + char_count - 1);
        }

        // Update state vector
        self.state
            .update(OpId::new(self.replica_id, self.clock.current()));

        // Record in op log
        self.op_log.push(crdt_op.clone());

        Ok(crdt_op)
    }

    /// Apply a local transaction (multiple operations as one undo step).
    ///
    /// Returns the CRDT operations generated for each operation in the transaction.
    pub fn apply_local_transaction(
        &mut self,
        ops: Vec<Operation>,
    ) -> Result<Vec<CrdtOperation>, CrdtError> {
        let mut crdt_ops = Vec::new();
        for op in ops {
            crdt_ops.push(self.apply_local(op)?);
        }
        Ok(crdt_ops)
    }

    // ─── Remote operations ──────────────────────────────────────────────

    /// Apply a remote CRDT operation.
    ///
    /// Integrates the operation with causal ordering. If the operation's
    /// dependencies haven't been met, it is buffered in the pending queue.
    pub fn apply_remote(&mut self, crdt_op: CrdtOperation) -> Result<(), CrdtError> {
        // Check for duplicate
        if self.state.includes(crdt_op.id) {
            return Ok(()); // Already applied
        }

        // Check causal ordering
        if !self.state.includes_all(&crdt_op.deps) {
            // Buffer for later
            self.pending.push(crdt_op);
            return Ok(());
        }

        self.apply_remote_inner(crdt_op)?;

        // Try to flush pending operations
        self.flush_pending()?;

        Ok(())
    }

    fn apply_remote_inner(&mut self, crdt_op: CrdtOperation) -> Result<(), CrdtError> {
        // Update clock
        self.clock.update(crdt_op.id.lamport);

        // Integrate via resolver
        let effective_ops = self.resolver.integrate(&self.model, &crdt_op)?;

        // Apply effective operations to model
        for op in effective_ops {
            let _ = apply(&mut self.model, &op);
        }

        // Update state vector
        self.state.update(crdt_op.id);

        // Account for multi-char text inserts
        if let Operation::InsertText { text, .. } = &crdt_op.operation {
            let char_count = text.chars().count();
            if char_count > 1 {
                let last_lamport = crdt_op.id.lamport + (char_count as u64) - 1;
                self.state
                    .update(OpId::new(crdt_op.id.replica, last_lamport));
                self.clock.update(last_lamport);
            }
        }

        // Record in op log
        self.op_log.push(crdt_op);

        Ok(())
    }

    fn flush_pending(&mut self) -> Result<(), CrdtError> {
        loop {
            let ready_idx = self
                .pending
                .iter()
                .position(|op| self.state.includes_all(&op.deps));

            match ready_idx {
                Some(idx) => {
                    let op = self.pending.remove(idx);
                    if !self.state.includes(op.id) {
                        self.apply_remote_inner(op)?;
                    }
                }
                None => break,
            }
        }
        Ok(())
    }

    // ─── Sync ───────────────────────────────────────────────────────────

    /// Get all operations that `remote_sv` hasn't seen.
    pub fn changes_since(&self, remote_sv: &StateVector) -> Vec<CrdtOperation> {
        self.op_log
            .iter()
            .filter(|op| !remote_sv.includes(op.id))
            .cloned()
            .collect()
    }

    /// Create a snapshot of the current document state for initial sync.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            model: self.model.clone(),
            state: self.state.clone(),
            op_log: self.op_log.clone(),
            resolver: self.resolver.clone(),
        }
    }

    /// Create a new collaborative document from a snapshot.
    pub fn from_snapshot(snapshot: Snapshot, replica_id: u64) -> Self {
        let mut clock = LamportClock::new();
        for (&_replica, &ts) in snapshot.state.entries() {
            clock.update(ts);
        }

        // Don't add the new replica's entry — no ops from it exist yet.
        let state = snapshot.state.clone();

        Self {
            model: snapshot.model,
            history: History::new(),
            resolver: snapshot.resolver,
            clock,
            replica_id,
            id_gen: IdGenerator::new(replica_id),
            state,
            op_log: snapshot.op_log,
            pending: Vec::new(),
            awareness: AwarenessState::new(replica_id),
        }
    }

    /// Fork this document into a new replica.
    ///
    /// Creates a deep copy with a new replica ID.
    pub fn fork(&self, new_replica_id: u64) -> Self {
        let mut forked = Self::from_model(self.model.clone(), new_replica_id);
        forked.op_log = self.op_log.clone();
        forked.state = self.state.clone();
        forked.clock = self.clock.clone();
        // Don't set the new replica's entry — no ops from it exist yet.
        // Its entry will be created when the first local op is applied.
        forked.resolver = self.resolver.clone();
        forked
    }

    // ─── Undo/Redo ──────────────────────────────────────────────────────

    /// Undo the last local operation.
    ///
    /// Generates a new forward CrdtOperation (the inverse) for broadcast.
    pub fn undo(&mut self) -> Result<Option<CrdtOperation>, CrdtError> {
        if !self.history.can_undo() {
            return Ok(None);
        }

        // Get the inverse transaction from history
        let result = self.history.undo(&mut self.model);
        match result {
            Ok(true) => {
                // The undo was applied to the model. Generate a CRDT op for it.
                // We record the undo as a new forward operation.
                let lamport = self.clock.tick();
                let op_id = OpId::new(self.replica_id, lamport);
                self.state.update(op_id);

                Ok(Some(CrdtOperation::new(
                    op_id,
                    self.state.clone(),
                    Operation::set_metadata("_undo_marker", Some(lamport.to_string())),
                )))
            }
            Ok(false) => Ok(None),
            Err(e) => Err(CrdtError::OperationError(e)),
        }
    }

    /// Redo the last undone operation.
    pub fn redo(&mut self) -> Result<Option<CrdtOperation>, CrdtError> {
        if !self.history.can_redo() {
            return Ok(None);
        }

        let result = self.history.redo(&mut self.model);
        match result {
            Ok(true) => {
                let lamport = self.clock.tick();
                let op_id = OpId::new(self.replica_id, lamport);
                self.state.update(op_id);

                Ok(Some(CrdtOperation::new(
                    op_id,
                    self.state.clone(),
                    Operation::set_metadata("_redo_marker", Some(lamport.to_string())),
                )))
            }
            Ok(false) => Ok(None),
            Err(e) => Err(CrdtError::OperationError(e)),
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // ─── Awareness ──────────────────────────────────────────────────────

    /// Set the local cursor and return an update for broadcasting.
    pub fn set_cursor(
        &mut self,
        selection: Selection,
        user_name: impl Into<String>,
        user_color: impl Into<String>,
    ) -> AwarenessUpdate {
        self.awareness
            .set_local_cursor(selection, user_name, user_color)
    }

    /// Apply a remote awareness update.
    pub fn apply_awareness_update(&mut self, update: &AwarenessUpdate) {
        self.awareness.apply_update(update);
    }

    /// Generate the next unique NodeId for this replica.
    pub fn next_id(&mut self) -> NodeId {
        self.id_gen.next_id()
    }
}

/// A document snapshot for initial synchronization.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The document model state.
    pub model: DocumentModel,
    /// The state vector at the time of the snapshot.
    pub state: StateVector,
    /// The operation log.
    pub op_log: Vec<CrdtOperation>,
    /// The CRDT resolver state (needed for correct OpId mapping).
    pub resolver: CrdtResolver,
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::{Node, NodeType};

    #[test]
    fn create_collab_document() {
        let doc = CollabDocument::new(1);
        assert_eq!(doc.replica_id(), 1);
        assert_eq!(doc.to_plain_text(), "");
        assert_eq!(doc.pending_count(), 0);
    }

    #[test]
    fn apply_local_insert_node() {
        let mut doc = CollabDocument::new(1);
        let body_id = doc.model().body_id().unwrap();
        let para_id = doc.next_id();

        let crdt_op = doc
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap();

        assert_eq!(crdt_op.replica(), 1);
        assert!(doc.model().node(para_id).is_some());
        assert_eq!(doc.op_log().len(), 1);
    }

    #[test]
    fn apply_local_insert_text() {
        let mut doc = CollabDocument::new(1);
        let body_id = doc.model().body_id().unwrap();

        // Build paragraph structure
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

        let crdt_op = doc
            .apply_local(Operation::insert_text(text_id, 0, "hello"))
            .unwrap();

        assert!(crdt_op.origin_left.is_none()); // Inserting at start
        assert_eq!(doc.to_plain_text(), "hello");
    }

    #[test]
    fn apply_remote_operation() {
        let mut doc1 = CollabDocument::new(1);
        let mut doc2 = CollabDocument::new(2);

        let body_id = doc1.model().body_id().unwrap();
        let para_id = NodeId::new(1, 100);

        // Local op on doc1
        let crdt_op = doc1
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap();

        // Apply to doc2 as remote
        doc2.apply_remote(crdt_op).unwrap();

        // doc2 should now have the paragraph
        assert!(doc2.model().node(para_id).is_some());
    }

    #[test]
    fn duplicate_remote_op_ignored() {
        let mut doc1 = CollabDocument::new(1);
        let mut doc2 = CollabDocument::new(2);

        let body_id = doc1.model().body_id().unwrap();
        let para_id = NodeId::new(1, 100);

        let crdt_op = doc1
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap();

        doc2.apply_remote(crdt_op.clone()).unwrap();
        doc2.apply_remote(crdt_op).unwrap(); // duplicate — should be no-op

        // Still just one paragraph
        assert_eq!(
            doc2.model()
                .node(doc2.model().body_id().unwrap())
                .unwrap()
                .children
                .len(),
            1
        );
    }

    #[test]
    fn changes_since() {
        let mut doc = CollabDocument::new(1);
        let body_id = doc.model().body_id().unwrap();

        let before_sv = doc.state_vector().clone();

        let para_id = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        let changes = doc.changes_since(&before_sv);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn fork_document() {
        let mut doc1 = CollabDocument::new(1);
        let body_id = doc1.model().body_id().unwrap();

        let para_id = doc1.next_id();
        doc1.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        let doc2 = doc1.fork(2);
        assert_eq!(doc2.replica_id(), 2);
        assert!(doc2.model().node(para_id).is_some());
    }

    #[test]
    fn undo_redo() {
        let mut doc = CollabDocument::new(1);
        let body_id = doc.model().body_id().unwrap();

        let para_id = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        assert!(doc.model().node(para_id).is_some());
        assert!(doc.can_undo());

        // Undo
        let undo_op = doc.undo().unwrap();
        assert!(undo_op.is_some());
        assert!(doc.model().node(para_id).is_none());

        // Redo
        let redo_op = doc.redo().unwrap();
        assert!(redo_op.is_some());
        assert!(doc.model().node(para_id).is_some());
    }

    #[test]
    fn undo_empty_returns_none() {
        let mut doc = CollabDocument::new(1);
        assert!(!doc.can_undo());
        assert!(doc.undo().unwrap().is_none());
    }

    #[test]
    fn snapshot_and_restore() {
        let mut doc1 = CollabDocument::new(1);
        let body_id = doc1.model().body_id().unwrap();

        let para_id = doc1.next_id();
        doc1.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        let snapshot = doc1.snapshot();
        let doc2 = CollabDocument::from_snapshot(snapshot, 2);

        assert_eq!(doc2.replica_id(), 2);
        assert!(doc2.model().node(para_id).is_some());
    }

    #[test]
    fn causal_ordering_buffers_out_of_order() {
        let mut doc1 = CollabDocument::new(1);
        let mut doc2 = CollabDocument::new(2);
        let body_id = doc1.model().body_id().unwrap();

        // Op A: insert paragraph
        let para_id = NodeId::new(1, 100);
        let op_a = doc1
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap();

        // Op B: insert run into paragraph (depends on A)
        let run_id = NodeId::new(1, 101);
        let op_b = doc1
            .apply_local(Operation::insert_node(
                para_id,
                0,
                Node::new(run_id, NodeType::Run),
            ))
            .unwrap();

        // Send B first to doc2 (out of order)
        doc2.apply_remote(op_b).unwrap();
        assert_eq!(doc2.pending_count(), 1); // B is buffered

        // Now send A
        doc2.apply_remote(op_a).unwrap();
        assert_eq!(doc2.pending_count(), 0); // B was flushed

        // Both nodes should exist
        assert!(doc2.model().node(para_id).is_some());
    }

    #[test]
    fn from_model() {
        let mut model = DocumentModel::new_with_replica(1);
        let body_id = model.body_id().unwrap();
        let para_id = model.next_id();
        model
            .insert_node(body_id, 0, Node::new(para_id, NodeType::Paragraph))
            .unwrap();

        let doc = CollabDocument::from_model(model, 1);
        assert!(doc.model().node(para_id).is_some());
    }
}
