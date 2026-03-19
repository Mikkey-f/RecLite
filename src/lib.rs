//! # RecLite - The SQLite for Recommendation Systems
//!
//! RecLite is an embedded, zero-configuration recommendation engine for Rust that provides
//! high-performance vector similarity search through SIMD-accelerated linear scan.
//!
//! ## Features
//!
//! - **Zero Configuration**: Single function call initialization with file path
//! - **Embedded Architecture**: No daemon processes, all operations in-process
//! - **ACID Guarantees**: Persistent storage via redb with transaction safety
//! - **Concurrent Access**: RwLock-based coordination for multiple readers, single writer
//! - **SIMD Acceleration**: High-performance vector similarity search using simsimd
//! - **Extensibility**: Pluggable search backend abstraction for future scaling
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use reclite::{RecEngine, RecError};
//!
//! # fn main() -> Result<(), RecError> {
//! // Open or create a database
//! let engine = RecEngine::open("recommendations.db")?;
//!
//! // Add some items with their embedding vectors
//! engine.upsert("item1".to_string(), vec![0.1, 0.2, 0.3])?;
//! engine.upsert("item2".to_string(), vec![0.4, 0.5, 0.6])?;
//! engine.upsert("item3".to_string(), vec![0.7, 0.8, 0.9])?;
//!
//! // Search for similar items
//! let results = engine.search(vec![0.1, 0.2, 0.3], 2)?;
//! for result in results {
//!     println!("{}: {:.3}", result.id, result.score);
//! }
//!
//! // Clean shutdown
//! engine.close()?;
//! # Ok(())
//! # }
//! ```

// Core modules
pub mod error;
pub mod storage;
pub mod id_mapper;
pub mod vector_index;
pub mod tombstone;
pub mod backend;
pub mod engine;

// Python bindings (optional)
#[cfg(feature = "python")]
pub mod python;

// Re-export public API
pub use engine::RecEngine;
pub use error::RecError;

/// A single search result containing item ID and similarity score
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// External string identifier for the item
    pub id: String,
    
    /// Cosine similarity score (range: 0.0 to 1.0)
    /// Higher scores indicate greater similarity
    pub score: f32,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(id: String, score: f32) -> Self {
        Self { id, score }
    }
}

/// Database statistics and metadata
#[derive(Debug, Clone)]
pub struct RecStats {
    /// Total number of active items (excluding tombstoned)
    pub item_count: u32,
    
    /// Number of deleted (tombstoned) items
    pub tombstone_count: u32,
    
    /// Embedding vector dimension
    pub dimension: usize,
    
    /// Database file size in bytes
    pub file_size: u64,
}