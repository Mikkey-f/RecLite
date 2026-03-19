//! Error types for RecLite operations
//!
//! This module defines the comprehensive error enumeration covering all failure modes
//! in RecLite operations, with proper error context and conversion traits.

use std::fmt;

/// Comprehensive error type for all RecLite operations
#[derive(Debug)]
pub enum RecError {
    /// Vector dimension doesn't match engine dimension
    ///
    /// Contains: (expected_dimension, actual_dimension)
    DimensionMismatch { expected: usize, actual: usize },

    /// Item ID not found in database
    ///
    /// Contains: the missing ID
    NotFound(String),

    /// File system I/O error
    ///
    /// Contains: underlying std::io::Error
    IoError(std::io::Error),

    /// Invalid user input
    ///
    /// Contains: description of the validation failure
    InvalidInput(String),

    /// Database storage/transaction error
    ///
    /// Contains: underlying redb error details
    StorageError(String),
}

impl fmt::Display for RecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecError::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "Dimension mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            RecError::NotFound(id) => {
                write!(f, "Item not found: {}", id)
            }
            RecError::IoError(err) => {
                write!(f, "I/O error: {}", err)
            }
            RecError::InvalidInput(msg) => {
                write!(f, "Invalid input: {}", msg)
            }
            RecError::StorageError(msg) => {
                write!(f, "Storage error: {}", msg)
            }
        }
    }
}

impl std::error::Error for RecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RecError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

// Conversion from std::io::Error
impl From<std::io::Error> for RecError {
    fn from(err: std::io::Error) -> Self {
        RecError::IoError(err)
    }
}

// Conversion from redb errors
impl From<redb::Error> for RecError {
    fn from(err: redb::Error) -> Self {
        RecError::StorageError(format!("redb error: {}", err))
    }
}

impl From<redb::TransactionError> for RecError {
    fn from(err: redb::TransactionError) -> Self {
        RecError::StorageError(format!("redb transaction error: {}", err))
    }
}

impl From<redb::StorageError> for RecError {
    fn from(err: redb::StorageError) -> Self {
        RecError::StorageError(format!("redb storage error: {}", err))
    }
}

// Additional redb error conversions
impl From<redb::DatabaseError> for RecError {
    fn from(err: redb::DatabaseError) -> Self {
        RecError::StorageError(format!("redb database error: {}", err))
    }
}

impl From<redb::TableError> for RecError {
    fn from(err: redb::TableError) -> Self {
        RecError::StorageError(format!("redb table error: {}", err))
    }
}

impl From<redb::CommitError> for RecError {
    fn from(err: redb::CommitError) -> Self {
        RecError::StorageError(format!("redb commit error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_dimension_mismatch_display() {
        let error = RecError::DimensionMismatch {
            expected: 128,
            actual: 64,
        };
        assert_eq!(
            error.to_string(),
            "Dimension mismatch: expected 128, got 64"
        );
    }

    #[test]
    fn test_not_found_display() {
        let error = RecError::NotFound("item123".to_string());
        assert_eq!(error.to_string(), "Item not found: item123");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let rec_err = RecError::from(io_err);

        match rec_err {
            RecError::IoError(_) => (),
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_invalid_input_display() {
        let error = RecError::InvalidInput("Vector cannot be empty".to_string());
        assert_eq!(error.to_string(), "Invalid input: Vector cannot be empty");
    }

    #[test]
    fn test_storage_error_display() {
        let error = RecError::StorageError("Transaction failed".to_string());
        assert_eq!(error.to_string(), "Storage error: Transaction failed");
    }

    #[test]
    fn test_error_source() {
        use std::error::Error;

        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let rec_err = RecError::from(io_err);

        assert!(rec_err.source().is_some());

        let dimension_err = RecError::DimensionMismatch {
            expected: 10,
            actual: 5,
        };
        assert!(dimension_err.source().is_none());
    }
}
