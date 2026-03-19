//! Storage layer implementation using redb
//!
//! This module provides persistent storage for RecLite using redb as the underlying
//! database engine. It implements ACID transactions and handles serialization of
//! vectors and metadata.

use crate::error::RecError;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

// Table definitions
const ID_MAP_TABLE: TableDefinition<&str, u32> = TableDefinition::new("id_map");
const VECTOR_TABLE: TableDefinition<u32, &[u8]> = TableDefinition::new("vectors");
const METADATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("metadata");
const TOMBSTONE_TABLE: TableDefinition<u32, ()> = TableDefinition::new("tombstones");

/// Storage layer operations using redb
pub struct StorageLayer {
    pub db: Database,
    path: std::path::PathBuf,
}

impl StorageLayer {
    /// Open or create database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, RecError> {
        let path_buf = path.as_ref().to_path_buf();
        let db = Database::create(&path_buf)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(ID_MAP_TABLE)?;
            write_txn.open_table(VECTOR_TABLE)?;
            write_txn.open_table(METADATA_TABLE)?;
            write_txn.open_table(TOMBSTONE_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db, path: path_buf })
    }

    /// Persist a single item (two-phase commit pattern)
    pub fn upsert_item(&self, id: &str, internal_id: u32, vector: &[f32]) -> Result<(), RecError> {
        let write_txn = self.db.begin_write()?;

        {
            // Write ID mapping
            let mut id_table = write_txn.open_table(ID_MAP_TABLE)?;
            id_table.insert(id, internal_id)?;

            // Serialize and write vector
            let vector_bytes = bincode::serialize(vector)
                .map_err(|e| RecError::StorageError(format!("Serialization failed: {}", e)))?;

            let mut vector_table = write_txn.open_table(VECTOR_TABLE)?;
            vector_table.insert(internal_id, vector_bytes.as_slice())?;
        }

        // Commit atomically
        write_txn.commit()?;

        Ok(())
    }

    /// Batch upsert multiple items in a single transaction
    pub fn batch_upsert(&self, items: &[(String, u32, Vec<f32>)]) -> Result<(), RecError> {
        let write_txn = self.db.begin_write()?;

        {
            let mut id_table = write_txn.open_table(ID_MAP_TABLE)?;
            let mut vector_table = write_txn.open_table(VECTOR_TABLE)?;

            for (id, internal_id, vector) in items {
                id_table.insert(id.as_str(), *internal_id)?;

                let vector_bytes = bincode::serialize(vector)
                    .map_err(|e| RecError::StorageError(format!("Serialization failed: {}", e)))?;

                vector_table.insert(*internal_id, vector_bytes.as_slice())?;
            }
        }

        write_txn.commit()?;

        Ok(())
    }

    /// Batch upsert multiple items and clear tombstones in a single transaction
    pub fn batch_upsert_with_tombstones(
        &self,
        items: &[(String, u32, Vec<f32>)],
        tombstones_to_clear: &[u32],
    ) -> Result<(), RecError> {
        let write_txn = self.db.begin_write()?;

        {
            let mut id_table = write_txn.open_table(ID_MAP_TABLE)?;
            let mut vector_table = write_txn.open_table(VECTOR_TABLE)?;

            // Write all vectors
            for (id, internal_id, vector) in items {
                id_table.insert(id.as_str(), *internal_id)?;

                let vector_bytes = bincode::serialize(vector)
                    .map_err(|e| RecError::StorageError(format!("Serialization failed: {}", e)))?;

                vector_table.insert(*internal_id, vector_bytes.as_slice())?;
            }

            // Clear tombstones in same transaction
            let mut tombstone_table = write_txn.open_table(TOMBSTONE_TABLE)?;
            for id in tombstones_to_clear {
                tombstone_table.remove(*id)?;
            }
        }

        write_txn.commit()?;

        Ok(())
    }

    /// Load all vectors during initialization
    pub fn load_all_vectors(&self) -> Result<Vec<(u32, Vec<f32>)>, RecError> {
        let read_txn = self.db.begin_read()?;
        let vector_table = read_txn.open_table(VECTOR_TABLE)?;

        let mut vectors = Vec::new();

        for result in vector_table.iter()? {
            let (internal_id, vector_bytes) = result?;
            let vector: Vec<f32> = bincode::deserialize(vector_bytes.value())
                .map_err(|e| RecError::StorageError(format!("Deserialization failed: {}", e)))?;

            vectors.push((internal_id.value(), vector));
        }

        Ok(vectors)
    }

    /// Load all ID mappings during initialization
    pub fn load_all_mappings(&self) -> Result<Vec<(String, u32)>, RecError> {
        let read_txn = self.db.begin_read()?;
        let id_table = read_txn.open_table(ID_MAP_TABLE)?;

        let mut mappings = Vec::new();

        for result in id_table.iter()? {
            let (id, internal_id) = result?;
            mappings.push((id.value().to_string(), internal_id.value()));
        }

        Ok(mappings)
    }

    /// Store metadata (dimension, tombstones, etc.)
    pub fn store_metadata(&self, key: &str, value: &[u8]) -> Result<(), RecError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut metadata_table = write_txn.open_table(METADATA_TABLE)?;
            metadata_table.insert(key, value)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Load metadata
    pub fn load_metadata(&self, key: &str) -> Result<Option<Vec<u8>>, RecError> {
        let read_txn = self.db.begin_read()?;
        let metadata_table = read_txn.open_table(METADATA_TABLE)?;

        match metadata_table.get(key)? {
            Some(value) => Ok(Some(value.value().to_vec())),
            None => Ok(None),
        }
    }

    /// Mark an item as deleted in tombstone table
    pub fn mark_tombstone(&self, internal_id: u32) -> Result<(), RecError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut tombstone_table = write_txn.open_table(TOMBSTONE_TABLE)?;
            tombstone_table.insert(internal_id, ())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Clear tombstone marker
    pub fn clear_tombstone(&self, internal_id: u32) -> Result<(), RecError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut tombstone_table = write_txn.open_table(TOMBSTONE_TABLE)?;
            tombstone_table.remove(internal_id)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Load all tombstones during initialization
    pub fn load_all_tombstones(&self) -> Result<Vec<u32>, RecError> {
        let read_txn = self.db.begin_read()?;
        let tombstone_table = read_txn.open_table(TOMBSTONE_TABLE)?;

        let mut tombstones = Vec::new();
        for result in tombstone_table.iter()? {
            let (internal_id, _) = result?;
            tombstones.push(internal_id.value());
        }

        Ok(tombstones)
    }

    /// Get database file size
    pub fn file_size(&self) -> Result<u64, RecError> {
        // Get file size using std::fs::metadata
        use std::fs;
        match fs::metadata(&self.path) {
            Ok(metadata) => Ok(metadata.len()),
            Err(e) => Err(RecError::IoError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_storage_layer_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = StorageLayer::open(temp_file.path()).unwrap();

        // Verify database was created successfully
        // Note: file_size() implementation will be completed in later tasks
        // For now, just verify the storage layer was created without error
        assert!(storage.file_size().is_ok());
    }

    #[test]
    fn test_upsert_and_load_single_item() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = StorageLayer::open(temp_file.path()).unwrap();

        let vector = vec![0.1, 0.2, 0.3];
        storage.upsert_item("test_item", 0, &vector).unwrap();

        let vectors = storage.load_all_vectors().unwrap();
        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].0, 0);
        assert_eq!(vectors[0].1, vector);

        let mappings = storage.load_all_mappings().unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].0, "test_item");
        assert_eq!(mappings[0].1, 0);
    }

    #[test]
    fn test_metadata_storage() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = StorageLayer::open(temp_file.path()).unwrap();

        let dimension_bytes = bincode::serialize(&128usize).unwrap();
        storage
            .store_metadata("dimension", &dimension_bytes)
            .unwrap();

        let loaded = storage.load_metadata("dimension").unwrap().unwrap();
        let dimension: usize = bincode::deserialize(&loaded).unwrap();
        assert_eq!(dimension, 128);
    }

    #[test]
    fn test_tombstone_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = StorageLayer::open(temp_file.path()).unwrap();

        // Mark tombstone
        storage.mark_tombstone(42).unwrap();

        // Load tombstones
        let tombstones = storage.load_all_tombstones().unwrap();
        assert_eq!(tombstones, vec![42]);

        // Clear tombstone
        storage.clear_tombstone(42).unwrap();

        // Verify cleared
        let tombstones = storage.load_all_tombstones().unwrap();
        assert!(tombstones.is_empty());
    }
}
