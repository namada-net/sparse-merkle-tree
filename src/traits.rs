use crate::{
    error::Error,
    tree::{BranchNode, LeafNode},
    Key, H256,
};

/// Trait for customize hash function
pub trait Hasher {
    fn write_bytes(&mut self, h: &[u8]);
    fn finish(self) -> H256;
    fn hash_op() -> ics23::HashOp {
        ics23::HashOp::NoHash
    }
}

/// Trait for define value structures
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
