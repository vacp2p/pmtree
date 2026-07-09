//! # pmtree
//! Persistent Merkle Tree in Rust
//!
//! ## How it is stored
//! - `u64::MAX - 1` → `depth`
//! - `u64::MAX` → `next_index`
//! - a node position `(depth, index)` (converted to a [`DBKey`]) → its [`Value`]

pub mod database;
pub mod error;
pub mod hasher;
pub mod tree;

pub use database::Database;
pub use error::{PmtreeError, PmtreeResult};
pub use hasher::Hasher;
pub use tree::{MerkleTree, MAX_DEPTH};

/// Denotes keys in a [`Database`].
pub type DBKey = [u8; 8];

/// Denotes values in a [`Database`].
pub type Value = Vec<u8>;
