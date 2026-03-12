//! Cursor and presence sharing for collaborative editing.
//!
//! Each replica broadcasts its current cursor position and user metadata.
//! The [`AwarenessState`] tracks all known cursors and provides methods
//! for updating and querying the current state.

use std::collections::HashMap;

use s1_ops::Selection;

/// Represents a remote user's cursor/selection state.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorState {
    /// The replica that owns this cursor.
    pub replica_id: u64,
    /// Current selection (cursor is a collapsed selection).
    pub selection: Selection,
    /// User display name.
    pub user_name: String,
    /// User display color (CSS hex, e.g., "#ff0000").
    pub user_color: String,
    /// Monotonically increasing sequence number for staleness detection.
    pub sequence: u64,
}

/// An awareness update message for broadcasting.
#[derive(Debug, Clone, PartialEq)]
pub struct AwarenessUpdate {
    /// The replica that sent this update.
    pub replica_id: u64,
    /// Updated cursor state (None means the user disconnected).
    pub state: Option<CursorState>,
}

/// Tracks awareness (cursor/presence) state for all connected replicas.
#[derive(Debug, Clone)]
pub struct AwarenessState {
    /// Per-replica cursor state.
    cursors: HashMap<u64, CursorState>,
    /// Local replica ID.
    local_replica: u64,
    /// Local sequence counter.
    local_sequence: u64,
}

impl AwarenessState {
    /// Create a new awareness state for the given local replica.
    pub fn new(local_replica: u64) -> Self {
        Self {
            cursors: HashMap::new(),
            local_replica,
            local_sequence: 0,
        }
    }

    /// Set the local cursor position.
    ///
    /// Returns an [`AwarenessUpdate`] to broadcast to other replicas.
    pub fn set_local_cursor(
        &mut self,
        selection: Selection,
        user_name: impl Into<String>,
        user_color: impl Into<String>,
    ) -> AwarenessUpdate {
        self.local_sequence += 1;
        let state = CursorState {
            replica_id: self.local_replica,
            selection,
            user_name: user_name.into(),
            user_color: user_color.into(),
            sequence: self.local_sequence,
        };
        self.cursors.insert(self.local_replica, state.clone());

        AwarenessUpdate {
            replica_id: self.local_replica,
            state: Some(state),
        }
    }

    /// Apply a remote awareness update.
    pub fn apply_update(&mut self, update: &AwarenessUpdate) {
        match &update.state {
            Some(state) => {
                // Only apply if sequence is newer
                let should_apply = self
                    .cursors
                    .get(&update.replica_id)
                    .map(|existing| state.sequence > existing.sequence)
                    .unwrap_or(true);

                if should_apply {
                    self.cursors.insert(update.replica_id, state.clone());
                }
            }
            None => {
                // User disconnected
                self.cursors.remove(&update.replica_id);
            }
        }
    }

    /// Get all current cursor states (excluding local).
    pub fn remote_cursors(&self) -> Vec<&CursorState> {
        self.cursors
            .values()
            .filter(|c| c.replica_id != self.local_replica)
            .collect()
    }

    /// Get all cursor states (including local).
    pub fn all_cursors(&self) -> Vec<&CursorState> {
        self.cursors.values().collect()
    }

    /// Get a specific replica's cursor state.
    pub fn cursor(&self, replica_id: u64) -> Option<&CursorState> {
        self.cursors.get(&replica_id)
    }

    /// Remove stale cursors that haven't updated beyond `min_sequence`.
    ///
    /// Returns the number of cursors removed.
    pub fn remove_stale(&mut self, min_sequence: u64) -> usize {
        let before = self.cursors.len();
        self.cursors.retain(|&replica_id, state| {
            replica_id == self.local_replica || state.sequence >= min_sequence
        });
        before - self.cursors.len()
    }

    /// Generate a disconnect update for the local replica.
    pub fn disconnect(&mut self) -> AwarenessUpdate {
        self.cursors.remove(&self.local_replica);
        AwarenessUpdate {
            replica_id: self.local_replica,
            state: None,
        }
    }

    /// Number of tracked cursors.
    pub fn cursor_count(&self) -> usize {
        self.cursors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use s1_model::NodeId;
    use s1_ops::Position;

    fn collapsed_selection(node_id: NodeId, offset: usize) -> Selection {
        let pos = Position { node_id, offset };
        Selection {
            anchor: pos.clone(),
            focus: pos,
        }
    }

    #[test]
    fn set_local_cursor() {
        let mut awareness = AwarenessState::new(1);
        let sel = collapsed_selection(NodeId::new(0, 5), 3);
        let update = awareness.set_local_cursor(sel, "Alice", "#ff0000");

        assert_eq!(update.replica_id, 1);
        assert!(update.state.is_some());
        assert_eq!(awareness.cursor_count(), 1);
    }

    #[test]
    fn apply_remote_update() {
        let mut awareness = AwarenessState::new(1);

        let remote_state = CursorState {
            replica_id: 2,
            selection: collapsed_selection(NodeId::new(0, 5), 0),
            user_name: "Bob".into(),
            user_color: "#0000ff".into(),
            sequence: 1,
        };

        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: Some(remote_state),
        });

        assert_eq!(awareness.cursor_count(), 1);
        assert!(awareness.cursor(2).is_some());
        assert_eq!(awareness.cursor(2).unwrap().user_name, "Bob");
    }

    #[test]
    fn remote_cursors_excludes_local() {
        let mut awareness = AwarenessState::new(1);
        let sel = collapsed_selection(NodeId::new(0, 5), 0);
        awareness.set_local_cursor(sel, "Alice", "#ff0000");

        let remote = CursorState {
            replica_id: 2,
            selection: collapsed_selection(NodeId::new(0, 5), 3),
            user_name: "Bob".into(),
            user_color: "#0000ff".into(),
            sequence: 1,
        };
        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: Some(remote),
        });

        assert_eq!(awareness.all_cursors().len(), 2);
        assert_eq!(awareness.remote_cursors().len(), 1);
        assert_eq!(awareness.remote_cursors()[0].user_name, "Bob");
    }

    #[test]
    fn ignore_older_update() {
        let mut awareness = AwarenessState::new(1);

        let newer = CursorState {
            replica_id: 2,
            selection: collapsed_selection(NodeId::new(0, 5), 5),
            user_name: "Bob".into(),
            user_color: "#0000ff".into(),
            sequence: 5,
        };
        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: Some(newer),
        });

        let older = CursorState {
            replica_id: 2,
            selection: collapsed_selection(NodeId::new(0, 5), 0),
            user_name: "Bob".into(),
            user_color: "#0000ff".into(),
            sequence: 3,
        };
        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: Some(older),
        });

        // Should still have the newer state
        assert_eq!(awareness.cursor(2).unwrap().selection.anchor.offset, 5);
    }

    #[test]
    fn disconnect_removes_cursor() {
        let mut awareness = AwarenessState::new(1);
        let sel = collapsed_selection(NodeId::new(0, 5), 0);
        awareness.set_local_cursor(sel, "Alice", "#ff0000");
        assert_eq!(awareness.cursor_count(), 1);

        let update = awareness.disconnect();
        assert!(update.state.is_none());
        assert_eq!(awareness.cursor_count(), 0);
    }

    #[test]
    fn remote_disconnect() {
        let mut awareness = AwarenessState::new(1);

        let remote = CursorState {
            replica_id: 2,
            selection: collapsed_selection(NodeId::new(0, 5), 0),
            user_name: "Bob".into(),
            user_color: "#0000ff".into(),
            sequence: 1,
        };
        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: Some(remote),
        });
        assert_eq!(awareness.cursor_count(), 1);

        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: None,
        });
        assert_eq!(awareness.cursor_count(), 0);
    }

    #[test]
    fn remove_stale_cursors() {
        let mut awareness = AwarenessState::new(1);
        let sel = collapsed_selection(NodeId::new(0, 5), 0);
        awareness.set_local_cursor(sel.clone(), "Alice", "#ff0000");

        // Add two remote cursors with different sequences
        awareness.apply_update(&AwarenessUpdate {
            replica_id: 2,
            state: Some(CursorState {
                replica_id: 2,
                selection: sel.clone(),
                user_name: "Bob".into(),
                user_color: "#0000ff".into(),
                sequence: 1,
            }),
        });
        awareness.apply_update(&AwarenessUpdate {
            replica_id: 3,
            state: Some(CursorState {
                replica_id: 3,
                selection: sel,
                user_name: "Carol".into(),
                user_color: "#00ff00".into(),
                sequence: 10,
            }),
        });

        assert_eq!(awareness.cursor_count(), 3);

        // Remove cursors with sequence < 5 (Bob's sequence=1 is stale)
        let removed = awareness.remove_stale(5);
        assert_eq!(removed, 1);
        assert_eq!(awareness.cursor_count(), 2);
        assert!(awareness.cursor(2).is_none()); // Bob removed
        assert!(awareness.cursor(3).is_some()); // Carol kept
        assert!(awareness.cursor(1).is_some()); // Local always kept
    }
}
