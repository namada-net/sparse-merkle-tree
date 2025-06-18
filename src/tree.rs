use crate::{
    collections::{BTreeMap, VecDeque},
    error::{Error, Result},
    merge::{hash_leaf, merge},
    merkle_proof::MerkleProof,
    proof_ics23,
    traits::{Hasher, Store, Value},
    vec::Vec,
    Key, InternalKey, EXPECTED_PATH_SIZE, H256,
};
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use core::{cmp::max, marker::PhantomData};
use ics23::commitment_proof::Proof;
use ics23::{CommitmentProof, NonExistenceProof};

/// A branch in the SMT
#[derive(Debug, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct BranchNode<K, const N: usize>
where
    K: Key<N>,
{
    pub fork_height: usize,
    pub key: K,
    pub node: H256,
    pub sibling: H256,
}

impl<K, const N: usize> BranchNode<K, N>
where
    K: Key<N>,
{
    fn branch(&self, height: usize) -> (&H256, &H256) {
        let is_right = self.key.get_bit(height);
        if is_right {
            (&self.sibling, &self.node)
        } else {
            (&self.node, &self.sibling)
        }
    }
}

/// A leaf in the SMT
#[derive(Debug, Eq, PartialEq, Clone)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
pub struct LeafNode<K, V, const N: usize>
where
    K: Key<N>,
{
    pub key: K,
    pub value: V,
}

/// Sparse merkle tree
#[derive(Debug)]
pub struct SparseMerkleTree<H, K, V, S, const N: usize>
where
    H: Hasher + Default,
    K: Key<N>,
    V: Value,
    S: Store<K, V, N>,
{
    store: S,
    root: H256,
    phantom: PhantomData<(H, K, V)>,
}

impl<H, K, V, S, const N: usize> Default for SparseMerkleTree<H, K, V, S, N>
where
    H: Hasher + Default,
    K: Key<N>,
    V: Value + core::cmp::PartialEq,
    S: Store<K, V, N>,
{
    fn default() -> Self {
        Self::new(H256::default(), S::default())
    }
}

impl<H, K, V, S, const N: usize> SparseMerkleTree<H, K, V, S, N>
where
    H: Hasher + Default,
    K: Key<N>,
    V: Value + core::cmp::PartialEq,
    S: Store<K, V, N>,
{
    /// Build a merkle tree from root and store
    pub fn new(root: H256, store: S) -> SparseMerkleTree<H, K, V, S, N> {
        SparseMerkleTree {
            root,
            store,
            phantom: PhantomData,
        }
    }

    /// Merkle root
    pub fn root(&self) -> &H256 {
        &self.root
    }

    /// Check empty of the tree
    pub fn is_empty(&self) -> bool {
        self.root.is_zero()
    }

    /// Destroy current tree and retake store
    pub fn take_store(self) -> S {
        self.store
    }

    /// Get backend store
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Get mutable backend store
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Update a leaf, return new merkle root
    /// set to zero value to delete a key
    pub fn update(&mut self, key: K, value: V) -> Result<&H256> {
        // store the path, sparse index will ignore zero members
        let mut path: BTreeMap<_, _> = Default::default();
        // walk path from root to leaf
        let mut node = self.root;
        let mut branch = self.store.get_branch(&node)?;
        let mut height = branch
            .as_ref()
            .map(|b| max(b.key.fork_height(&key), b.fork_height))
            .unwrap_or(0);
        // branch.is_none() represents the descendants are zeros, so we can stop the
        // loop
        while branch.is_some() {
            let branch_node = branch.unwrap();
            let fork_height = max(key.fork_height(&branch_node.key), branch_node.fork_height);
            if height > branch_node.fork_height {
                // the merge height is higher than node, so we do not need to remove node's
                // branch
                path.insert(fork_height, node);
                break;
            }
            // branch node is parent if height is less than branch_node's height
            // remove it from store
            if branch_node.fork_height > 0 {
                self.store.remove_branch(&node)?;
            }
            let (left, right) = branch_node.branch(height);
            let is_right = key.get_bit(height);
            let sibling = if is_right {
                if &node == right {
                    break;
                }
                node = *right;
                *left
            } else {
                if &node == left {
                    break;
                }
                node = *left;
                *right
            };
            path.insert(height, sibling);
            // get next branch and fork_height
            branch = self.store.get_branch(&node)?;
            if let Some(branch_node) = branch.as_ref() {
                height = max(key.fork_height(&branch_node.key), branch_node.fork_height);
            }
        }
        // delete previous leaf
        if let Some(leaf) = self.store.get_leaf(&node)? {
            if leaf.key == key {
                self.store.remove_leaf(&node)?;
                self.store.remove_branch(&node)?;
            }
        }

        // compute and store new leaf
        let mut node = hash_leaf::<H, K, V, N>(&key, &value);
        // notice when value is zero the leaf is deleted, so we do not need to store it
        if !node.is_zero() {
            self.store.insert_leaf(node, LeafNode { key, value })?;

            // build at least one branch for leaf
            self.store.insert_branch(
                node,
                BranchNode {
                    key,
                    fork_height: 0,
                    node,
                    sibling: H256::zero(),
                },
            )?;
        }

        // recompute the tree from top to bottom
        while !path.is_empty() {
            // pop from path
            let height = path.iter().next().map(|(height, _)| *height).unwrap();
            let sibling = path.remove(&height).unwrap();

            let is_right = key.get_bit(height);
            let parent = if is_right {
                merge::<H>(&sibling, &node)
            } else {
                merge::<H>(&node, &sibling)
            };

            if !node.is_zero() {
                // node exists
                let branch_node = BranchNode {
                    fork_height: height,
                    sibling,
                    node,
                    key,
                };
                self.store.insert_branch(parent, branch_node)?;
            }
            node = parent;
        }
        self.root = node;
        Ok(&self.root)
    }

    /// Get value of a leaf
    /// return zero value if leaf not exists
    pub fn get(&self, key: &K) -> Result<V> {
        let mut node = self.root;
        // children must equal zero when parent equals zero
        while !node.is_zero() {
            let branch_node = match self.store.get_branch(&node)? {
                Some(branch_node) => branch_node,
                None => {
                    break;
                }
            };
            let is_right = key.get_bit(branch_node.fork_height);
            let (left, right) = branch_node.branch(branch_node.fork_height);
            node = if is_right { *right } else { *left };
            if branch_node.fork_height == 0 {
                break;
            }
        }

        // return zero is leaf_key is zero
        if node.is_zero() {
            return Ok(V::zero());
        }
        // get leaf node
        match self.store.get_leaf(&node)? {
            Some(leaf) if &leaf.key == key => Ok(leaf.value),
            _ => Ok(V::zero()),
        }
    }

    /// fetch merkle path of key into cache
    /// cache: (height, key) -> node
    fn fetch_merkle_path(
        &self,
        key: &K,
        cache: &mut BTreeMap<(usize, InternalKey<N>), H256>,
    ) -> Result<()> {
        let mut node = self.root;
        let mut height = self
            .store
            .get_branch(&node)?
            .map(|b| max(b.key.fork_height(key), b.fork_height))
            .unwrap_or(0);
        while !node.is_zero() {
            // the descendants are zeros, so we can break the loop
            if node.is_zero() {
                break;
            }
            match self.store.get_branch(&node)? {
                Some(branch_node) => {
                    if height > branch_node.fork_height {
                        let fork_height =
                            max(key.fork_height(&branch_node.key), branch_node.fork_height);

                        let is_right = key.get_bit(fork_height);
                        let mut sibling_key = key.parent_path(fork_height);
                        if !is_right {
                            // mark sibling's index, sibling on the right path.
                            sibling_key.set_bit(height);
                        };
                        if !node.is_zero() {
                            cache
                                .entry((fork_height as usize, sibling_key))
                                .or_insert(node);
                        }
                        break;
                    }
                    let (left, right) = branch_node.branch(height);
                    let is_right = key.get_bit(height);
                    let sibling = if is_right {
                        if &node == right {
                            break;
                        }
                        node = *right;
                        *left
                    } else {
                        if &node == left {
                            break;
                        }
                        node = *left;
                        *right
                    };
                    let mut sibling_key = key.parent_path(height);
                    if !is_right {
                        // mark sibling's index, sibling on the right path.
                        sibling_key.set_bit(height);
                    };
                    cache.insert((height as usize, sibling_key), sibling);
                    if let Some(branch_node) = self.store.get_branch(&node)? {
                        let fork_height =
                            max(key.fork_height(&branch_node.key), branch_node.fork_height);
                        height = fork_height;
                    }
                }
                None => break,
            };
        }
        Ok(())
    }

    /// Generate merkle proof
    pub fn merkle_proof(&self, mut keys: Vec<K>) -> Result<MerkleProof> {
        if keys.is_empty() {
            return Err(Error::EmptyKeys);
        }

        // sort keys
        keys.sort_unstable_by_key(|k| **k);

        // fetch all merkle path
        let mut cache: BTreeMap<(usize, _), H256> = Default::default();
        for k in &keys {
            self.fetch_merkle_path(k, &mut cache)?;
        }

        // (node, height)
        let mut proof: Vec<(H256, usize)> = Vec::with_capacity(EXPECTED_PATH_SIZE * keys.len());
        // key_index -> merkle path height
        let mut leaves_path: Vec<Vec<usize>> = Vec::with_capacity(keys.len());
        leaves_path.resize_with(keys.len(), Default::default);

        let keys_len = keys.len();
        // build merkle proofs from bottom to up
        // (key, height, key_index)
        let mut queue: VecDeque<(_, usize, usize)> = keys
            .into_iter()
            .enumerate()
            .map(|(i, k)| (*k, 0, i))
            .collect();

        while let Some((key, height, leaf_index)) = queue.pop_front() {
            if queue.is_empty() && cache.is_empty() || height == 8 * N {
                // tree only contains one leaf
                if leaves_path[leaf_index].is_empty() {
                    leaves_path[leaf_index].push((8 * N) - 1);
                }
                break;
            }
            // compute sibling key
            let mut sibling_key = key.parent_path(height);

            let is_right = key.get_bit(height);
            if is_right {
                // sibling on left
                sibling_key.clear_bit(height);
            } else {
                // sibling on right
                sibling_key.set_bit(height);
            }
            if Some((&sibling_key, &height))
                == queue
                    .front()
                    .map(|(sibling_key, height, _leaf_index)| (sibling_key, height))
            {
                // drop the sibling, mark sibling's merkle path
                let (_sibling_key, height, leaf_index) = queue.pop_front().unwrap();
                leaves_path[leaf_index].push(height);
            } else {
                match cache.remove(&(height, sibling_key)) {
                    Some(sibling) => {
                        debug_assert!(height < 8 * N);
                        // save first non-zero sibling's height for leaves
                        proof.push((sibling, height));
                    }
                    None => {
                        // skip zero siblings
                        if !is_right {
                            sibling_key.clear_bit(height);
                        }
                        let parent_key = sibling_key;
                        queue.push_back((parent_key, height + 1, leaf_index));
                        continue;
                    }
                }
            }
            // find new non-zero sibling, append to leaf's path
            leaves_path[leaf_index].push(height);
            if height < 8 * N {
                // get parent_key, which k.get_bit(height) is false
                let parent_key = if is_right { sibling_key } else { key };
                queue.push_back((parent_key, height + 1, leaf_index));
            }
        }
        debug_assert_eq!(leaves_path.len(), keys_len);
        Ok(MerkleProof::new(leaves_path, proof))
    }

    /// Generate ICS 23 commitment proof for the existing key
    pub fn membership_proof(&self, key: &K) -> Result<CommitmentProof> {
        let value = self.get(key)?;
        if value == V::zero() {
            return Err(Error::ExistenceProof);
        }
        let merkle_proof = self.merkle_proof(vec![*key])?;
        let existence_proof =
            proof_ics23::convert(merkle_proof, key, &value, H::hash_op())?;
        Ok(CommitmentProof {
            proof: Some(Proof::Exist(existence_proof)),
        })
    }

    /// Generate ICS 23 commitment proof for the non-existing key
    pub fn non_membership_proof(&self, key: &K) -> Result<CommitmentProof> {
        let value = self.get(key)?;
        if value != V::zero() {
            return Err(Error::NonExistenceProof);
        }

        // fetch all merkle path
        let mut cache: BTreeMap<(usize, _), H256> = Default::default();
        self.fetch_merkle_path(key, &mut cache)?;
        let mut left = None;
        let mut right = None;
        for (_, node) in cache.iter() {
            let branch = self
                .store
                .get_branch(node)?
                .expect("the forked branch should exist");
            let fork_height = key.fork_height(&branch.key);
            let is_right = key.get_bit(fork_height);
            if is_right && left.is_none() {
                // get the left which is the most right in the left subtree
                let mut n = *node;
                while let Some(branch) = self.store.get_branch(&n)? {
                    if branch.fork_height == 0 {
                        break;
                    }
                    let (left_node, right_node) = branch.branch(branch.fork_height);
                    n = if right_node.is_zero() {
                        *left_node
                    } else {
                        *right_node
                    };
                }
                let leaf = self.store.get_leaf(&n)?.expect("the leaf should exist");
                let merkle_proof = self.merkle_proof(vec![leaf.key])?;
                left = Some(proof_ics23::convert(
                    merkle_proof,
                    &leaf.key,
                    &leaf.value,
                    H::hash_op(),
                )?);
            } else if !is_right && right.is_none() {
                // get the right which is the most left in the right subtree
                let mut n = *node;
                while let Some(branch) = self.store.get_branch(&n)? {
                    if branch.fork_height == 0 {
                        break;
                    }
                    let (left_node, right_node) = branch.branch(branch.fork_height);
                    n = if left_node.is_zero() {
                        *right_node
                    } else {
                        *left_node
                    };
                }
                let leaf = self.store.get_leaf(&n)?.expect("the leaf should exist");
                let merkle_proof = self.merkle_proof(vec![leaf.key])?;
                right = Some(proof_ics23::convert(
                    merkle_proof,
                    &leaf.key,
                    &leaf.value,
                    H::hash_op(),
                )?);
            }
            if left.is_some() && right.is_some() {
                break;
            }
        }
        let proof = NonExistenceProof {
            key: key.to_vec(),
            left,
            right,
        };
        Ok(CommitmentProof {
            proof: Some(Proof::Nonexist(proof)),
        })
    }

    /// Recompute the root of the merkle tree from the store. Check if it agrees with the
    /// root in `self`.
    pub fn validate(&self) -> bool {
        // create an iterator over consecutive pairs of leaves
        let pairs = {
            let sorted_leaves = self.store.sorted_leaves();
            let mut other = self.store.sorted_leaves();
            _ = other.next();
            sorted_leaves.zip(other)
        };

        // handle case when tree is empty
        if self.store.size() == 0 {
            return self.root == H256::zero()
        }

        // construct a vector of nodes and distance to next node
        let mut leaves = Vec::with_capacity(self.store.size());
        for ((k1, v1), (k2, _)) in pairs {
            let height = k1.fork_height(&k2);
            let hash = hash_leaf::<H, K, V, N>(&k1, &v1);
            leaves.push((hash, height));
        }
        let (last_k, last_v) = self.store
            .sorted_leaves()
            .last()
            .map(|(k, v)| (k, v))
            .unwrap();
        let last = hash_leaf::<H, K, V, N>(&last_k, last_v);
        if leaves.is_empty() {
            return self.root == last;
        }
        leaves.push((last, usize::MAX));

        let mut left: usize = 0;
        let mut right: usize = 1;
        let mut merged = Default::default();

        // stack of previous `left` indexes that are yet to be merged
        let mut prev: Vec<usize> = Vec::with_capacity(leaves.len() / 2);

        // Iterate finding the first node `left` such that `left+1` (`right`) is
        // its closest neighbor and vice versa, merging them until a single node
        // remains.
        while right < leaves.len() {
            if leaves[left].1 < leaves[right].1 {
                loop {
                    // perform merge
                    merged = merge::<H>(&leaves[left].0, &leaves[right].0);
                    leaves[right].0 = merged;

                    // check previous `left` node next (if present)
                    match prev.last() {
                        Some(&idx) if leaves[idx].1 < leaves[right].1 => {
                            left = idx;
                            _ = prev.pop();
                            continue;
                        }
                        _ => {
                            break;
                        }
                    }
                }
            } else {
                prev.push(left);
            }
            left = right;
            right += 1;
        }
        // check that the recovered root matches the precomputed one
        merged == self.root
    }
}
