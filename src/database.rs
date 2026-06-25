use std::collections::HashMap;

use crate::*;

/// Trait that must be implemented for a database backend.
pub trait Database {
    /// Configuration for the database; its `Default` backs [`MerkleTree::default`].
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

    /// Puts a batch of [`DBKey`]/[`Value`] entries into the database.
    fn put_batch(&mut self, subtree: HashMap<DBKey, Value>) -> PmtreeResult<()>;

    /// Closes the database connection.
    fn close(&mut self) -> PmtreeResult<()>;
}
