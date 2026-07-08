use std::fmt::Debug;

use crate::{PmtreeResult, Value};

/// Trait that must be implemented for the hash function.
pub trait Hasher {
    /// Native type for a leaf and tree node.
    type Scalar: Debug + Copy + PartialEq + Default + Send + Sync;

    /// Serializes a `Scalar` into its stored [`Value`].
    fn serialize(value: Self::Scalar) -> PmtreeResult<Value>;

    /// Deserializes a `Scalar` from its stored bytes.
    fn deserialize(bytes: &[u8]) -> PmtreeResult<Self::Scalar>;

    /// Outputs the default leaf (`Scalar::default()`).
    fn default_leaf() -> Self::Scalar {
        Self::Scalar::default()
    }

    /// Combines two child nodes into their parent.
    fn hash_pair(left: Self::Scalar, right: Self::Scalar) -> Self::Scalar;
}
