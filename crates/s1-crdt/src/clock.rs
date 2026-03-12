//! Logical clocks for causal ordering.
//!
//! [`LamportClock`] provides a scalar logical timestamp that advances on every
//! local event and incorporates remote timestamps. [`VectorClock`] tracks the
//! latest known timestamp per replica for causal ordering comparisons.

use std::collections::HashMap;

/// A Lamport scalar clock.
///
/// Advances on every local event. On receiving a remote timestamp, updates to
/// `max(local, remote)` before advancing.
#[derive(Debug, Clone)]
pub struct LamportClock {
    timestamp: u64,
}

impl LamportClock {
    /// Create a new clock starting at 0.
    pub fn new() -> Self {
        Self { timestamp: 0 }
    }

    /// Create a clock starting at a specific timestamp.
    pub fn with_timestamp(ts: u64) -> Self {
        Self { timestamp: ts }
    }

    /// Advance the clock and return the new timestamp.
    pub fn tick(&mut self) -> u64 {
        self.timestamp += 1;
        self.timestamp
    }

    /// Incorporate a remote timestamp, then advance.
    pub fn update(&mut self, remote_ts: u64) {
        self.timestamp = self.timestamp.max(remote_ts);
    }

    /// Get the current timestamp without advancing.
    pub fn current(&self) -> u64 {
        self.timestamp
    }
}

impl Default for LamportClock {
    fn default() -> Self {
        Self::new()
    }
}

/// A vector clock mapping replica IDs to their latest known timestamps.
///
/// Used for causal ordering: if clock A dominates clock B, then all events
/// reflected in B have also been seen by A.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorClock {
    entries: HashMap<u64, u64>,
}

impl VectorClock {
    /// Create an empty vector clock.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get the timestamp for a replica (0 if unseen).
    pub fn get(&self, replica: u64) -> u64 {
        self.entries.get(&replica).copied().unwrap_or(0)
    }

    /// Set the timestamp for a replica.
    pub fn set(&mut self, replica: u64, ts: u64) {
        self.entries.insert(replica, ts);
    }

    /// Increment a replica's timestamp and return the new value.
    pub fn increment(&mut self, replica: u64) -> u64 {
        let entry = self.entries.entry(replica).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Merge another vector clock into this one (component-wise max).
    pub fn merge(&mut self, other: &VectorClock) {
        for (&replica, &ts) in &other.entries {
            let entry = self.entries.entry(replica).or_insert(0);
            *entry = (*entry).max(ts);
        }
    }

    /// Returns `true` if `self` dominates `other` (all entries >= other's).
    pub fn dominates(&self, other: &VectorClock) -> bool {
        for (&replica, &ts) in &other.entries {
            if self.get(replica) < ts {
                return false;
            }
        }
        true
    }

    /// Returns `true` if `self` and `other` are concurrent (neither dominates).
    pub fn concurrent_with(&self, other: &VectorClock) -> bool {
        !self.dominates(other) && !other.dominates(self)
    }

    /// Returns `true` if the clock has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries.
    pub fn entries(&self) -> &HashMap<u64, u64> {
        &self.entries
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── LamportClock ───────────────────────────────────────────────────

    #[test]
    fn lamport_tick() {
        let mut clock = LamportClock::new();
        assert_eq!(clock.current(), 0);
        assert_eq!(clock.tick(), 1);
        assert_eq!(clock.tick(), 2);
        assert_eq!(clock.current(), 2);
    }

    #[test]
    fn lamport_update_from_higher() {
        let mut clock = LamportClock::new();
        clock.tick(); // 1
        clock.update(10);
        assert_eq!(clock.current(), 10);
        assert_eq!(clock.tick(), 11);
    }

    #[test]
    fn lamport_update_from_lower() {
        let mut clock = LamportClock::new();
        clock.tick(); // 1
        clock.tick(); // 2
        clock.update(1);
        assert_eq!(clock.current(), 2); // unchanged
    }

    #[test]
    fn lamport_with_timestamp() {
        let clock = LamportClock::with_timestamp(42);
        assert_eq!(clock.current(), 42);
    }

    // ─── VectorClock ────────────────────────────────────────────────────

    #[test]
    fn vector_clock_empty() {
        let vc = VectorClock::new();
        assert_eq!(vc.get(1), 0);
        assert!(vc.is_empty());
    }

    #[test]
    fn vector_clock_set_get() {
        let mut vc = VectorClock::new();
        vc.set(1, 5);
        vc.set(2, 3);
        assert_eq!(vc.get(1), 5);
        assert_eq!(vc.get(2), 3);
        assert_eq!(vc.get(3), 0);
    }

    #[test]
    fn vector_clock_increment() {
        let mut vc = VectorClock::new();
        assert_eq!(vc.increment(1), 1);
        assert_eq!(vc.increment(1), 2);
        assert_eq!(vc.increment(2), 1);
    }

    #[test]
    fn vector_clock_merge() {
        let mut a = VectorClock::new();
        a.set(1, 3);
        a.set(2, 1);

        let mut b = VectorClock::new();
        b.set(1, 1);
        b.set(2, 5);
        b.set(3, 2);

        a.merge(&b);
        assert_eq!(a.get(1), 3); // max(3, 1)
        assert_eq!(a.get(2), 5); // max(1, 5)
        assert_eq!(a.get(3), 2); // max(0, 2)
    }

    #[test]
    fn vector_clock_dominates() {
        let mut a = VectorClock::new();
        a.set(1, 3);
        a.set(2, 5);

        let mut b = VectorClock::new();
        b.set(1, 2);
        b.set(2, 4);

        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn vector_clock_dominates_equal() {
        let mut a = VectorClock::new();
        a.set(1, 3);

        let mut b = VectorClock::new();
        b.set(1, 3);

        assert!(a.dominates(&b));
        assert!(b.dominates(&a));
    }

    #[test]
    fn vector_clock_concurrent() {
        let mut a = VectorClock::new();
        a.set(1, 3);
        a.set(2, 1);

        let mut b = VectorClock::new();
        b.set(1, 1);
        b.set(2, 3);

        assert!(a.concurrent_with(&b));
        assert!(b.concurrent_with(&a));
    }

    #[test]
    fn vector_clock_not_concurrent_when_dominates() {
        let mut a = VectorClock::new();
        a.set(1, 5);
        a.set(2, 5);

        let mut b = VectorClock::new();
        b.set(1, 3);

        assert!(!a.concurrent_with(&b));
    }
}
