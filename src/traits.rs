use crate::{
    error::Error,
    tree::{BranchNode, LeafNode},
    Hash as KeyHash, InternalKey, H256,
};
use core::hash::Hash;
use core::ops::Deref;

/// Trait for customize hash function
pub trait Hasher {
    fn write_bytes(&mut self, h: &[u8]);
    fn finish(self) -> H256;
    fn hash_op() -> ics23::HashOp {
        ics23::HashOp::NoHash
    }
}

/// This trait is map keys to / from the users key space into a finite
/// key space used internally. This space is the set of all N-byte arrays
/// where N < 2^32
pub trait Key<const N: usize>:
    Eq + PartialEq + Copy + Clone + Hash + Deref<Target = InternalKey<N>>
{
    /// The error type for failed mappings
    type Error;
    /// This should map from the internal key space
    /// back into the user's key space
    fn as_slice(&self) -> &[u8];
    /// This should map from the internal key space
    /// back into the user's key space
    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
    /// This should map from the user's key space into
    /// the internal keyspace
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
}

impl Key<32> for KeyHash {
    type Error = crate::error::Error;

    fn as_slice(&self) -> &[u8] {
        <Self as Deref>::deref(self).as_slice()
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Self::Error> {
        use std::convert::TryInto;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| crate::error::Error::KeyTooLarge)?;
        Ok(bytes.into())
    }
}

/// Trait for define value structures
pub trait Value: PartialEq + Clone {
    fn as_slice(&self) -> &[u8];
    fn zero() -> Self;
    fn is_zero(&self) -> bool {
        self == &Self::zero()
    }
}

impl Value for H256 {
    fn as_slice(&self) -> &[u8] {
        self.as_slice()
    }
    fn zero() -> Self {
        H256::zero()
    }
}

/// Trait for customize backend storage
pub trait Store<K, V, const N: usize>: Default
where
    K: Key<N>,
{
    fn get_branch(&self, node: &H256) -> Result<Option<BranchNode<K, N>>, Error>;
    fn get_leaf(&self, leaf_key: &H256) -> Result<Option<LeafNode<K, V, N>>, Error>;
    fn insert_branch(&mut self, node: H256, branch: BranchNode<K, N>) -> Result<(), Error>;
    fn insert_leaf(&mut self, leaf_key: H256, leaf: LeafNode<K, V, N>) -> Result<(), Error>;
    fn remove_branch(&mut self, node: &H256) -> Result<(), Error>;
    fn remove_leaf(&mut self, leaf_key: &H256) -> Result<(), Error>;
}
