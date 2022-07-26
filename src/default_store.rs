use crate::{
    collections,
    error::Error,
    traits::{Key, Store, Value},
    tree::{BranchNode, LeafNode},
    H256,
};
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct DefaultStore<K: Key, V: Value> {
    branches_map: Map<H256, BranchNode<K>>,
    leaves_map: Map<H256, LeafNode<K, V>>,
}

impl<K: Key, V: Value> DefaultStore<K, V> {
    pub fn branches_map(&self) -> &Map<H256, BranchNode<K>> {
        &self.branches_map
    }
    pub fn leaves_map(&self) -> &Map<H256, LeafNode<K, V>> {
        &self.leaves_map
    }
    pub fn clear(&mut self) {
        self.branches_map.clear();
        self.leaves_map.clear();
    }
}

impl<K: Key, V: Clone + Value> Store<K, V> for DefaultStore<K, V> {
    fn get_branch(&self, node: &H256) -> Result<Option<BranchNode<K>>, Error> {
        Ok(self.branches_map.get(node).map(Clone::clone))
    }
    fn get_leaf(&self, leaf_hash: &H256) -> Result<Option<LeafNode<K, V>>, Error> {
        Ok(self.leaves_map.get(leaf_hash).map(Clone::clone))
    }
    fn insert_branch(&mut self, node: H256, branch: BranchNode<K>) -> Result<(), Error> {
        self.branches_map.insert(node, branch);
        Ok(())
    }
    fn insert_leaf(&mut self, leaf_hash: H256, leaf: LeafNode<K, V>) -> Result<(), Error> {
        self.leaves_map.insert(leaf_hash, leaf);
        Ok(())
    }
    fn remove_branch(&mut self, node: &H256) -> Result<(), Error> {
        self.branches_map.remove(node);
        Ok(())
    }
    fn remove_leaf(&mut self, leaf_hash: &H256) -> Result<(), Error> {
        self.leaves_map.remove(leaf_hash);
        Ok(())
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        pub type Map<K, V> = collections::HashMap<K, V>;
        pub type Entry<'a, K, V> = collections::hash_map::Entry<'a, K, V>;
    } else {
        pub type Map<K, V> = collections::BTreeMap<K, V>;
        pub type Entry<'a, K, V> = collections::btree_map::Entry<'a, K, V>;
    }
}
