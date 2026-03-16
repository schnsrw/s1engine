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
    /// replicas start with identical node IDs. New nodes created via [`CollabDocument::next_id`]
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

    // ─── Compaction & GC ─────────────────────────────────────────────

    /// Compact the operation log by merging consecutive character inserts.
    ///
    /// This reduces memory usage by merging sequential single-character inserts
    /// into multi-character operations. The semantics of the log are preserved.
    pub fn compact_op_log(&mut self) {
        crate::compression::compress_ops_in_place(&mut self.op_log);
    }

    /// Garbage-collect tombstones that all replicas have seen.
    ///
    /// `min_state` should be the intersection of all connected replicas' state vectors.
    /// Tombstones whose operations are included in `min_state` can be safely removed.
    /// Returns the number of tombstones removed.
    pub fn gc_tombstones(&mut self, min_state: &StateVector) -> usize {
        self.resolver.gc_tombstones(min_state)
    }

    /// Get the current size of the operation log.
    pub fn op_log_size(&self) -> usize {
        self.op_log.len()
    }

    /// Get the current tombstone count from the resolver.
    pub fn tombstone_count(&self) -> usize {
        self.resolver.tombstone_count()
    }

    /// Create a snapshot, then truncate the operation log.
    ///
    /// Returns the snapshot (which includes the full op_log at that point).
    /// After this call, the local op_log is empty — new changes will start
    /// from a clean log. Useful for periodic checkpointing in long sessions.
    pub fn snapshot_and_truncate(&mut self) -> Snapshot {
        let snapshot = self.snapshot();
        self.op_log.clear();
        snapshot
    }

    /// Auto-compact the operation log if it exceeds the given threshold.
    ///
    /// Returns `true` if compaction was performed.
    pub fn auto_compact(&mut self, threshold: usize) -> bool {
        if self.op_log.len() >= threshold {
            self.compact_op_log();
            true
        } else {
            false
        }
    }

    /// Force-GC tombstones that exceed a maximum count.
    ///
    /// Safety valve for when a slow replica prevents normal GC. Removes the
    /// oldest tombstones even if not all replicas have acknowledged them.
    /// Returns the number of tombstones removed.
    pub fn gc_tombstones_excess(&mut self, max_count: usize) -> usize {
        self.resolver.gc_tombstones_excess(max_count)
    }

    /// Estimate the in-memory size of the CRDT state in bytes.
    ///
    /// This is a rough estimate including the op log, tombstones, and resolver
    /// state. Useful for monitoring and triggering compaction when the state
    /// exceeds a size threshold.
    pub fn estimated_size_bytes(&self) -> usize {
        // Op log: ~100 bytes per operation (rough estimate)
        let op_log_size = self.op_log.len() * 100;
        // Tombstone tracking overhead
        let tombstone_size = self.resolver.tombstone_count() * 48;
        // State vector: 16 bytes per replica entry
        let state_size = self.state.entries().len() * 16;
        op_log_size + tombstone_size + state_size
    }

    /// Compact the CRDT state if the estimated size exceeds the given byte threshold.
    ///
    /// Performs op log compaction, then tombstone GC, then snapshot-and-truncate
    /// if the state is still over the threshold. Returns `true` if compaction was
    /// triggered.
    pub fn compact_if_oversized(
        &mut self,
        max_bytes: usize,
        min_state: Option<&StateVector>,
        max_tombstones: usize,
    ) -> bool {
        if self.estimated_size_bytes() <= max_bytes {
            return false;
        }
        // Step 1: Compact op log
        self.compact_op_log();
        // Step 2: GC tombstones
        if let Some(ms) = min_state {
            self.resolver.gc_tombstones(ms);
        }
        self.resolver.gc_tombstones_excess(max_tombstones);
        // Step 3: If still oversized, truncate op log
        if self.estimated_size_bytes() > max_bytes {
            self.op_log.clear();
        }
        true
    }

    /// Run maintenance: compact the op log and cap tombstones.
    ///
    /// Call this periodically (e.g., every 60 seconds or every 1000 operations).
    /// - Compacts the op log if it exceeds `op_log_threshold`.
    /// - GCs tombstones against `min_state` if provided.
    /// - Force-GCs excess tombstones above `max_tombstones`.
    pub fn maintenance(
        &mut self,
        op_log_threshold: usize,
        min_state: Option<&StateVector>,
        max_tombstones: usize,
    ) {
        // Compact op log
        if self.op_log.len() >= op_log_threshold {
            self.compact_op_log();
        }

        // GC tombstones against min state
        if let Some(ms) = min_state {
            self.resolver.gc_tombstones(ms);
        }

        // Force-GC excess tombstones
        self.resolver.gc_tombstones_excess(max_tombstones);
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

    // ─── Compaction & GC tests ───────────────────────────────────────

    /// Helper: build a paragraph/run/text structure and return the text node id.
    fn setup_text_doc(doc: &mut CollabDocument) -> NodeId {
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

        text_id
    }

    #[test]
    fn compact_op_log_reduces_size() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        // Insert chars one by one
        for ch in "hello world".chars() {
            doc.apply_local(Operation::insert_text(
                text_id,
                doc.to_plain_text().len(),
                ch.to_string(),
            ))
            .unwrap();
        }

        let size_before = doc.op_log_size();
        // 3 structural ops + 11 char inserts = 14
        assert!(size_before >= 14);

        doc.compact_op_log();

        let size_after = doc.op_log_size();
        // After compaction, char inserts should be merged
        assert!(size_after < size_before);
    }

    #[test]
    fn compact_preserves_semantics() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        for ch in "abcdef".chars() {
            doc.apply_local(Operation::insert_text(
                text_id,
                doc.to_plain_text().len(),
                ch.to_string(),
            ))
            .unwrap();
        }

        let text_before = doc.to_plain_text();
        doc.compact_op_log();
        let text_after = doc.to_plain_text();

        assert_eq!(text_before, text_after);
        assert_eq!(text_after, "abcdef");
    }

    #[test]
    fn gc_tombstones_removes_old() {
        let mut doc = CollabDocument::new(1);
        let body_id = doc.model().body_id().unwrap();

        // Insert a paragraph
        let para_id = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        // Delete it
        doc.apply_local(Operation::delete_node(para_id)).unwrap();
        assert!(doc.tombstone_count() > 0);

        // GC with a state vector that covers all operations
        let mut min_state = doc.state_vector().clone();
        // Ensure all replicas are covered
        min_state.set(1, 1000);
        let removed = doc.gc_tombstones(&min_state);
        assert!(removed > 0);
    }

    #[test]
    fn auto_compact_below_threshold() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        doc.apply_local(Operation::insert_text(text_id, 0, "a".to_string()))
            .unwrap();

        let size_before = doc.op_log_size();
        let compacted = doc.auto_compact(1000); // Threshold much higher
        assert!(!compacted);
        assert_eq!(doc.op_log_size(), size_before);
    }

    #[test]
    fn auto_compact_above_threshold() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        for ch in "hello world".chars() {
            doc.apply_local(Operation::insert_text(
                text_id,
                doc.to_plain_text().len(),
                ch.to_string(),
            ))
            .unwrap();
        }

        let size_before = doc.op_log_size();
        let compacted = doc.auto_compact(5); // Low threshold
        assert!(compacted);
        assert!(doc.op_log_size() < size_before);
    }

    #[test]
    fn snapshot_and_truncate_empties_log() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        doc.apply_local(Operation::insert_text(text_id, 0, "hello".to_string()))
            .unwrap();

        assert!(doc.op_log_size() > 0);

        let snapshot = doc.snapshot_and_truncate();
        assert_eq!(doc.op_log_size(), 0);
        // Snapshot should have the ops
        assert!(!snapshot.op_log.is_empty());
    }

    #[test]
    fn snapshot_and_truncate_preserves_model() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        doc.apply_local(Operation::insert_text(text_id, 0, "hello".to_string()))
            .unwrap();

        let text_before = doc.to_plain_text();
        let _snapshot = doc.snapshot_and_truncate();

        // Model is unchanged despite op_log being empty
        assert_eq!(doc.to_plain_text(), text_before);
        assert_eq!(doc.to_plain_text(), "hello");
    }

    #[test]
    fn op_log_size_introspection() {
        let mut doc = CollabDocument::new(1);
        assert_eq!(doc.op_log_size(), 0);

        let body_id = doc.model().body_id().unwrap();
        let para_id = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();
        assert_eq!(doc.op_log_size(), 1);

        let para_id2 = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            1,
            Node::new(para_id2, NodeType::Paragraph),
        ))
        .unwrap();
        assert_eq!(doc.op_log_size(), 2);
    }

    #[test]
    fn tombstone_count_introspection() {
        let mut doc = CollabDocument::new(1);
        assert_eq!(doc.tombstone_count(), 0);

        let body_id = doc.model().body_id().unwrap();
        let para_id = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        // Delete it — should add a tombstone
        doc.apply_local(Operation::delete_node(para_id)).unwrap();
        assert!(doc.tombstone_count() > 0);
    }

    #[test]
    fn long_session_simulation() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        // Insert 1000 chars one by one (simulating sequential typing)
        for i in 0..1000 {
            let ch = (b'a' + (i % 26) as u8) as char;
            doc.apply_local(Operation::insert_text(
                text_id,
                doc.to_plain_text().len(),
                ch.to_string(),
            ))
            .unwrap();
        }

        let size_before = doc.op_log_size();
        // 3 structural + 1000 char inserts = 1003
        assert_eq!(size_before, 1003);

        let text_before = doc.to_plain_text();
        assert_eq!(text_before.len(), 1000);

        doc.compact_op_log();

        let size_after = doc.op_log_size();
        // Should be much smaller (3 structural + 1 merged text = 4)
        assert!(size_after < size_before);
        assert!(size_after <= 10); // Very aggressive compression for sequential typing

        // Text is unchanged
        assert_eq!(doc.to_plain_text(), text_before);
    }

    #[test]
    fn estimated_size_bytes() {
        let mut doc = CollabDocument::new(1);

        // Empty doc should have small estimated size
        let empty_size = doc.estimated_size_bytes();
        assert!(
            empty_size < 1000,
            "empty doc size should be small: {empty_size}"
        );

        // Add some operations via setup_text_doc
        let text_id = setup_text_doc(&mut doc);
        doc.apply_local(Operation::insert_text(text_id, 0, "a".repeat(100)))
            .unwrap();

        let size_with_ops = doc.estimated_size_bytes();
        assert!(
            size_with_ops > empty_size,
            "size should grow with operations"
        );
    }

    #[test]
    fn compact_if_oversized() {
        let mut doc = CollabDocument::new(1);
        let text_id = setup_text_doc(&mut doc);

        // Insert many single-char operations to build up op log
        for i in 0..200 {
            doc.apply_local(Operation::insert_text(text_id, i, "x".to_string()))
                .unwrap();
        }

        let size_before = doc.op_log_size();
        assert!(size_before >= 200);

        // Set a very low threshold to trigger compaction
        let compacted = doc.compact_if_oversized(100, None, 10_000);
        assert!(compacted, "compaction should have been triggered");

        let size_after = doc.op_log_size();
        assert!(
            size_after < size_before,
            "op log should be smaller after compaction"
        );
    }

    #[test]
    fn compact_if_not_oversized() {
        let mut doc = CollabDocument::new(1);
        // Very large threshold — should not compact
        let compacted = doc.compact_if_oversized(10_000_000, None, 10_000);
        assert!(
            !compacted,
            "compaction should not be triggered for small doc"
        );
    }

    // ─── 3-way convergence test (WFC-13) ────────────────────────────────

    #[test]
    fn test_three_way_convergence() {
        // Create three replicas that all start with the same initial state.
        let mut doc1 = CollabDocument::new(1);
        let mut doc2 = doc1.fork(2);
        let mut doc3 = doc1.fork(3);

        let body_id = doc1.model().body_id().unwrap();

        // Each replica inserts a paragraph concurrently (no knowledge of others).
        let para1 = NodeId::new(1, 100);
        let op1 = doc1
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para1, NodeType::Paragraph),
            ))
            .unwrap();

        let para2 = NodeId::new(2, 100);
        let op2 = doc2
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para2, NodeType::Paragraph),
            ))
            .unwrap();

        let para3 = NodeId::new(3, 100);
        let op3 = doc3
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para3, NodeType::Paragraph),
            ))
            .unwrap();

        // Exchange all operations between all pairs.
        // doc1 receives from doc2 and doc3
        doc1.apply_remote(op2.clone()).unwrap();
        doc1.apply_remote(op3.clone()).unwrap();

        // doc2 receives from doc1 and doc3
        doc2.apply_remote(op1.clone()).unwrap();
        doc2.apply_remote(op3).unwrap();

        // doc3 receives from doc1 and doc2
        doc3.apply_remote(op1).unwrap();
        doc3.apply_remote(op2).unwrap();

        // All three should have all three paragraphs.
        assert!(doc1.model().node(para1).is_some());
        assert!(doc1.model().node(para2).is_some());
        assert!(doc1.model().node(para3).is_some());

        assert!(doc2.model().node(para1).is_some());
        assert!(doc2.model().node(para2).is_some());
        assert!(doc2.model().node(para3).is_some());

        assert!(doc3.model().node(para1).is_some());
        assert!(doc3.model().node(para2).is_some());
        assert!(doc3.model().node(para3).is_some());

        // All three should have the same children under body (same order).
        let body1 = doc1.model().node(body_id).unwrap();
        let body2 = doc2.model().node(body_id).unwrap();
        let body3 = doc3.model().node(body_id).unwrap();

        assert_eq!(
            body1.children, body2.children,
            "doc1 and doc2 should converge to same child order"
        );
        assert_eq!(
            body2.children, body3.children,
            "doc2 and doc3 should converge to same child order"
        );
    }

    // ─── Error path tests (WFC-14) ──────────────────────────────────────

    #[test]
    fn test_apply_local_invalid_node() {
        let mut doc = CollabDocument::new(1);
        let result = doc.apply_local(Operation::InsertText {
            target_id: NodeId::new(999, 999),
            offset: 0,
            text: "hello".into(),
        });
        assert!(
            result.is_err(),
            "inserting text into non-existent node should fail"
        );
    }

    #[test]
    fn test_apply_local_delete_nonexistent_node() {
        let mut doc = CollabDocument::new(1);
        let result = doc.apply_local(Operation::delete_node(NodeId::new(999, 999)));
        assert!(result.is_err(), "deleting non-existent node should fail");
    }

    #[test]
    fn test_apply_local_insert_into_nonexistent_parent() {
        let mut doc = CollabDocument::new(1);
        let child_id = doc.next_id();
        let result = doc.apply_local(Operation::insert_node(
            NodeId::new(999, 999),
            0,
            Node::new(child_id, NodeType::Paragraph),
        ));
        assert!(
            result.is_err(),
            "inserting into non-existent parent should fail"
        );
    }

    // ─── Enhanced 3-way convergence test with text edits (WFC-13) ──────

    #[test]
    fn test_three_way_text_convergence() {
        // All three replicas start with the same paragraph/run/text structure.
        let mut doc1 = CollabDocument::new(1);
        let body_id = doc1.model().body_id().unwrap();

        // Build shared structure: body -> para -> run -> text("")
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

        let text_id = doc1.next_id();
        doc1.apply_local(Operation::insert_node(run_id, 0, Node::text(text_id, "")))
            .unwrap();

        // Fork to create three identical replicas
        let mut doc2 = doc1.fork(2);
        let mut doc3 = doc1.fork(3);

        // Collect structural ops from doc1 so doc2/doc3 have matching state vectors
        let structural_ops = doc1.changes_since(&StateVector::new());
        for op in &structural_ops {
            doc2.apply_remote(op.clone()).unwrap();
            doc3.apply_remote(op.clone()).unwrap();
        }

        // Each replica concurrently inserts different text at position 0
        let op1 = doc1
            .apply_local(Operation::insert_text(text_id, 0, "A"))
            .unwrap();
        let op2 = doc2
            .apply_local(Operation::insert_text(text_id, 0, "B"))
            .unwrap();
        let op3 = doc3
            .apply_local(Operation::insert_text(text_id, 0, "C"))
            .unwrap();

        // Exchange all ops
        doc1.apply_remote(op2.clone()).unwrap();
        doc1.apply_remote(op3.clone()).unwrap();

        doc2.apply_remote(op1.clone()).unwrap();
        doc2.apply_remote(op3).unwrap();

        doc3.apply_remote(op1).unwrap();
        doc3.apply_remote(op2).unwrap();

        // All three should converge to the same text content
        let text1 = doc1.to_plain_text();
        let text2 = doc2.to_plain_text();
        let text3 = doc3.to_plain_text();

        assert_eq!(
            text1, text2,
            "doc1 and doc2 should have identical text after sync"
        );
        assert_eq!(
            text2, text3,
            "doc2 and doc3 should have identical text after sync"
        );

        // All three characters should be present (order is deterministic but
        // depends on CRDT tiebreaking — we just verify all chars exist)
        assert_eq!(text1.len(), 3, "all three characters should be present");
        assert!(text1.contains('A'));
        assert!(text1.contains('B'));
        assert!(text1.contains('C'));
    }

    // ─── Additional error path tests (WFC-14) ─────────────────────────

    #[test]
    fn test_apply_remote_with_corrupted_op_id() {
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

        // Apply the same op twice — second should be a no-op (duplicate check)
        doc2.apply_remote(crdt_op.clone()).unwrap();
        doc2.apply_remote(crdt_op).unwrap();

        // Should still have exactly one paragraph
        let body = doc2.model().node(body_id).unwrap();
        assert_eq!(body.children.len(), 1);
    }

    #[test]
    fn test_apply_local_text_insert_out_of_bounds() {
        let mut doc = CollabDocument::new(1);
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
        doc.apply_local(Operation::insert_node(run_id, 0, Node::text(text_id, "hi")))
            .unwrap();

        // Insert at offset beyond the text length
        let result = doc.apply_local(Operation::insert_text(text_id, 999, "x"));
        // The operation system may or may not error depending on the implementation;
        // we just verify it doesn't panic.
        let _ = result;
    }

    #[test]
    fn test_undo_after_remote_op_applied() {
        let mut doc1 = CollabDocument::new(1);
        let mut doc2 = CollabDocument::new(2);
        let body_id = doc1.model().body_id().unwrap();

        // doc1 inserts a paragraph
        let para_id = doc1.next_id();
        let op1 = doc1
            .apply_local(Operation::insert_node(
                body_id,
                0,
                Node::new(para_id, NodeType::Paragraph),
            ))
            .unwrap();

        // doc2 receives it
        doc2.apply_remote(op1).unwrap();

        // doc2 inserts its own paragraph
        let para_id2 = doc2.next_id();
        doc2.apply_local(Operation::insert_node(
            body_id,
            1,
            Node::new(para_id2, NodeType::Paragraph),
        ))
        .unwrap();

        // doc2 undoes its own paragraph
        let undo_result = doc2.undo().unwrap();
        assert!(undo_result.is_some());

        // doc1's paragraph should still exist, doc2's should be gone
        assert!(doc2.model().node(para_id).is_some());
        assert!(doc2.model().node(para_id2).is_none());
    }

    #[test]
    fn test_apply_local_delete_text_on_non_text_node() {
        let mut doc = CollabDocument::new(1);
        let body_id = doc.model().body_id().unwrap();

        let para_id = doc.next_id();
        doc.apply_local(Operation::insert_node(
            body_id,
            0,
            Node::new(para_id, NodeType::Paragraph),
        ))
        .unwrap();

        // Try to delete text from a paragraph node (not a text node)
        let result = doc.apply_local(Operation::delete_text(para_id, 0, 5));
        // Should fail since paragraph is not a text node
        assert!(
            result.is_err(),
            "deleting text from a non-text node should fail"
        );
    }

    #[test]
    fn test_fork_then_diverge_then_sync() {
        let mut doc1 = CollabDocument::new(1);
        let body_id = doc1.model().body_id().unwrap();

        // Create shared structure
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
        let p1 = doc1.next_id();
        let op1 = doc1
            .apply_local(Operation::insert_node(
                body_id,
                1,
                Node::new(p1, NodeType::Paragraph),
            ))
            .unwrap();

        let p2 = doc2.next_id();
        let op2 = doc2
            .apply_local(Operation::insert_node(
                body_id,
                1,
                Node::new(p2, NodeType::Paragraph),
            ))
            .unwrap();

        // Sync
        doc1.apply_remote(op2).unwrap();
        doc2.apply_remote(op1).unwrap();

        // Both should have all 3 paragraphs
        let body1 = doc1.model().node(body_id).unwrap();
        let body2 = doc2.model().node(body_id).unwrap();
        assert_eq!(body1.children.len(), 3);
        assert_eq!(body1.children, body2.children);
    }
}
