use crate::{
    error::Error,
    tree::{BranchNode, LeafNode},
    Hash, H256,
};
use core::ops::{Deref, DerefMut};

/// Trait for customize hash function
pub trait Hasher {
    fn write_bytes(&mut self, h: &[u8]);
    fn finish(self) -> H256;
    fn hash_op() -> ics23::HashOp {
        ics23::HashOp::NoHash
    }
}

/// Trait for key values
pub trait Key: Clone + Default + Deref<Target = H256> {
    fn write_bytes<H: Hasher>(&self, hasher: &mut H);
    /*fn write_bytes<W: Writer>(&self, buf: &mut W);
    fn write_to_hasher<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_bytes(self.to_vec().as_slice());
    }
    fn to_vec(&self) -> Vec<u8> {
        let mut buf = vec![];
        self.write_bytes(&mut buf);
        buf
    }*/
    fn to_vec(&self) -> Vec<u8>;
    fn is_equal(&self, other: &Self) -> bool {
        let hash_self = <Self as Deref>::deref(self);
        let hash_other = <Self as Deref>::deref(other);
        hash_self == hash_other
    }
}

impl Deref for Hash {
    type Target = H256;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Key for Hash {
    fn write_bytes<H: Hasher>(&self, hasher: &mut H) {
        hasher.write_bytes(self.as_slice());
    }

    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

/// Trait for defining value structures
pub trait Value {
    fn to_h256(&self) -> H256;
    fn zero() -> Self;
}

impl Value for H256 {
    fn to_h256(&self) -> H256 {
        *self
    }
    fn zero() -> Self {
        H256::zero()
    }
}

/// Trait for customize backend storage
pub trait Store<K: Key, V: Value> {
    fn get_branch(&self, node: &H256) -> Result<Option<BranchNode<K>>, Error>;
    fn get_leaf(&self, leaf_hash: &H256) -> Result<Option<LeafNode<K, V>>, Error>;
    fn insert_branch(&mut self, node: H256, branch: BranchNode<K>) -> Result<(), Error>;
    fn insert_leaf(&mut self, leaf_hash: H256, leaf: LeafNode<K, V>) -> Result<(), Error>;
    fn remove_branch(&mut self, node: &H256) -> Result<(), Error>;
    fn remove_leaf(&mut self, leaf_hash: &H256) -> Result<(), Error>;
}
