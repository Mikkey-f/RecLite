//! Tombstone tracker for deleted items
//!
//! This module provides efficient bit-vector tracking of deleted items using
//! the bitvec crate for compact memory usage.

use bitvec::prelude::*;

/// Tracks deleted items using a compact bit vector
pub struct TombstoneTracker {
    // Bit vector: true = tombstoned (deleted), false = active
    bits: BitVec,
}

impl TombstoneTracker {
    /// Create a new empty tombstone tracker
    pub fn new() -> Self {
        Self {
            bits: BitVec::new(),
        }
    }

    /// Create a tombstone tracker with preallocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bits: BitVec::with_capacity(capacity),
        }
    }

    /// Mark an item as deleted
    pub fn mark_deleted(&mut self, internal_id: u32) {
        let idx = internal_id as usize;

        // Grow bit vector if necessary
        if idx >= self.bits.len() {
            self.bits.resize(idx + 1, false);
        }

        self.bits.set(idx, true);
    }

    /// Clear tombstone (restore item)
    pub fn clear(&mut self, internal_id: u32) {
        let idx = internal_id as usize;

        if idx < self.bits.len() {
            self.bits.set(idx, false);
        }
    }

    /// Check if an item is tombstoned
    pub fn is_deleted(&self, internal_id: u32) -> bool {
        let idx = internal_id as usize;

        if idx >= self.bits.len() {
            return false;
        }

        self.bits[idx]
    }

    /// Count total tombstoned items
    pub fn count_deleted(&self) -> u32 {
        self.bits.count_ones() as u32
    }
}

impl Default for TombstoneTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tombstone_tracker() {
        let tracker = TombstoneTracker::new();
        assert_eq!(tracker.count_deleted(), 0);
        assert!(!tracker.is_deleted(0));
    }

    #[test]
    fn test_with_capacity() {
        let tracker = TombstoneTracker::with_capacity(100);
        assert_eq!(tracker.count_deleted(), 0);
        assert!(tracker.bits.capacity() >= 100);
    }

    #[test]
    fn test_mark_deleted() {
        let mut tracker = TombstoneTracker::new();

        tracker.mark_deleted(5);
        tracker.mark_deleted(10);

        assert!(tracker.is_deleted(5));
        assert!(tracker.is_deleted(10));
        assert!(!tracker.is_deleted(0));
        assert!(!tracker.is_deleted(7));
        assert_eq!(tracker.count_deleted(), 2);
    }

    #[test]
    fn test_clear_tombstone() {
        let mut tracker = TombstoneTracker::new();

        tracker.mark_deleted(3);
        assert!(tracker.is_deleted(3));
        assert_eq!(tracker.count_deleted(), 1);

        tracker.clear(3);
        assert!(!tracker.is_deleted(3));
        assert_eq!(tracker.count_deleted(), 0);
    }

    #[test]
    fn test_clear_nonexistent() {
        let mut tracker = TombstoneTracker::new();

        // Clearing a tombstone that was never set should be safe
        tracker.clear(999);
        assert!(!tracker.is_deleted(999));
    }

    #[test]
    fn test_is_deleted_out_of_bounds() {
        let tracker = TombstoneTracker::new();

        // Checking beyond current size should return false
        assert!(!tracker.is_deleted(1000));
    }

    #[test]
    fn test_auto_grow() {
        let mut tracker = TombstoneTracker::new();

        // Mark a high ID to test auto-growth
        tracker.mark_deleted(100);

        assert!(tracker.is_deleted(100));
        assert!(tracker.bits.len() >= 101);
        assert_eq!(tracker.count_deleted(), 1);
    }

    #[test]
    fn test_multiple_operations() {
        let mut tracker = TombstoneTracker::new();

        // Mark several items as deleted
        for i in [1, 3, 5, 7, 9] {
            tracker.mark_deleted(i);
        }

        assert_eq!(tracker.count_deleted(), 5);

        // Clear some tombstones
        tracker.clear(3);
        tracker.clear(7);

        assert_eq!(tracker.count_deleted(), 3);
        assert!(tracker.is_deleted(1));
        assert!(!tracker.is_deleted(3));
        assert!(tracker.is_deleted(5));
        assert!(!tracker.is_deleted(7));
        assert!(tracker.is_deleted(9));
    }
}
