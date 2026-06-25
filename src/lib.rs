//! # pmtree
//! Persistent Merkle Tree in Rust
//!
//! ## How it is stored
//! - `u64::MAX - 1` → `depth`
//! - `u64::MAX` → `next_index`
//! - a node position `(depth, index)` (converted to a [`DBKey`]) → its [`Value`]

pub mod database;
pub mod hasher;
pub mod tree;

pub use database::*;
pub use hasher::*;
pub use tree::MerkleTree;

/// Denotes keys in a [`Database`].
pub type DBKey = [u8; 8];

/// Denotes values in a [`Database`].
pub type Value = Vec<u8>;

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
    /// A recompute lock was poisoned by a panicking worker thread.
    #[error("Recompute lock poisoned")]
    LockPoisoned,
    /// An error surfaced by a [`Database`] implementation.
    #[error("Database error: {0}")]
    Database(String),
    /// An error surfaced by a [`Hasher`] implementation.
    #[error("Hasher error: {0}")]
    Hasher(String),
}

// `PoisonError<T>` is generic so can't use `#[from]` by thiserror.
impl<T> From<std::sync::PoisonError<T>> for PmtreeError {
    fn from(_: std::sync::PoisonError<T>) -> Self {
        PmtreeError::LockPoisoned
    }
}

/// Custom [`Result`] type carrying the crate's [`PmtreeError`].
pub type PmtreeResult<T> = std::result::Result<T, PmtreeError>;
