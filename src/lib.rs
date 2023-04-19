//! # pmtree
//! Persistent Merkle Tree in Rust
//!
//! ## How it stored
//! { (usize::MAX - 1) : depth }
//! { (usize::MAX)     : next_index}
//! { Position (tuple - (depth, index), converted to DBKey) : Value}

pub mod database;
pub mod hasher;
pub mod tree;

use std::fmt::{Debug, Display};

pub use database::*;
pub use hasher::*;
pub use tree::MerkleTree;

/// Denotes keys in a database
pub type DBKey = [u8; 8];

/// Denotes values in a database
pub type Value = Vec<u8>;

#[derive(Debug)]
pub enum TreeErrorKind {
    MerkleTreeIsFull,
    InvalidKey,
    IndexOutOfBounds,
    UnknownError(String),
}

#[derive(Debug)]
pub enum DatabaseErrorKind {
    CannotLoadDatabase,
    DatabaseExists,
    UnknownError(String),
}

#[derive(Debug)]
pub enum PmtreeErrorKind {
    /// Error in database
    DatabaseError(DatabaseErrorKind),
    /// Error in tree
    TreeError(TreeErrorKind),
    UnknownError(String),
}

impl Display for PmtreeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PmtreeErrorKind::DatabaseError(e) => write!(f, "Database error: {e:?}"),
            PmtreeErrorKind::TreeError(e) => write!(f, "Tree error: {e:?}"),
            PmtreeErrorKind::UnknownError(e) => write!(f, "Unknown error: {e:?}"),
        }
    }
}

impl std::error::Error for PmtreeErrorKind {}

/// Custom `Result` type with custom `Error` type
pub type PmtreeResult<T> = std::result::Result<T, PmtreeErrorKind>;
