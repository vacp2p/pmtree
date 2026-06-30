use std::collections::HashMap;

use crate::{DBKey, PmtreeResult, Value};

/// Storage backend that persists the [`MerkleTree`](crate::MerkleTree)'s nodes.
///
/// Each tree is parameterized over a `Database`; it is the durable key/value store
/// holding every serialized node plus the tree's depth and next index.
pub trait Database {
    /// Configuration for the database; its `Default` backs [`MerkleTree::default`](crate::MerkleTree::default).
    type Config: Default;

    /// Creates a new database instance.
    fn new(config: Self::Config) -> PmtreeResult<Self>
    where
        Self: Sized;

    /// Loads an existing database (existence check required).
    fn load(config: Self::Config) -> PmtreeResult<Self>
    where
        Self: Sized;

    /// Returns the [`Value`] stored at `key`.
    fn get(&self, key: DBKey) -> PmtreeResult<Option<Value>>;

    /// Puts `value` at `key`.
    fn put(&mut self, key: DBKey, value: Value) -> PmtreeResult<()>;

    /// Atomically puts a batch of [`DBKey`]/[`Value`] entries into the database.
    fn put_batch(&mut self, subtree: HashMap<DBKey, Value>) -> PmtreeResult<()>;

    /// Closes the database connection.
    fn close(&mut self) -> PmtreeResult<()>;
}
