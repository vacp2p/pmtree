use std::fmt::Debug;

use crate::*;

/// Trait that must be implemented for the hash function.
pub trait Hasher {
    /// Native type for the hash function.
    type Fr: Copy + Eq + Default + Sync + Send + Debug;

    /// Serializes `Fr` into its stored byte representation.
    fn serialize(value: Self::Fr) -> PmtreeResult<Value>;

    /// Deserializes `Fr` from its stored byte representation. Fails on malformed bytes.
    fn deserialize(bytes: &[u8]) -> PmtreeResult<Self::Fr>;

    /// Outputs the default leaf (`Fr::default()`).
    fn default_leaf() -> Self::Fr {
        Self::Fr::default()
    }

    /// Hashes a pair of nodes into their parent.
    fn hash_pair(left: Self::Fr, right: Self::Fr) -> Self::Fr;
}
