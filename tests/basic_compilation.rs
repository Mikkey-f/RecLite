//! Basic compilation and functionality tests
//!
//! This test ensures that all modules compile correctly and basic
//! functionality works as expected.

use reclite::{RecError, RecStats, SearchResult};

#[test]
fn test_search_result_creation() {
    let result = SearchResult::new("test_id".to_string(), 0.95);
    assert_eq!(result.id, "test_id");
    assert_eq!(result.score, 0.95);
}

#[test]
fn test_rec_stats_creation() {
    let stats = RecStats {
        item_count: 100,
        tombstone_count: 5,
        dimension: 128,
        file_size: 1024,
    };

    assert_eq!(stats.item_count, 100);
    assert_eq!(stats.tombstone_count, 5);
    assert_eq!(stats.dimension, 128);
    assert_eq!(stats.file_size, 1024);
}

#[test]
fn test_error_display() {
    let error = RecError::DimensionMismatch {
        expected: 128,
        actual: 64,
    };
    let error_string = format!("{}", error);
    assert!(error_string.contains("128"));
    assert!(error_string.contains("64"));
}
