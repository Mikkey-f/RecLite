//! Search backend abstraction and implementations
//!
//! This module defines the pluggable search backend trait and provides
//! the SIMD-accelerated linear scan implementation.

use crate::error::RecError;
use crate::tombstone::TombstoneTracker;
use crate::vector_index::FlatVectorIndex;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

// Helper functions for cosine similarity calculation
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn magnitude(vector: &[f32]) -> f32 {
    vector.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// Trait defining the interface for similarity search backends
pub trait SearchBackend: Send + Sync {
    /// Add a vector to the search index
    fn add_vector(&mut self, internal_id: u32, vector: &[f32]) -> Result<(), RecError>;

    /// Update an existing vector
    fn update_vector(&mut self, internal_id: u32, vector: &[f32]) -> Result<(), RecError>;

    /// Search for top-K most similar vectors
    ///
    /// Returns: Vec<(internal_id, score)> sorted by score descending
    fn search(
        &self,
        query: &[f32],
        top_k: usize,
        tombstones: &TombstoneTracker,
    ) -> Result<Vec<(u32, f32)>, RecError>;

    /// Get the dimension of vectors in this backend
    fn dimension(&self) -> usize;

    /// Get the number of vectors currently stored
    fn len(&self) -> usize;

    /// Check if the backend is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Linear scan backend optimized for datasets up to ~10,000 items
pub struct LinearScanBackend {
    index: FlatVectorIndex,
}

impl LinearScanBackend {
    /// Create a new linear scan backend with specified dimension
    pub fn new(dimension: usize) -> Self {
        Self {
            index: FlatVectorIndex::new(dimension),
        }
    }

    /// Create a linear scan backend with preallocated capacity
    pub fn with_capacity(dimension: usize, capacity: usize) -> Self {
        Self {
            index: FlatVectorIndex::with_capacity(dimension, capacity),
        }
    }
}

impl SearchBackend for LinearScanBackend {
    fn add_vector(&mut self, internal_id: u32, vector: &[f32]) -> Result<(), RecError> {
        // For linear scan, internal_id must equal current count
        let expected_id = self.index.len() as u32;
        if internal_id != expected_id {
            return Err(RecError::InvalidInput(format!(
                "Expected internal_id {}, got {}",
                expected_id, internal_id
            )));
        }

        self.index.push(vector)?;
        Ok(())
    }

    fn update_vector(&mut self, internal_id: u32, vector: &[f32]) -> Result<(), RecError> {
        self.index.update(internal_id, vector)
    }

    fn len(&self) -> usize {
        self.index.len()
    }

    fn dimension(&self) -> usize {
        self.index.dimension()
    }
    fn search(
        &self,
        query: &[f32],
        top_k: usize,
        tombstones: &TombstoneTracker,
    ) -> Result<Vec<(u32, f32)>, RecError> {
        if query.len() != self.dimension() {
            return Err(RecError::DimensionMismatch {
                expected: self.dimension(),
                actual: query.len(),
            });
        }

        // Min-heap to maintain top-K results (invert ordering for max-heap behavior)
        let mut heap = BinaryHeap::with_capacity(top_k);

        // Scan all vectors
        for internal_id in 0..self.index.len() as u32 {
            // Skip tombstoned items
            if tombstones.is_deleted(internal_id) {
                continue;
            }

            let vector = self.index.get(internal_id).unwrap();

            // Compute cosine similarity using simsimd
            // For now, use a simple dot product implementation until simsimd API is confirmed
            let score = dot_product(query, vector) / (magnitude(query) * magnitude(vector));

            // Maintain top-K heap
            if heap.len() < top_k {
                heap.push(ScoredItem { internal_id, score });
            } else if let Some(min_item) = heap.peek() {
                if score > min_item.score {
                    heap.pop();
                    heap.push(ScoredItem { internal_id, score });
                }
            }
        }

        // Extract results and sort descending by score
        let mut results: Vec<(u32, f32)> = heap
            .into_iter()
            .map(|item| (item.internal_id, item.score))
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        Ok(results)
    }
}

/// Helper struct for BinaryHeap (min-heap by score)
#[derive(Debug, Clone)]
struct ScoredItem {
    internal_id: u32,
    score: f32,
}

impl PartialEq for ScoredItem {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for ScoredItem {}

impl PartialOrd for ScoredItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering for min-heap (lower scores have higher priority)
        other.score.partial_cmp(&self.score)
    }
}

impl Ord for ScoredItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Use total_cmp for safe NaN handling, then reverse for min-heap
        self.score.total_cmp(&other.score).reverse()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_scan_backend_creation() {
        let backend = LinearScanBackend::new(3);
        assert_eq!(backend.dimension(), 3);
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
    }

    #[test]
    fn test_add_vector() {
        let mut backend = LinearScanBackend::new(2);

        let vector1 = vec![0.1, 0.2];
        let vector2 = vec![0.3, 0.4];

        backend.add_vector(0, &vector1).unwrap();
        backend.add_vector(1, &vector2).unwrap();

        assert_eq!(backend.len(), 2);
    }

    #[test]
    fn test_add_vector_wrong_id() {
        let mut backend = LinearScanBackend::new(2);

        let vector = vec![0.1, 0.2];

        // Should fail because expected ID is 0, not 5
        let result = backend.add_vector(5, &vector);
        assert!(matches!(result, Err(RecError::InvalidInput(_))));
    }

    #[test]
    fn test_update_vector() {
        let mut backend = LinearScanBackend::new(2);

        let original = vec![0.1, 0.2];
        let updated = vec![0.5, 0.6];

        backend.add_vector(0, &original).unwrap();
        backend.update_vector(0, &updated).unwrap();

        // Verify update worked by checking the underlying index
        assert_eq!(backend.index.get(0), Some([0.5, 0.6].as_slice()));
    }

    #[test]
    fn test_search_empty_backend() {
        let backend = LinearScanBackend::new(3);
        let tombstones = TombstoneTracker::new();

        let query = vec![0.1, 0.2, 0.3];
        let results = backend.search(&query, 5, &tombstones).unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_search_single_item() {
        let mut backend = LinearScanBackend::new(3);
        let tombstones = TombstoneTracker::new();

        let vector = vec![0.1, 0.2, 0.3];
        backend.add_vector(0, &vector).unwrap();

        let query = vec![0.1, 0.2, 0.3]; // Identical vector
        let results = backend.search(&query, 1, &tombstones).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0); // internal_id
        assert!(results[0].1 > 0.99); // score should be very high (near 1.0)
    }

    #[test]
    fn test_search_with_tombstones() {
        let mut backend = LinearScanBackend::new(2);
        let mut tombstones = TombstoneTracker::new();

        let vector1 = vec![1.0, 0.0];
        let vector2 = vec![0.0, 1.0];

        backend.add_vector(0, &vector1).unwrap();
        backend.add_vector(1, &vector2).unwrap();

        // Mark first vector as deleted
        tombstones.mark_deleted(0);

        let query = vec![1.0, 0.0]; // Should match vector1, but it's tombstoned
        let results = backend.search(&query, 2, &tombstones).unwrap();

        // Should only return vector2 (internal_id = 1)
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[test]
    fn test_search_wrong_dimension() {
        let backend = LinearScanBackend::new(3);
        let tombstones = TombstoneTracker::new();

        let wrong_query = vec![0.1, 0.2]; // dimension 2, expected 3
        let result = backend.search(&wrong_query, 1, &tombstones);

        assert!(matches!(
            result,
            Err(RecError::DimensionMismatch {
                expected: 3,
                actual: 2
            })
        ));
    }

    #[test]
    fn test_scored_item_ordering() {
        let item1 = ScoredItem {
            internal_id: 1,
            score: 0.5,
        };
        let item2 = ScoredItem {
            internal_id: 2,
            score: 0.8,
        };
        let item3 = ScoredItem {
            internal_id: 3,
            score: 0.3,
        };

        let mut heap = BinaryHeap::new();
        heap.push(item1);
        heap.push(item2);
        heap.push(item3);

        // BinaryHeap is max-heap, but we want min-heap behavior
        // So the item with lowest score should come out first
        let min_item = heap.pop().unwrap();
        assert_eq!(min_item.score, 0.3);
    }
}
