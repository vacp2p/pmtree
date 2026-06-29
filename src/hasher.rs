use std::fmt::Debug;

use crate::{PmtreeResult, Value};

/// Trait that must be implemented for the hash function.
pub trait Hasher {
    /// Native type for a leaf and tree node.
    type Fr: Copy + Eq + Default + Sync + Send + Debug;

    /// Serializes an `Fr` into its stored [`Value`].
    fn serialize(value: Self::Fr) -> PmtreeResult<Value>;

    /// Deserializes an `Fr` from its stored bytes.
    fn deserialize(bytes: &[u8]) -> PmtreeResult<Self::Fr>;

    /// Outputs the default leaf (`Fr::default()`).
    fn default_leaf() -> Self::Fr {
        Self::Fr::default()
    }

    /// Combines two child nodes into their parent.
    fn hash_pair(left: Self::Fr, right: Self::Fr) -> Self::Fr;
}
