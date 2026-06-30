/// Errors returned by pmtree operations.
#[derive(Debug, thiserror::Error)]
pub enum PmtreeError {
    /// The tree has no free leaves left.
    #[error("Merkle tree is full")]
    TreeIsFull,
    /// The index is out of bounds, or the leaf at it is not set.
    #[error("Index out of bounds")]
    IndexOutOfBounds,
    /// The requested tree depth exceeds the supported maximum.
    #[error("Tree depth {0} exceeds the supported maximum")]
    DepthTooLarge(usize),
    /// The stored tree is missing required data or is otherwise inconsistent.
    #[error("Corrupted tree storage")]
    Corrupted,
    /// A stored value could not be parsed back.
    #[error("Malformed stored value: {0}")]
    Malformed(#[from] std::array::TryFromSliceError),
    /// An error surfaced by a [`Database`](crate::Database) implementation.
    #[error("Database error: {0}")]
    Database(String),
    /// An error surfaced by a [`Hasher`](crate::Hasher) implementation.
    #[error("Hasher error: {0}")]
    Hasher(String),
}

/// Custom [`Result`] type carrying the crate's [`PmtreeError`].
pub type PmtreeResult<T> = std::result::Result<T, PmtreeError>;
