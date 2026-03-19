//! Flat vector index for in-memory storage
//!
//! This module provides contiguous flat array storage for fast SIMD access,
//! optimized for efficient vector operations and memory layout.

use crate::error::RecError;

/// Flat in-memory vector storage optimized for SIMD operations
pub struct FlatVectorIndex {
    // Flat array: [vec0_dim0, vec0_dim1, ..., vec1_dim0, vec1_dim1, ...]
    data: Vec<f32>,
    
    // Fixed dimension for all vectors
    dimension: usize,
    
    // Number of vectors currently stored
    count: usize,
}

impl FlatVectorIndex {
    /// Create a new vector index with specified dimension
    pub fn new(dimension: usize) -> Self {
        Self {
            data: Vec::new(),
            dimension,
            count: 0,
        }
    }

    /// Preallocate capacity for expected number of vectors
    pub fn with_capacity(dimension: usize, capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(dimension * capacity),
            dimension,
            count: 0,
        }
    }

    /// Add a new vector (append to end)
    pub fn push(&mut self, vector: &[f32]) -> Result<u32, RecError> {
        if vector.len() != self.dimension {
            return Err(RecError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let internal_id = self.count as u32;
        self.data.extend_from_slice(vector);
        self.count += 1;
        
        Ok(internal_id)
    }

    /// Update an existing vector at a specific internal ID
    pub fn update(&mut self, internal_id: u32, vector: &[f32]) -> Result<(), RecError> {
        if vector.len() != self.dimension {
            return Err(RecError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }

        let idx = internal_id as usize;
        if idx >= self.count {
            return Err(RecError::NotFound(format!("Internal ID {} out of bounds", internal_id)));
        }

        let start = idx * self.dimension;
        let end = start + self.dimension;
        self.data[start..end].copy_from_slice(vector);
        
        Ok(())
    }

    /// Get a vector by internal ID
    pub fn get(&self, internal_id: u32) -> Option<&[f32]> {
        let idx = internal_id as usize;
        if idx >= self.count {
            return None;
        }

        let start = idx * self.dimension;
        let end = start + self.dimension;
        Some(&self.data[start..end])
    }
    /// Get raw data slice for SIMD operations
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    /// Get number of vectors
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get the dimension of vectors in this index
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Reserve additional capacity to minimize reallocations
    ///
    /// Uses 1.5x growth factor for balance between memory and performance
    pub fn reserve(&mut self, additional: usize) {
        let new_capacity = (self.count + additional) * self.dimension;
        self.data.reserve(new_capacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_vector_index() {
        let index = FlatVectorIndex::new(3);
        assert_eq!(index.dimension(), 3);
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
    }

    #[test]
    fn test_with_capacity() {
        let index = FlatVectorIndex::with_capacity(4, 100);
        assert_eq!(index.dimension(), 4);
        assert_eq!(index.len(), 0);
        assert!(index.data.capacity() >= 400); // 4 * 100
    }

    #[test]
    fn test_push_vector() {
        let mut index = FlatVectorIndex::new(3);
        
        let vector1 = vec![0.1, 0.2, 0.3];
        let vector2 = vec![0.4, 0.5, 0.6];
        
        let id1 = index.push(&vector1).unwrap();
        let id2 = index.push(&vector2).unwrap();
        
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(index.len(), 2);
        
        // Verify data layout
        assert_eq!(index.as_slice(), &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
    }

    #[test]
    fn test_push_wrong_dimension() {
        let mut index = FlatVectorIndex::new(3);
        let wrong_vector = vec![0.1, 0.2]; // dimension 2, expected 3
        
        let result = index.push(&wrong_vector);
        assert!(matches!(result, Err(RecError::DimensionMismatch { expected: 3, actual: 2 })));
    }

    #[test]
    fn test_get_vector() {
        let mut index = FlatVectorIndex::new(2);
        
        let vector1 = vec![1.0, 2.0];
        let vector2 = vec![3.0, 4.0];
        
        index.push(&vector1).unwrap();
        index.push(&vector2).unwrap();
        
        assert_eq!(index.get(0), Some([1.0, 2.0].as_slice()));
        assert_eq!(index.get(1), Some([3.0, 4.0].as_slice()));
        assert_eq!(index.get(2), None);
    }

    #[test]
    fn test_update_vector() {
        let mut index = FlatVectorIndex::new(2);
        
        let original = vec![1.0, 2.0];
        let updated = vec![5.0, 6.0];
        
        let id = index.push(&original).unwrap();
        index.update(id, &updated).unwrap();
        
        assert_eq!(index.get(id), Some([5.0, 6.0].as_slice()));
    }

    #[test]
    fn test_update_wrong_dimension() {
        let mut index = FlatVectorIndex::new(3);
        
        let vector = vec![1.0, 2.0, 3.0];
        let id = index.push(&vector).unwrap();
        
        let wrong_update = vec![4.0, 5.0]; // dimension 2, expected 3
        let result = index.update(id, &wrong_update);
        
        assert!(matches!(result, Err(RecError::DimensionMismatch { expected: 3, actual: 2 })));
    }

    #[test]
    fn test_update_out_of_bounds() {
        let mut index = FlatVectorIndex::new(2);
        
        let vector = vec![1.0, 2.0];
        let result = index.update(999, &vector);
        
        assert!(matches!(result, Err(RecError::NotFound(_))));
    }

    #[test]
    fn test_reserve_capacity() {
        let mut index = FlatVectorIndex::new(4);
        let initial_capacity = index.data.capacity();
        
        index.reserve(100);
        
        assert!(index.data.capacity() >= initial_capacity + 400); // 100 * 4
    }
}