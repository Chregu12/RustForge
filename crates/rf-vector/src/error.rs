//! Error types for `rf-vector`.

use thiserror::Error;

/// Errors produced by vector operations.
#[derive(Debug, Error)]
pub enum VectorError {
    /// Two vectors that must match in length did not.
    #[error("dimension mismatch: {left} vs {right}")]
    DimensionMismatch {
        /// Dimension of the left-hand vector.
        left: usize,
        /// Dimension of the right-hand vector.
        right: usize,
    },

    /// An operation that requires a non-empty vector received an empty one.
    #[error("empty vector")]
    Empty,
}

/// Convenience result alias for vector operations.
pub type VectorResult<T> = Result<T, VectorError>;
