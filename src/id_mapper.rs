//! ID mapping component for bi-directional String ↔ u32 conversion
//!
//! This module provides thread-safe mapping between human-readable string identifiers
//! and compact u32 internal IDs used for efficient vector storage and indexing.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

/// Bi-directional ID mapper with atomic allocation
pub struct IDMapper {
    // String → u32 mapping
    string_to_id: HashMap<String, u32>,
    
    // u32 → String mapping
    id_to_string: HashMap<u32, String>,
    
    // Next available internal ID
    next_id: AtomicU32,
}

impl IDMapper {
    /// Create a new empty ID mapper
    pub fn new() -> Self {
        Self {
            string_to_id: HashMap::new(),
            id_to_string: HashMap::new(),
            next_id: AtomicU32::new(0),
        }
    }

    /// Get or allocate an internal ID for a string identifier
    ///
    /// Thread-safe: uses atomic counter for ID allocation
    pub fn get_or_allocate(&mut self, id: &str) -> u32 {
        if let Some(&internal_id) = self.string_to_id.get(id) {
            return internal_id;
        }

        // Allocate new ID atomically
        let internal_id = self.next_id.fetch_add(1, Ordering::SeqCst);
        
        self.string_to_id.insert(id.to_string(), internal_id);
        self.id_to_string.insert(internal_id, id.to_string());
        
        internal_id
    }

    /// Lookup string ID from internal ID
    pub fn get_string(&self, internal_id: u32) -> Option<&str> {
        self.id_to_string.get(&internal_id).map(|s| s.as_str())
    }

    /// Lookup internal ID from string ID
    pub fn get_internal(&self, id: &str) -> Option<u32> {
        self.string_to_id.get(id).copied()
    }

    /// Load mappings from storage during initialization
    pub fn load_from_storage(&mut self, mappings: Vec<(String, u32)>) {
        let mut max_id = 0u32;
        
        for (string_id, internal_id) in mappings {
            self.string_to_id.insert(string_id.clone(), internal_id);
            self.id_to_string.insert(internal_id, string_id);
            max_id = max_id.max(internal_id);
        }
        
        // Set next_id to one past the maximum loaded ID
        self.next_id.store(max_id + 1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_or_allocate_new_id() {
        let mut mapper = IDMapper::new();
        
        let id1 = mapper.get_or_allocate("item1");
        let id2 = mapper.get_or_allocate("item2");
        
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_get_or_allocate_idempotent() {
        let mut mapper = IDMapper::new();
        
        let id1 = mapper.get_or_allocate("item1");
        let id2 = mapper.get_or_allocate("item1");
        
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_round_trip_mapping() {
        let mut mapper = IDMapper::new();
        
        let original_string = "test_item";
        let internal_id = mapper.get_or_allocate(original_string);
        let retrieved_string = mapper.get_string(internal_id).unwrap();
        
        assert_eq!(original_string, retrieved_string);
    }

    #[test]
    fn test_get_internal_lookup() {
        let mut mapper = IDMapper::new();
        
        let internal_id = mapper.get_or_allocate("item1");
        let looked_up_id = mapper.get_internal("item1").unwrap();
        
        assert_eq!(internal_id, looked_up_id);
    }

    #[test]
    fn test_get_string_nonexistent() {
        let mapper = IDMapper::new();
        assert!(mapper.get_string(999).is_none());
    }

    #[test]
    fn test_get_internal_nonexistent() {
        let mapper = IDMapper::new();
        assert!(mapper.get_internal("nonexistent").is_none());
    }

    #[test]
    fn test_load_from_storage() {
        let mut mapper = IDMapper::new();
        
        let mappings = vec![
            ("item1".to_string(), 5),
            ("item2".to_string(), 10),
            ("item3".to_string(), 3),
        ];
        
        mapper.load_from_storage(mappings);
        
        // Verify mappings were loaded
        assert_eq!(mapper.get_internal("item1"), Some(5));
        assert_eq!(mapper.get_internal("item2"), Some(10));
        assert_eq!(mapper.get_internal("item3"), Some(3));
        
        assert_eq!(mapper.get_string(5), Some("item1"));
        assert_eq!(mapper.get_string(10), Some("item2"));
        assert_eq!(mapper.get_string(3), Some("item3"));
        
        // Verify next_id is set correctly (max + 1)
        let new_id = mapper.get_or_allocate("new_item");
        assert_eq!(new_id, 11);
    }
}