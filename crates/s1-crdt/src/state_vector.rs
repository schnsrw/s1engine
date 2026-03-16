//! State vectors for sync protocol.
//!
//! A [`StateVector`] tracks the highest Lamport timestamp seen from each replica.
//! Two state vectors can be compared to determine what operations one side is
//! missing from the other, enabling efficient delta synchronization.

use std::collections::HashMap;

use crate::op_id::OpId;

/// Maximum number of distinct replicas tracked in a state vector.
/// Exceeding this limit emits a debug warning and refuses to insert new entries.
/// This prevents unbounded memory growth in adversarial or misconfigured clusters.
pub const MAX_REPLICAS: usize = 10_000;

/// Tracks the highest operation timestamp seen from each replica.
///
/// Used in the sync protocol to compute deltas: `changes_since(remote_sv)` returns
/// all operations that the remote side hasn't seen.
#[derive(Debug, Clone, PartialEq)]
pub struct StateVector {
    entries: HashMap<u64, u64>,
}

impl StateVector {
    /// Create an empty state vector.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get the highest timestamp for a replica (0 if unseen).
    pub fn get(&self, replica: u64) -> u64 {
        self.entries.get(&replica).copied().unwrap_or(0)
    }

    /// Update the state vector with an operation ID.
    ///
    /// Sets the entry to `max(current, op_id.lamport)`.
    /// If the replica is new and the state vector is at the `MAX_REPLICAS` limit,
    /// the update is silently ignored and a debug warning is emitted.
    pub fn update(&mut self, op_id: OpId) {
        if !self.entries.contains_key(&op_id.replica) && self.entries.len() >= MAX_REPLICAS {
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: StateVector at MAX_REPLICAS limit ({MAX_REPLICAS}), ignoring new replica {}",
                op_id.replica
            );
            return;
        }
        let entry = self.entries.entry(op_id.replica).or_insert(0);
        *entry = (*entry).max(op_id.lamport);
    }

    /// Set the timestamp for a specific replica.
    ///
    /// If the replica is new and the state vector is at the `MAX_REPLICAS` limit,
    /// the set is silently ignored and a debug warning is emitted.
    pub fn set(&mut self, replica: u64, lamport: u64) {
        if !self.entries.contains_key(&replica) && self.entries.len() >= MAX_REPLICAS {
            #[cfg(debug_assertions)]
            eprintln!(
                "Warning: StateVector at MAX_REPLICAS limit ({MAX_REPLICAS}), ignoring new replica {replica}"
            );
            return;
        }
        self.entries.insert(replica, lamport);
    }

    /// Returns `true` if this state vector has seen the given operation.
    pub fn includes(&self, op_id: OpId) -> bool {
        self.get(op_id.replica) >= op_id.lamport
    }

    /// Returns replica IDs where `self` has newer operations than `other`.
    pub fn diff(&self, other: &StateVector) -> Vec<u64> {
        let mut result = Vec::new();
        for (&replica, &ts) in &self.entries {
            if ts > other.get(replica) {
                result.push(replica);
            }
        }
        result
    }

    /// Merge another state vector into this one (component-wise max).
    pub fn merge(&mut self, other: &StateVector) {
        for (&replica, &ts) in &other.entries {
            let entry = self.entries.entry(replica).or_insert(0);
            *entry = (*entry).max(ts);
        }
    }

    /// Returns `true` if the state vector has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries.
    pub fn entries(&self) -> &HashMap<u64, u64> {
        &self.entries
    }

    /// Returns `true` if this state vector includes all operations in `other`.
    pub fn includes_all(&self, other: &StateVector) -> bool {
        for (&replica, &ts) in &other.entries {
            if self.get(replica) < ts {
                return false;
            }
        }
        true
    }

    /// Remove entries for replicas not in the given active set.
    ///
    /// This helps prevent unbounded growth of the state vector when replicas
    /// disconnect permanently. Call periodically with the set of currently
    /// connected replica IDs.
    pub fn retain_active(&mut self, active_replicas: &[u64]) {
        self.entries
            .retain(|replica_id, _| active_replicas.contains(replica_id));
    }

    /// Number of replicas tracked.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for StateVector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state_vector() {
        let sv = StateVector::new();
        assert!(sv.is_empty());
        assert_eq!(sv.get(1), 0);
    }

    #[test]
    fn update_and_get() {
        let mut sv = StateVector::new();
        sv.update(OpId::new(1, 5));
        sv.update(OpId::new(1, 3)); // lower, should not regress
        sv.update(OpId::new(2, 7));
        assert_eq!(sv.get(1), 5);
        assert_eq!(sv.get(2), 7);
    }

    #[test]
    fn includes_op() {
        let mut sv = StateVector::new();
        sv.update(OpId::new(1, 5));
        assert!(sv.includes(OpId::new(1, 3)));
        assert!(sv.includes(OpId::new(1, 5)));
        assert!(!sv.includes(OpId::new(1, 6)));
        assert!(!sv.includes(OpId::new(2, 1)));
    }

    #[test]
    fn diff_finds_newer() {
        let mut a = StateVector::new();
        a.set(1, 5);
        a.set(2, 3);
        a.set(3, 1);

        let mut b = StateVector::new();
        b.set(1, 5);
        b.set(2, 1);

        let mut diff = a.diff(&b);
        diff.sort();
        assert_eq!(diff, vec![2, 3]); // replica 2 and 3 have newer ops in a
    }

    #[test]
    fn merge_state_vectors() {
        let mut a = StateVector::new();
        a.set(1, 3);
        a.set(2, 1);

        let mut b = StateVector::new();
        b.set(1, 1);
        b.set(2, 5);

        a.merge(&b);
        assert_eq!(a.get(1), 3);
        assert_eq!(a.get(2), 5);
    }

    #[test]
    fn includes_all() {
        let mut a = StateVector::new();
        a.set(1, 5);
        a.set(2, 3);

        let mut b = StateVector::new();
        b.set(1, 3);
        b.set(2, 2);

        assert!(a.includes_all(&b));
        assert!(!b.includes_all(&a));
    }

    #[test]
    fn set_explicit() {
        let mut sv = StateVector::new();
        sv.set(1, 10);
        assert_eq!(sv.get(1), 10);
        sv.set(1, 5); // overwrites, not max
        assert_eq!(sv.get(1), 5);
    }

    #[test]
    fn retain_active_removes_stale() {
        let mut sv = StateVector::new();
        sv.set(1, 10);
        sv.set(2, 20);
        sv.set(3, 30);
        sv.set(4, 40);

        assert_eq!(sv.len(), 4);

        sv.retain_active(&[1, 3]);
        assert_eq!(sv.len(), 2);
        assert_eq!(sv.get(1), 10);
        assert_eq!(sv.get(2), 0); // removed
        assert_eq!(sv.get(3), 30);
        assert_eq!(sv.get(4), 0); // removed
    }

    #[test]
    fn retain_active_empty_set() {
        let mut sv = StateVector::new();
        sv.set(1, 10);
        sv.set(2, 20);

        sv.retain_active(&[]);
        assert!(sv.is_empty());
        assert_eq!(sv.len(), 0);
    }

    #[test]
    fn len_tracks_entries() {
        let mut sv = StateVector::new();
        assert_eq!(sv.len(), 0);

        sv.set(1, 5);
        assert_eq!(sv.len(), 1);

        sv.set(2, 10);
        assert_eq!(sv.len(), 2);

        sv.set(1, 15); // update existing
        assert_eq!(sv.len(), 2);
    }

    #[test]
    fn max_replicas_limit_on_set() {
        let mut sv = StateVector::new();
        // Fill to the limit
        for i in 0..MAX_REPLICAS as u64 {
            sv.set(i, 1);
        }
        assert_eq!(sv.len(), MAX_REPLICAS);

        // Attempting to add a new replica should be ignored
        sv.set(MAX_REPLICAS as u64 + 1, 999);
        assert_eq!(sv.len(), MAX_REPLICAS);

        // Updating an existing replica should still work
        sv.set(0, 42);
        assert_eq!(sv.get(0), 42);
    }

    #[test]
    fn max_replicas_limit_on_update() {
        let mut sv = StateVector::new();
        for i in 0..MAX_REPLICAS as u64 {
            sv.update(OpId::new(i, 1));
        }
        assert_eq!(sv.len(), MAX_REPLICAS);

        // New replica via update should be ignored
        sv.update(OpId::new(MAX_REPLICAS as u64 + 1, 5));
        assert_eq!(sv.len(), MAX_REPLICAS);

        // Existing replica update should still work
        sv.update(OpId::new(0, 100));
        assert_eq!(sv.get(0), 100);
    }
}
